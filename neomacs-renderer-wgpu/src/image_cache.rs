//! Async image loading and caching for wgpu renderer
//!
//! Provides non-blocking image loading:
//! - Dimension queries for pending-image placeholders
//! - Background decoding in thread pool
//! - GPU texture upload when ready
//! - LRU cache with memory limits

use std::cell::Cell;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

#[cfg(target_os = "linux")]
use crate::external_buffer::DmaBufBuffer;

/// Maximum texture dimension (width or height)
const MAX_TEXTURE_SIZE: u32 = 4096;

/// Constrain dimensions to the renderer's limits while preserving aspect ratio.
pub(crate) fn constrain_dimensions(
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
) -> (u32, u32) {
    let mut width = width;
    let mut height = height;
    let width_limit = if max_width > 0 {
        max_width.min(MAX_TEXTURE_SIZE)
    } else {
        MAX_TEXTURE_SIZE
    };
    let height_limit = if max_height > 0 {
        max_height.min(MAX_TEXTURE_SIZE)
    } else {
        MAX_TEXTURE_SIZE
    };

    if width > width_limit {
        height = (f64::from(height) * f64::from(width_limit) / f64::from(width)) as u32;
        width = width_limit;
    }
    if height > height_limit {
        width = (f64::from(width) * f64::from(height_limit) / f64::from(height)) as u32;
        height = height_limit;
    }

    (width.max(1), height.max(1))
}

/// Maximum total cache memory in bytes (64MB)
const MAX_CACHE_MEMORY: usize = 64 * 1024 * 1024;

/// Get number of decoder threads (use all available CPU cores)
fn decoder_thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Image loading state
#[derive(Debug, Clone)]
pub enum ImageState {
    /// Queued for loading
    Pending,
    /// Currently being decoded
    Decoding,
    /// Ready with texture
    Ready,
    /// Failed to load
    Failed(String),
}

/// Cached image with GPU texture
pub struct CachedImage {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
    pub metadata: Option<ImageMetadata>,
    /// Memory size in bytes
    pub memory_size: usize,
    /// Monotonic access stamp for LRU eviction; refreshed by `get` (a `Cell`
    /// so draw-path lookups stay `&self`).
    last_access: Cell<u64>,
}

/// Decoded image data waiting for GPU upload
struct DecodedImage {
    load: ImageLoadToken,
    width: u32,
    height: u32,
    data: Vec<u8>, // RGBA
    metadata: ImageMetadata,
}

enum WorkerDecodeOutcome {
    Ready(DecodedImage),
    Failed(ImageLoadToken),
}

impl WorkerDecodeOutcome {
    fn load(&self) -> ImageLoadToken {
        match self {
            Self::Ready(decoded) => decoded.load,
            Self::Failed(load) => *load,
        }
    }
}

/// A terminal async image decode result accepted for the current load generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageDecodeOutcome {
    Ready { id: u32, metadata: ImageMetadata },
    Failed { id: u32, error: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ImageLoadToken {
    id: u32,
    generation: u64,
}

#[derive(Default)]
struct ImageLoadLifecycle {
    next_generation: u64,
    active: HashMap<u32, u64>,
}

impl ImageLoadLifecycle {
    fn begin(&mut self, id: u32) -> ImageLoadToken {
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .expect("image load generation exhausted");
        let generation = self.next_generation;
        self.active.insert(id, generation);
        ImageLoadToken { id, generation }
    }

    fn accept(&mut self, load: ImageLoadToken) -> bool {
        if self.active.get(&load.id) != Some(&load.generation) {
            return false;
        }
        self.active.remove(&load.id);
        true
    }

    fn take_current(&mut self, outcome: WorkerDecodeOutcome) -> Option<WorkerDecodeOutcome> {
        self.accept(outcome.load()).then_some(outcome)
    }

    fn free(&mut self, id: u32) {
        self.active.remove(&id);
    }

    fn clear(&mut self) {
        self.active.clear();
    }
}

/// Image dimensions (from header)
#[derive(Debug, Clone, Copy)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

/// Facts derived from the final decoded RGBA pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    /// GNU's four-corner background guess, encoded as 0x00RRGGBB.
    pub background: u32,
    /// Whether GNU's four-corner mask heuristic classifies the background as transparent.
    pub background_transparent: bool,
}

/// Async image cache
pub struct ImageCache {
    /// Next image ID
    next_id: AtomicU32,
    /// Cached textures: id -> CachedImage
    textures: HashMap<u32, CachedImage>,
    /// Image states: id -> state
    states: HashMap<u32, ImageState>,
    /// Identifies the one decode request currently allowed to publish for each ID.
    loads: ImageLoadLifecycle,
    /// Pending dimensions (before texture is ready)
    pending_dimensions: HashMap<u32, ImageDimensions>,
    /// Channel to receive decoded images
    decoded_rx: mpsc::Receiver<WorkerDecodeOutcome>,
    /// Channel to send decode requests
    decode_tx: mpsc::Sender<DecodeRequest>,
    /// Bind group layout for image textures
    bind_group_layout: wgpu::BindGroupLayout,
    /// Sampler for image textures
    sampler: wgpu::Sampler,
    /// Total cached memory
    total_memory: usize,
    /// Monotonic clock stamping `CachedImage::last_access` (LRU order).
    access_clock: Cell<u64>,
}

/// Pick the least-recently-used entry: the id with the smallest access stamp
/// (ties broken by smaller id for determinism).
fn lru_victim(entries: impl Iterator<Item = (u32, u64)>) -> Option<u32> {
    entries
        .min_by_key(|&(id, stamp)| (stamp, id))
        .map(|(id, _)| id)
}

/// Request to decode an image
struct DecodeRequest {
    load: ImageLoadToken,
    source: ImageSource,
    max_width: u32,
    max_height: u32,
    /// Foreground color as 0xAARRGGBB for monochrome formats (XBM). 0 = default.
    fg_color: u32,
    /// Background color as 0xAARRGGBB for monochrome formats (XBM). 0 = default.
    bg_color: u32,
}

/// Image source
enum ImageSource {
    File(String),
    Data(Vec<u8>),
    /// Raw ARGB32 pixel data (A,R,G,B byte order, 4 bytes per pixel)
    RawArgb32 {
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
    },
    /// Raw RGB24 pixel data (R,G,B byte order, 3 bytes per pixel)
    RawRgb24 {
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
    },
}

impl ImageCache {
    /// Create a new image cache
    pub fn new(device: &wgpu::Device) -> Self {
        // Create bind group layout for image textures
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Image Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Image Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // Create channels for async decoding
        let (decode_tx, decode_rx) = mpsc::channel::<DecodeRequest>();
        let (decoded_tx, decoded_rx) = mpsc::channel::<WorkerDecodeOutcome>();

        // Wrap receiver in Arc<Mutex> for sharing across threads
        let decode_rx = Arc::new(Mutex::new(decode_rx));

        // Spawn decoder thread pool (one per CPU core)
        let num_threads = decoder_thread_count();
        tracing::info!("Starting {} image decoder threads", num_threads);
        for i in 0..num_threads {
            let rx = Arc::clone(&decode_rx);
            let tx = decoded_tx.clone();
            thread::spawn(move || {
                Self::decoder_thread_pooled(i, rx, tx);
            });
        }

        Self {
            next_id: AtomicU32::new(1),
            textures: HashMap::new(),
            states: HashMap::new(),
            loads: ImageLoadLifecycle::default(),
            pending_dimensions: HashMap::new(),
            decoded_rx,
            decode_tx,
            bind_group_layout,
            sampler,
            total_memory: 0,
            access_clock: Cell::new(0),
        }
    }

    /// Advance the access clock and return a fresh stamp.
    fn next_access_stamp(&self) -> u64 {
        let stamp = self.access_clock.get() + 1;
        self.access_clock.set(stamp);
        stamp
    }

    fn begin_load(&mut self, id: u32) -> ImageLoadToken {
        if let Some(cached) = self.textures.remove(&id) {
            self.total_memory -= cached.memory_size;
        }
        self.pending_dimensions.remove(&id);
        self.loads.begin(id)
    }

    /// Background decoder thread (pooled version)
    fn decoder_thread_pooled(
        thread_id: usize,
        rx: Arc<Mutex<mpsc::Receiver<DecodeRequest>>>,
        tx: mpsc::Sender<WorkerDecodeOutcome>,
    ) {
        tracing::debug!("Decoder thread {} started", thread_id);
        loop {
            // Lock, receive, unlock immediately to allow other threads to grab work
            let request = {
                let guard = rx.lock().unwrap_or_else(|e| e.into_inner());
                guard.recv()
            };

            match request {
                Ok(request) => {
                    tracing::debug!("Thread {} decoding image {}", thread_id, request.load.id);
                    let fg_bg = (request.fg_color, request.bg_color);
                    let result = match request.source {
                        ImageSource::File(path) => {
                            Self::decode_file(&path, request.max_width, request.max_height, fg_bg)
                        }
                        ImageSource::Data(data) => {
                            Self::decode_data(&data, request.max_width, request.max_height, fg_bg)
                        }
                        ImageSource::RawArgb32 {
                            data,
                            width,
                            height,
                            stride,
                        } => Self::convert_argb32_to_rgba(
                            &data,
                            width,
                            height,
                            stride,
                            request.max_width,
                            request.max_height,
                        ),
                        ImageSource::RawRgb24 {
                            data,
                            width,
                            height,
                            stride,
                        } => Self::convert_rgb24_to_rgba(
                            &data,
                            width,
                            height,
                            stride,
                            request.max_width,
                            request.max_height,
                        ),
                    };

                    if let Some((width, height, data)) = result {
                        let _ = tx.send(WorkerDecodeOutcome::Ready(Self::decoded_image(
                            request.load,
                            width,
                            height,
                            data,
                        )));
                    } else {
                        let _ = tx.send(WorkerDecodeOutcome::Failed(request.load));
                    }
                }
                Err(_) => {
                    // Channel closed, exit thread
                    tracing::debug!("Decoder thread {} exiting", thread_id);
                    break;
                }
            }
        }
    }

    /// Convert 0xAARRGGBB color to [R,G,B,A] array.
    /// If color is 0 (default), return the provided fallback.
    fn argb_to_rgba(color: u32, fallback: [u8; 4]) -> [u8; 4] {
        if color == 0 {
            return fallback;
        }
        let a = ((color >> 24) & 0xFF) as u8;
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;
        [r, g, b, a]
    }

    /// Decode image file with size constraints
    fn decode_file(
        path: &str,
        max_width: u32,
        max_height: u32,
        fg_bg: (u32, u32),
    ) -> Option<(u32, u32, Vec<u8>)> {
        if let Ok(img) = image::open(path) {
            return Self::process_image(img, max_width, max_height);
        }
        // Fallback: try XPM
        if let Some(result) = crate::xpm::decode_xpm_file(Path::new(path), max_width, max_height) {
            return Some(result);
        }
        // Fallback: try XBM
        let fg = Self::argb_to_rgba(fg_bg.0, [255, 255, 255, 255]);
        let bg = Self::argb_to_rgba(fg_bg.1, [0, 0, 0, 255]);
        if let Some(result) =
            crate::xbm::decode_xbm_file(Path::new(path), fg, bg, max_width, max_height)
        {
            return Some(result);
        }
        // Fallback: try SVG via the shared librsvg backend.
        let data = std::fs::read(path).ok()?;
        Self::decode_svg_data(&data, max_width, max_height)
    }

    /// Decode image data with size constraints
    fn decode_data(
        data: &[u8],
        max_width: u32,
        max_height: u32,
        fg_bg: (u32, u32),
    ) -> Option<(u32, u32, Vec<u8>)> {
        if let Ok(img) = image::load_from_memory(data) {
            return Self::process_image(img, max_width, max_height);
        }
        // Fallback: try XPM
        if let Some(result) = crate::xpm::decode_xpm_data(data, max_width, max_height) {
            return Some(result);
        }
        // Fallback: try XBM
        let fg = Self::argb_to_rgba(fg_bg.0, [255, 255, 255, 255]);
        let bg = Self::argb_to_rgba(fg_bg.1, [0, 0, 0, 255]);
        if let Some(result) = crate::xbm::decode_xbm_data(data, fg, bg, max_width, max_height) {
            return Some(result);
        }
        // Fallback: try SVG via the shared librsvg backend.
        Self::decode_svg_data(data, max_width, max_height)
    }

    #[cfg(test)]
    fn decode_data_with_metadata(
        data: &[u8],
        max_width: u32,
        max_height: u32,
        fg_bg: (u32, u32),
    ) -> Option<DecodedImage> {
        let (width, height, data) = Self::decode_data(data, max_width, max_height, fg_bg)?;
        Some(Self::decoded_image(
            ImageLoadToken {
                id: 0,
                generation: 0,
            },
            width,
            height,
            data,
        ))
    }

    fn decoded_image(load: ImageLoadToken, width: u32, height: u32, data: Vec<u8>) -> DecodedImage {
        let metadata = Self::metadata_from_rgba(width, height, &data);
        DecodedImage {
            load,
            width,
            height,
            data,
            metadata,
        }
    }

    fn metadata_from_rgba(width: u32, height: u32, rgba: &[u8]) -> ImageMetadata {
        let pixel = |x: u32, y: u32| {
            let offset = ((y * width + x) * 4) as usize;
            [
                rgba[offset],
                rgba[offset + 1],
                rgba[offset + 2],
                rgba[offset + 3],
            ]
        };
        let corners = [
            pixel(0, 0),
            pixel(width - 1, 0),
            pixel(width - 1, height - 1),
            pixel(0, height - 1),
        ];
        let most_frequent = |values: [[u8; 4]; 4], key: fn([u8; 4]) -> u32| {
            let mut best = values[0];
            let mut best_count = 0;
            for candidate in values {
                let count = values
                    .iter()
                    .filter(|value| key(**value) == key(candidate))
                    .count();
                if count > best_count {
                    best = candidate;
                    best_count = count;
                }
            }
            best
        };
        let background = most_frequent(corners, |pixel| {
            (u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2])
        });
        let mask = most_frequent(corners, |pixel| u32::from(pixel[3] == 0));
        ImageMetadata {
            width,
            height,
            background: (u32::from(background[0]) << 16)
                | (u32::from(background[1]) << 8)
                | u32::from(background[2]),
            background_transparent: mask[3] == 0,
        }
    }

    /// Decode SVG data through the platform SVG backend, returning RGBA pixels.
    fn decode_svg_data(
        data: &[u8],
        max_width: u32,
        max_height: u32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let decoded = crate::svg::decode(data, max_width, max_height)?;
        Some((decoded.width, decoded.height, decoded.rgba))
    }

    /// Process decoded image: resize if needed, convert to RGBA
    fn process_image(
        img: image::DynamicImage,
        max_width: u32,
        max_height: u32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let (mut width, mut height) = (img.width(), img.height());

        // Apply max constraints
        let mw = if max_width > 0 {
            max_width
        } else {
            MAX_TEXTURE_SIZE
        };
        let mh = if max_height > 0 {
            max_height
        } else {
            MAX_TEXTURE_SIZE
        };

        // Scale down if needed (preserve aspect ratio)
        if width > mw || height > mh {
            let ratio = (width as f64 / height as f64).min(mw as f64 / mh as f64);
            if width > mw {
                width = mw;
                height = (mw as f64 / ratio) as u32;
            }
            if height > mh {
                height = mh;
                width = (mh as f64 * ratio) as u32;
            }
        }

        // Resize if dimensions changed
        let img = if width != img.width() || height != img.height() {
            img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        // Convert to RGBA
        let rgba = img.to_rgba8();
        Some((width, height, rgba.into_raw()))
    }

    /// Convert ARGB32 raw pixel data to RGBA
    /// Input format: A,R,G,B byte order (4 bytes per pixel)
    /// Output format: R,G,B,A byte order (4 bytes per pixel)
    fn convert_argb32_to_rgba(
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
        max_width: u32,
        max_height: u32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let bytes_per_pixel = 4u32;
        let expected_min_size = (height.saturating_sub(1)) * stride + width * bytes_per_pixel;
        if data.len() < expected_min_size as usize {
            tracing::warn!(
                "ARGB32 data too small: got {} bytes, expected at least {} for {}x{} with stride {}",
                data.len(),
                expected_min_size,
                width,
                height,
                stride
            );
            return None;
        }

        // Convert ARGB32 to RGBA
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            let row_start = (y * stride) as usize;
            for x in 0..width {
                let pixel_start = row_start + (x * bytes_per_pixel) as usize;
                let a = data[pixel_start];
                let r = data[pixel_start + 1];
                let g = data[pixel_start + 2];
                let b = data[pixel_start + 3];
                let idx = ((y * width + x) * 4) as usize;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = a;
            }
        }

        // Apply size constraints if needed
        let (cw, ch) = constrain_dimensions(width, height, max_width, max_height);
        if cw != width || ch != height {
            // Need to resize - use image crate
            let img = image::RgbaImage::from_raw(width, height, rgba)?;
            let resized =
                image::imageops::resize(&img, cw, ch, image::imageops::FilterType::Lanczos3);
            Some((cw, ch, resized.into_raw()))
        } else {
            Some((width, height, rgba))
        }
    }

    /// Convert RGB24 raw pixel data to RGBA
    /// Input format: R,G,B byte order (3 bytes per pixel)
    /// Output format: R,G,B,A byte order (4 bytes per pixel, alpha=255)
    fn convert_rgb24_to_rgba(
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
        max_width: u32,
        max_height: u32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let bytes_per_pixel = 3u32;
        let expected_min_size = (height.saturating_sub(1)) * stride + width * bytes_per_pixel;
        if data.len() < expected_min_size as usize {
            tracing::warn!(
                "RGB24 data too small: got {} bytes, expected at least {} for {}x{} with stride {}",
                data.len(),
                expected_min_size,
                width,
                height,
                stride
            );
            return None;
        }

        // Convert RGB24 to RGBA (add alpha=255)
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            let row_start = (y * stride) as usize;
            for x in 0..width {
                let pixel_start = row_start + (x * bytes_per_pixel) as usize;
                let r = data[pixel_start];
                let g = data[pixel_start + 1];
                let b = data[pixel_start + 2];
                let idx = ((y * width + x) * 4) as usize;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }

        // Apply size constraints if needed
        let (cw, ch) = constrain_dimensions(width, height, max_width, max_height);
        if cw != width || ch != height {
            // Need to resize - use image crate
            let img = image::RgbaImage::from_raw(width, height, rgba)?;
            let resized =
                image::imageops::resize(&img, cw, ch, image::imageops::FilterType::Lanczos3);
            Some((cw, ch, resized.into_raw()))
        } else {
            Some((width, height, rgba))
        }
    }

    /// Get bind group layout
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Get sampler (for sharing with video cache)
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Query image file dimensions.
    ///
    /// Raster formats read only their header; SVG requires document parsing.
    pub fn query_file_dimensions(path: &str) -> Option<ImageDimensions> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);

        // Use image crate's dimension reader (reads header only)
        if let Ok(dims) = image::ImageReader::new(reader)
            .with_guessed_format()
            .ok()?
            .into_dimensions()
        {
            return Some(ImageDimensions {
                width: dims.0,
                height: dims.1,
            });
        }

        // Fallback: try SVG.
        let data = std::fs::read(path).ok()?;
        Self::query_svg_dimensions(&data)
    }

    /// Query image data dimensions.
    ///
    /// Raster formats read only their header; SVG requires document parsing.
    pub fn query_data_dimensions(data: &[u8]) -> Option<ImageDimensions> {
        let cursor = std::io::Cursor::new(data);
        if let Ok(dims) = image::ImageReader::new(BufReader::new(cursor))
            .with_guessed_format()
            .ok()?
            .into_dimensions()
        {
            return Some(ImageDimensions {
                width: dims.0,
                height: dims.1,
            });
        }

        // Fallback: try XPM header
        if let Some((w, h)) = crate::xpm::query_xpm_dimensions(data) {
            return Some(ImageDimensions {
                width: w,
                height: h,
            });
        }

        // Fallback: try XBM header
        if let Some((w, h)) = crate::xbm::query_xbm_dimensions(data) {
            return Some(ImageDimensions {
                width: w,
                height: h,
            });
        }

        // Fallback: try SVG.
        Self::query_svg_dimensions(data)
    }

    /// Query SVG dimensions without full rendering
    fn query_svg_dimensions(data: &[u8]) -> Option<ImageDimensions> {
        let (width, height) = crate::svg::query_dimensions(data)?;
        Some(ImageDimensions { width, height })
    }

    /// Load image from file (async)
    /// Returns image ID immediately, texture loads in background
    pub fn load_file(
        &mut self,
        path: &str,
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.load_file_with_id(id, path, max_width, max_height, fg_color, bg_color);
        id
    }

    /// Load image from data with a pre-allocated ID (for threaded mode)
    pub fn load_data_with_id(
        &mut self,
        id: u32,
        data: &[u8],
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) {
        let load = self.begin_load(id);
        // Query dimensions for the pending-image placeholder.
        if let Some(dims) = Self::query_data_dimensions(data) {
            let (w, h) = constrain_dimensions(dims.width, dims.height, max_width, max_height);
            self.pending_dimensions.insert(
                id,
                ImageDimensions {
                    width: w,
                    height: h,
                },
            );
        }

        // Queue for async decode
        self.states.insert(id, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::Data(data.to_vec()),
            max_width,
            max_height,
            fg_color,
            bg_color,
        });
    }

    /// Load image from file with a pre-allocated ID (for threaded mode)
    /// This allows the calling code to allocate the ID before sending a command.
    pub fn load_file_with_id(
        &mut self,
        id: u32,
        path: &str,
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) {
        let load = self.begin_load(id);
        // Query dimensions for the pending-image placeholder.
        if let Some(dims) = Self::query_file_dimensions(path) {
            // Apply max constraints to dimensions
            let (w, h) = constrain_dimensions(dims.width, dims.height, max_width, max_height);
            self.pending_dimensions.insert(
                id,
                ImageDimensions {
                    width: w,
                    height: h,
                },
            );
        }

        // Queue for async decode
        self.states.insert(id, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::File(path.to_string()),
            max_width,
            max_height,
            fg_color,
            bg_color,
        });
    }

    /// Allocate the next available image ID without loading anything.
    /// Used by threaded mode to pre-allocate IDs before sending commands.
    pub fn allocate_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Load image from data (async)
    pub fn load_data(
        &mut self,
        data: &[u8],
        max_width: u32,
        max_height: u32,
        fg_color: u32,
        bg_color: u32,
    ) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let load = self.begin_load(id);

        // Query dimensions for the pending-image placeholder.
        if let Some(dims) = Self::query_data_dimensions(data) {
            let (w, h) = constrain_dimensions(dims.width, dims.height, max_width, max_height);
            self.pending_dimensions.insert(
                id,
                ImageDimensions {
                    width: w,
                    height: h,
                },
            );
        }

        // Queue for async decode
        self.states.insert(id, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::Data(data.to_vec()),
            max_width,
            max_height,
            fg_color,
            bg_color,
        });

        id
    }

    /// Load image from raw ARGB32 pixel data (async)
    /// Format: A,R,G,B byte order, 4 bytes per pixel
    /// Stride is the number of bytes per row (may include padding)
    pub fn load_raw_argb32(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
        max_width: u32,
        max_height: u32,
    ) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let load = self.begin_load(id);

        // Store pending dimensions immediately (we know the exact size)
        let (w, h) = constrain_dimensions(width, height, max_width, max_height);
        self.pending_dimensions.insert(
            id,
            ImageDimensions {
                width: w,
                height: h,
            },
        );

        // Queue for async conversion
        self.states.insert(id, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::RawArgb32 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            max_width,
            max_height,
            fg_color: 0,
            bg_color: 0,
        });

        id
    }

    /// Load image from raw RGB24 pixel data (async)
    /// Format: R,G,B byte order, 3 bytes per pixel
    /// Stride is the number of bytes per row (may include padding)
    pub fn load_raw_rgb24(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
        max_width: u32,
        max_height: u32,
    ) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let load = self.begin_load(id);

        // Store pending dimensions immediately (we know the exact size)
        let (w, h) = constrain_dimensions(width, height, max_width, max_height);
        self.pending_dimensions.insert(
            id,
            ImageDimensions {
                width: w,
                height: h,
            },
        );

        // Queue for async conversion
        self.states.insert(id, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::RawRgb24 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            max_width,
            max_height,
            fg_color: 0,
            bg_color: 0,
        });

        id
    }

    /// Load image from raw ARGB32 pixel data with a pre-allocated ID (for threaded mode)
    pub fn load_raw_argb32_with_id(
        &mut self,
        id: u32,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
    ) {
        let load = self.begin_load(id);
        self.pending_dimensions
            .insert(id, ImageDimensions { width, height });
        self.states.insert(id, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::RawArgb32 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            max_width: 0,
            max_height: 0,
            fg_color: 0,
            bg_color: 0,
        });
    }

    /// Load image from raw RGB24 pixel data with a pre-allocated ID (for threaded mode)
    pub fn load_raw_rgb24_with_id(
        &mut self,
        id: u32,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
    ) {
        let load = self.begin_load(id);
        self.pending_dimensions
            .insert(id, ImageDimensions { width, height });
        self.states.insert(id, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::RawRgb24 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            max_width: 0,
            max_height: 0,
            fg_color: 0,
            bg_color: 0,
        });
    }

    /// Import image from DMA-BUF (zero-copy if supported)
    #[cfg(target_os = "linux")]
    pub fn import_dmabuf(
        &mut self,
        dmabuf: DmaBufBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> u32 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (width, height) = dmabuf.dimensions();

        // Try zero-copy import
        if let Some(texture) = dmabuf.to_wgpu_texture(device, queue) {
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("DMA-BUF Image Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            let memory_size = (width * height * 4) as usize;
            self.total_memory += memory_size;

            self.textures.insert(
                id,
                CachedImage {
                    texture,
                    view,
                    bind_group,
                    width,
                    height,
                    metadata: None,
                    memory_size,
                    last_access: Cell::new(self.next_access_stamp()),
                },
            );
            self.states.insert(id, ImageState::Ready);

            tracing::info!(
                "Imported DMA-BUF image {} ({}x{}) zero-copy",
                id,
                width,
                height
            );
        } else {
            self.states
                .insert(id, ImageState::Failed("DMA-BUF import failed".into()));
            tracing::warn!("DMA-BUF import failed for image {}", id);
        }

        id
    }

    /// Process pending decoded images (call each frame)
    pub fn process_pending(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<ImageDecodeOutcome> {
        let mut completed = Vec::new();
        // Drain decoded images from channel
        while let Ok(outcome) = self.decoded_rx.try_recv() {
            let Some(outcome) = self.loads.take_current(outcome) else {
                continue;
            };
            match outcome {
                WorkerDecodeOutcome::Ready(decoded) => {
                    completed.push(ImageDecodeOutcome::Ready {
                        id: decoded.load.id,
                        metadata: decoded.metadata,
                    });
                    self.upload_texture(device, queue, decoded);
                }
                WorkerDecodeOutcome::Failed(load) => {
                    let error = "image decode failed".to_owned();
                    self.states
                        .insert(load.id, ImageState::Failed(error.clone()));
                    self.pending_dimensions.remove(&load.id);
                    completed.push(ImageDecodeOutcome::Failed { id: load.id, error });
                }
            }
        }

        // Evict if over memory limit
        self.evict_if_needed();
        completed
    }

    /// Upload decoded image to GPU texture
    fn upload_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        decoded: DecodedImage,
    ) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image Texture"),
            size: wgpu::Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &decoded.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(decoded.width * 4),
                rows_per_image: Some(decoded.height),
            },
            wgpu::Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Image Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let memory_size = (decoded.width * decoded.height * 4) as usize;
        self.total_memory += memory_size;

        self.textures.insert(
            decoded.load.id,
            CachedImage {
                texture,
                view,
                bind_group,
                width: decoded.width,
                height: decoded.height,
                metadata: Some(decoded.metadata),
                memory_size,
                last_access: Cell::new(self.next_access_stamp()),
            },
        );

        self.states.insert(decoded.load.id, ImageState::Ready);
        self.pending_dimensions.remove(&decoded.load.id);

        tracing::debug!(
            "Uploaded image {} ({}x{}, {}KB)",
            decoded.load.id,
            decoded.width,
            decoded.height,
            memory_size / 1024
        );
    }

    /// Evict least-recently-used textures until under the memory limit.
    fn evict_if_needed(&mut self) {
        while self.total_memory > MAX_CACHE_MEMORY && !self.textures.is_empty() {
            let victim = lru_victim(
                self.textures
                    .iter()
                    .map(|(&id, cached)| (id, cached.last_access.get())),
            );
            if let Some(id) = victim
                && let Some(cached) = self.textures.remove(&id)
            {
                self.total_memory -= cached.memory_size;
                self.states.remove(&id);
                tracing::debug!(
                    "Evicted image {} to free {}KB",
                    id,
                    cached.memory_size / 1024
                );
            }
        }
    }

    /// Get cached image if ready. Refreshes the entry's LRU access stamp.
    pub fn get(&self, id: u32) -> Option<&CachedImage> {
        let cached = self.textures.get(&id)?;
        cached.last_access.set(self.next_access_stamp());
        Some(cached)
    }

    /// Get image dimensions (pending or loaded)
    pub fn get_dimensions(&self, id: u32) -> Option<ImageDimensions> {
        // Check loaded textures first
        if let Some(cached) = self.textures.get(&id) {
            return Some(ImageDimensions {
                width: cached.width,
                height: cached.height,
            });
        }
        // Check pending dimensions
        self.pending_dimensions.get(&id).copied()
    }

    /// Get image state
    pub fn get_state(&self, id: u32) -> Option<&ImageState> {
        self.states.get(&id)
    }

    /// Check if image is ready
    pub fn is_ready(&self, id: u32) -> bool {
        matches!(self.states.get(&id), Some(ImageState::Ready))
    }

    /// Whether async decode work still needs the render thread to poll its result channel.
    pub fn has_pending(&self) -> bool {
        self.states
            .values()
            .any(|state| matches!(state, ImageState::Pending | ImageState::Decoding))
    }

    /// Free an image from cache
    pub fn free(&mut self, id: u32) {
        self.loads.free(id);
        if let Some(cached) = self.textures.remove(&id) {
            self.total_memory -= cached.memory_size;
        }
        self.states.remove(&id);
        self.pending_dimensions.remove(&id);
    }

    /// Clear entire cache
    pub fn clear(&mut self) {
        self.loads.clear();
        self.textures.clear();
        self.states.clear();
        self.pending_dimensions.clear();
        self.total_memory = 0;
    }
}

#[cfg(test)]
#[path = "image_cache_test.rs"]
mod tests;
