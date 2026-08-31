//! Async image loading and caching for wgpu renderer
//!
//! Provides non-blocking image loading:
//! - Dimension queries for pending-image placeholders
//! - Background decoding in thread pool
//! - GPU texture upload when ready
//! - LRU cache with memory limits

use neomacs_display_protocol::{
    ImageColorContext, ImageId, ImageLoadAttempt, ImageLoadToken, ImageRealization, ImageRotation,
    ImageSizeSpec,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::num::NonZeroUsize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

#[cfg(target_os = "linux")]
use crate::external_buffer::DmaBufBuffer;

/// Maximum texture dimension (width or height)
const MAX_TEXTURE_SIZE: u32 = 4096;

/// Clamp to the renderer's texture limit, preserving aspect ratio.
///
/// This is a GPU constraint only — GNU's `:max-width`/`:max-height` are applied
/// by `ImageSizeSpec::desired`, which knows the native size and so can keep the
/// aspect ratio against the right numbers.
pub(crate) fn constrain_dimensions(width: u32, height: u32) -> (u32, u32) {
    let mut width = width;
    let mut height = height;
    let width_limit = MAX_TEXTURE_SIZE;
    let height_limit = MAX_TEXTURE_SIZE;

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

const MAX_IMAGE_DECODER_THREADS: usize = 4;

/// A deliberately small, non-empty pool of persistent image decoders.
///
/// Image requests are normally sparse and already queue through one shared
/// receiver.  Scaling this pool to every host CPU made each GUI reserve dozens
/// of idle thread stacks before it had seen an image.  GNU image decoding is
/// synchronous (`image.c` even declines WebP's multithreaded option), so four
/// asynchronous workers retain useful parallelism without making GUI startup
/// resources proportional to machine size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ImageDecoderPoolSize(NonZeroUsize);

impl ImageDecoderPoolSize {
    fn detected() -> Self {
        Self::from_available_parallelism(std::thread::available_parallelism().ok())
    }

    fn from_available_parallelism(available: Option<NonZeroUsize>) -> Self {
        let available = available.map_or(MAX_IMAGE_DECODER_THREADS, NonZeroUsize::get);
        Self(
            NonZeroUsize::new(available.min(MAX_IMAGE_DECODER_THREADS))
                .expect("the image decoder pool cap is nonzero"),
        )
    }

    const fn get(self) -> usize {
        self.0.get()
    }
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
    /// Uploaded texture dimensions in physical device pixels.
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
    raster_width: u32,
    raster_height: u32,
    data: Vec<u8>, // RGBA
    metadata: ImageMetadata,
}

/// Decoded pixels keep layout, GNU image-pixel, and texture extents separate.
/// Layout feeds redisplay; pixel_* is Fimage_size PIXELS space; raster is GPU.
struct DecodedPixels {
    layout_width: u32,
    layout_height: u32,
    /// GNU `img->width` / `img->height` after `compute_image_size`.
    pixel_width: u32,
    pixel_height: u32,
    raster_width: u32,
    raster_height: u32,
    rgba: Vec<u8>,
}

/// Dual extents from one native size: layout uses `layout_scale`, image-pixels
/// use [`ImageRealization::image_pixel_scale`] so `:scale default` on HiDPI
/// recovers the true GNU size without inverting a non-invertible ceil path.
fn dual_extents(
    size: ImageSizeSpec,
    native_width: u32,
    native_height: u32,
    realization: ImageRealization,
) -> (u32, u32, u32, u32) {
    let layout_scale = f64::from(realization.layout_scale());
    let (layout_width, layout_height) = size.desired(native_width, native_height, layout_scale);
    let image_pixel_scale = realization.image_pixel_scale();
    let (pixel_width, pixel_height) = if (image_pixel_scale - layout_scale).abs() < 1e-9 {
        (layout_width, layout_height)
    } else {
        size.desired(native_width, native_height, image_pixel_scale)
    };
    (layout_width, layout_height, pixel_width, pixel_height)
}

impl DecodedPixels {
    fn raster(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            layout_width: width,
            layout_height: height,
            pixel_width: width,
            pixel_height: height,
            raster_width: width,
            raster_height: height,
            rgba,
        }
    }

    fn from_raster_tuple((width, height, rgba): (u32, u32, Vec<u8>)) -> Self {
        Self::raster(width, height, rgba)
    }

    /// Resolve the spec's requested size against the decoded native size, then
    /// realize to texture pixels.
    ///
    /// This is where GNU's `compute_image_size` lands: the size cannot be known
    /// before decoding, so `:width`/`:height`/`:max-*` are applied here rather
    /// than as a bounding box handed to the decoder. With `AxisSize::Native` on
    /// both axes this reduces to `layout_dimension`, the previous behavior.
    fn realize_bitmap(
        self,
        size: ImageSizeSpec,
        rotation: ImageRotation,
        realization: ImageRealization,
    ) -> Option<Self> {
        let (layout_width, layout_height, pixel_width, pixel_height) =
            dual_extents(size, self.layout_width, self.layout_height, realization);
        let (raster_width, raster_height) = constrain_dimensions(
            realization.raster_dimension(layout_width),
            realization.raster_dimension(layout_height),
        );
        let rgba = if raster_width == self.raster_width && raster_height == self.raster_height {
            self.rgba
        } else {
            let source =
                image::RgbaImage::from_raw(self.raster_width, self.raster_height, self.rgba)?;
            image::imageops::resize(
                &source,
                raster_width,
                raster_height,
                image::imageops::FilterType::Lanczos3,
            )
            .into_raw()
        };
        // GNU rotates AFTER sizing, so `:width` sizes the upright image and the
        // turn then exchanges the axes (src/image.c:3169-3201). Quarter turns
        // are lossless, which is exactly why GNU only offers multiples of 90.
        let (rgba, raster_width, raster_height) = match rotation {
            ImageRotation::None => (rgba, raster_width, raster_height),
            turn => {
                let source = image::RgbaImage::from_raw(raster_width, raster_height, rgba)?;
                let turned = match turn {
                    ImageRotation::Quarter => image::imageops::rotate90(&source),
                    ImageRotation::Half => image::imageops::rotate180(&source),
                    ImageRotation::ThreeQuarter => image::imageops::rotate270(&source),
                    ImageRotation::None => unreachable!("handled above"),
                };
                let (width, height) = (turned.width(), turned.height());
                (turned.into_raw(), width, height)
            }
        };
        let (layout_width, layout_height) = rotation.orient(layout_width, layout_height);
        let (pixel_width, pixel_height) = rotation.orient(pixel_width, pixel_height);

        Some(Self {
            layout_width,
            layout_height,
            pixel_width,
            pixel_height,
            raster_width,
            raster_height,
            rgba,
        })
    }
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

/// A renderer image-cache lifecycle event. Callers must handle eviction as
/// well as terminal decode results so external catalogs cannot retain stale
/// residency state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageCacheEvent {
    Ready {
        load: ImageLoadToken,
        metadata: ImageMetadata,
    },
    Failed {
        load: ImageLoadToken,
        error: String,
    },
    Evicted {
        image: ImageId,
    },
}

#[derive(Default)]
struct ImageLoadLifecycle {
    next_attempt: u64,
    active: HashMap<ImageId, ImageLoadAttempt>,
}

impl ImageLoadLifecycle {
    fn generated_token(&mut self, image: ImageId) -> ImageLoadToken {
        self.next_attempt = self
            .next_attempt
            .checked_add(1)
            .expect("image load attempt exhausted");
        let attempt =
            ImageLoadAttempt::new(self.next_attempt).expect("checked nonzero image load attempt");
        ImageLoadToken::new(image, attempt)
    }

    #[cfg(test)]
    fn begin_generated(&mut self, image: ImageId) -> ImageLoadToken {
        let load = self.generated_token(image);
        self.begin(load)
    }

    fn begin(&mut self, load: ImageLoadToken) -> ImageLoadToken {
        self.next_attempt = self.next_attempt.max(load.attempt().get());
        self.active.insert(load.image(), load.attempt());
        load
    }

    fn accept(&mut self, load: ImageLoadToken) -> bool {
        if self.active.get(&load.image()) != Some(&load.attempt()) {
            return false;
        }
        self.active.remove(&load.image());
        true
    }

    fn take_current(&mut self, outcome: WorkerDecodeOutcome) -> Option<WorkerDecodeOutcome> {
        self.accept(outcome.load()).then_some(outcome)
    }

    fn free(&mut self, image: ImageId) {
        self.active.remove(&image);
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
    /// Redisplay dimensions in logical Emacs pixels.  These can differ from
    /// the texture dimensions for a scalable image on a HiDPI display.
    pub width: u32,
    pub height: u32,
    /// GNU `Fimage_size` pixel extents (`img->width` / `img->height` space).
    /// For `:scale default` on HiDPI this is `ceil(layout × report_scale)`.
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// GNU's four-corner background guess, encoded as 0x00RRGGBB.
    pub background: u32,
    /// Whether GNU's four-corner mask heuristic classifies the background as transparent.
    pub background_transparent: bool,
}

/// Async image cache
pub struct ImageCache {
    /// Budget accounting events since the last drain (texture create/free).
    accounting: Vec<crate::media_budget::MediaAccounting>,
    /// Next image ID
    next_id: AtomicU32,
    /// Cached textures: id -> CachedImage
    textures: HashMap<ImageId, CachedImage>,
    /// Image states: id -> state
    states: HashMap<ImageId, ImageState>,
    /// Identifies the one decode request currently allowed to publish for each ID.
    loads: ImageLoadLifecycle,
    /// Pending dimensions (before texture is ready)
    pending_dimensions: HashMap<ImageId, ImageDimensions>,
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
fn lru_victim(entries: impl Iterator<Item = (ImageId, u64)>) -> Option<ImageId> {
    entries
        .min_by_key(|&(id, stamp)| (stamp, id))
        .map(|(id, _)| id)
}

/// Request to decode an image
struct DecodeRequest {
    load: ImageLoadToken,
    source: ImageSource,
    size: ImageSizeSpec,
    rotation: ImageRotation,
    /// Semantic and device geometry resolved by evaluator/layout.
    realization: ImageRealization,
    /// Resolved face colors used by face-sensitive formats and cache identity.
    colors: ImageColorContext,
}

/// Image source
enum ImageSource {
    File(String),
    Data {
        data: Vec<u8>,
        resources: crate::svg::SvgResourceContext,
    },
    #[cfg(test)]
    Panic,
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

        let pool_size = ImageDecoderPoolSize::detected();
        tracing::info!("Starting {} image decoder threads", pool_size.get());
        for i in 0..pool_size.get() {
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
            accounting: Vec::new(),
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

    fn begin_load(&mut self, load: ImageLoadToken) -> ImageLoadToken {
        let image = load.image();
        if let Some(cached) = self.textures.remove(&image) {
            self.total_memory -= cached.memory_size;
            self.accounting
                .push(crate::media_budget::MediaAccounting::Freed {
                    media_type: crate::media_budget::MediaType::Image,
                    id: image.get(),
                });
        }
        self.pending_dimensions.remove(&image);
        self.loads.begin(load)
    }

    fn begin_generated_load(&mut self, image: ImageId) -> ImageLoadToken {
        let load = self.loads.generated_token(image);
        self.begin_load(load)
    }

    fn allocate_image_id(&self) -> ImageId {
        let raw = self
            .next_id
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                current.checked_add(1)
            })
            .expect("image identity space exhausted");
        ImageId::new(raw)
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
                    tracing::debug!(
                        "Thread {} decoding image {}",
                        thread_id,
                        request.load.image()
                    );
                    let DecodeRequest {
                        load,
                        source,
                        size,
                        rotation,
                        realization,
                        colors,
                    } = request;
                    let result = catch_unwind(AssertUnwindSafe(|| match source {
                        #[cfg(test)]
                        ImageSource::Panic => panic!("injected decoder panic"),
                        ImageSource::File(path) => {
                            Self::decode_file(&path, size, rotation, colors, realization)
                        }
                        ImageSource::Data { data, resources } => {
                            Self::decode_data(&data, size, rotation, colors, realization, resources)
                        }
                        ImageSource::RawArgb32 {
                            data,
                            width,
                            height,
                            stride,
                        } => Self::convert_argb32_to_rgba(&data, width, height, stride)
                            .map(DecodedPixels::from_raster_tuple)
                            .and_then(|pixels| pixels.realize_bitmap(size, rotation, realization)),
                        ImageSource::RawRgb24 {
                            data,
                            width,
                            height,
                            stride,
                        } => Self::convert_rgb24_to_rgba(&data, width, height, stride)
                            .map(DecodedPixels::from_raster_tuple)
                            .and_then(|pixels| pixels.realize_bitmap(size, rotation, realization)),
                    }));

                    let outcome = match result {
                        Ok(Some(pixels)) => WorkerDecodeOutcome::Ready(Self::decoded_image(
                            load,
                            pixels,
                            realization,
                        )),
                        Ok(None) => WorkerDecodeOutcome::Failed(load),
                        Err(_) => {
                            tracing::warn!(
                                "Decoder thread {} recovered from a panic while decoding image {}",
                                thread_id,
                                load.image()
                            );
                            WorkerDecodeOutcome::Failed(load)
                        }
                    };
                    let _ = tx.send(outcome);
                }
                Err(_) => {
                    // Channel closed, exit thread
                    tracing::debug!("Decoder thread {} exiting", thread_id);
                    break;
                }
            }
        }
    }

    /// Decode image file with size constraints
    fn decode_file(
        path: &str,
        size: ImageSizeSpec,
        rotation: ImageRotation,
        colors: ImageColorContext,
        realization: ImageRealization,
    ) -> Option<DecodedPixels> {
        if let Ok(img) = image::open(path) {
            return Self::process_image(img)?.realize_bitmap(size, rotation, realization);
        }
        // Fallback: try XPM
        if let Some(result) = crate::xpm::decode_xpm_file(Path::new(path)) {
            return DecodedPixels::from_raster_tuple(result).realize_bitmap(
                size,
                rotation,
                realization,
            );
        }
        // Fallback: try XBM
        let fg = colors.foreground().rgba8();
        let bg = colors.background().rgba8();
        if let Some(result) = crate::xbm::decode_xbm_file(Path::new(path), fg, bg) {
            return DecodedPixels::from_raster_tuple(result).realize_bitmap(
                size,
                rotation,
                realization,
            );
        }
        // Fallback: try SVG via the shared vector backend.
        let data = std::fs::read(path).ok()?;
        Self::decode_svg_data(
            &data,
            size,
            rotation,
            realization,
            colors,
            crate::svg::SvgResourceContext::BaseUri(path.to_owned()),
        )
    }

    /// Decode image data with size constraints
    fn decode_data(
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        colors: ImageColorContext,
        realization: ImageRealization,
        resources: crate::svg::SvgResourceContext,
    ) -> Option<DecodedPixels> {
        if let Ok(img) = image::load_from_memory(data) {
            return Self::process_image(img)?.realize_bitmap(size, rotation, realization);
        }
        // Fallback: try XPM
        if let Some(result) = crate::xpm::decode_xpm_data(data) {
            return DecodedPixels::from_raster_tuple(result).realize_bitmap(
                size,
                rotation,
                realization,
            );
        }
        // Fallback: try XBM
        let fg = colors.foreground().rgba8();
        let bg = colors.background().rgba8();
        if let Some(result) = crate::xbm::decode_xbm_data(data, fg, bg) {
            return DecodedPixels::from_raster_tuple(result).realize_bitmap(
                size,
                rotation,
                realization,
            );
        }
        // Fallback: try SVG via the shared vector backend.
        Self::decode_svg_data(data, size, rotation, realization, colors, resources)
    }

    #[cfg(test)]
    fn decode_data_with_metadata(
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        fg_bg: (u32, u32),
    ) -> Option<DecodedImage> {
        Self::decode_data_with_metadata_at_scale(data, size, rotation, fg_bg, 1.0)
    }

    #[cfg(test)]
    fn decode_data_with_metadata_at_scale(
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        fg_bg: (u32, u32),
        raster_scale: f32,
    ) -> Option<DecodedImage> {
        Self::decode_data_with_metadata_at_realization(
            data,
            size,
            rotation,
            fg_bg,
            1.0,
            raster_scale,
        )
    }

    #[cfg(test)]
    fn decode_data_with_metadata_at_realization(
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        fg_bg: (u32, u32),
        layout_scale: f32,
        device_scale: f32,
    ) -> Option<DecodedImage> {
        // Convenience path: layout already equals image-pixel space.
        Self::decode_data_with_metadata_at_full_realization(
            data,
            size,
            rotation,
            fg_bg,
            ImageRealization::with_device_scale(layout_scale, device_scale),
        )
    }

    #[cfg(test)]
    fn decode_data_with_metadata_at_full_realization(
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        fg_bg: (u32, u32),
        realization: ImageRealization,
    ) -> Option<DecodedImage> {
        let pixels = Self::decode_data(
            data,
            size,
            rotation,
            ImageColorContext::from_pixels(fg_bg.0, fg_bg.1),
            realization,
            crate::svg::SvgResourceContext::Isolated,
        )?;
        Some(Self::decoded_image(
            ImageLoadToken::new(
                ImageId::new(0),
                ImageLoadAttempt::new(1).expect("test load attempt"),
            ),
            pixels,
            realization,
        ))
    }

    fn decoded_image(
        load: ImageLoadToken,
        pixels: DecodedPixels,
        _realization: ImageRealization,
    ) -> DecodedImage {
        let metadata = Self::metadata_from_rgba(
            pixels.layout_width,
            pixels.layout_height,
            pixels.pixel_width,
            pixels.pixel_height,
            pixels.raster_width,
            pixels.raster_height,
            &pixels.rgba,
        );
        DecodedImage {
            load,
            raster_width: pixels.raster_width,
            raster_height: pixels.raster_height,
            data: pixels.rgba,
            metadata,
        }
    }

    fn metadata_from_rgba(
        layout_width: u32,
        layout_height: u32,
        pixel_width: u32,
        pixel_height: u32,
        raster_width: u32,
        raster_height: u32,
        rgba: &[u8],
    ) -> ImageMetadata {
        let pixel = |x: u32, y: u32| {
            let offset = ((y * raster_width + x) * 4) as usize;
            [
                rgba[offset],
                rgba[offset + 1],
                rgba[offset + 2],
                rgba[offset + 3],
            ]
        };
        let corners = [
            pixel(0, 0),
            pixel(raster_width - 1, 0),
            pixel(raster_width - 1, raster_height - 1),
            pixel(0, raster_height - 1),
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
            width: layout_width,
            height: layout_height,
            pixel_width,
            pixel_height,
            background: (u32::from(background[0]) << 16)
                | (u32::from(background[1]) << 8)
                | u32::from(background[2]),
            background_transparent: mask[3] == 0,
        }
    }

    /// Decode SVG data through the platform SVG backend, returning RGBA pixels.
    fn decode_svg_data(
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        realization: ImageRealization,
        colors: ImageColorContext,
        resources: crate::svg::SvgResourceContext,
    ) -> Option<DecodedPixels> {
        let decoded = crate::svg::decode(data, size, rotation, realization, colors, resources)?;
        Some(DecodedPixels {
            layout_width: decoded.layout_width,
            layout_height: decoded.layout_height,
            pixel_width: decoded.pixel_width,
            pixel_height: decoded.pixel_height,
            raster_width: decoded.raster_width,
            raster_height: decoded.raster_height,
            rgba: decoded.rgba,
        })
    }

    /// Process decoded image: resize if needed, convert to RGBA
    /// Decode to NATIVE pixels. Sizing happens in `realize_bitmap`, which is
    /// the only place that knows both the native size and the requested one.
    fn process_image(img: image::DynamicImage) -> Option<DecodedPixels> {
        let rgba = img.to_rgba8();
        Some(DecodedPixels::raster(
            rgba.width(),
            rgba.height(),
            rgba.into_raw(),
        ))
    }
    fn convert_argb32_to_rgba(
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
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
        let (cw, ch) = constrain_dimensions(width, height);
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
        let (cw, ch) = constrain_dimensions(width, height);
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
        size: ImageSizeSpec,
        rotation: ImageRotation,
        colors: ImageColorContext,
        raster_scale: f32,
    ) -> ImageId {
        let image = self.allocate_image_id();
        let load = self.loads.generated_token(image);
        self.load_file_with_id(
            load,
            path,
            size,
            rotation,
            ImageRealization::with_device_scale(1.0, raster_scale),
            colors,
        );
        image
    }

    /// Load image from data with a pre-allocated ID (for threaded mode)
    pub fn load_data_with_id(
        &mut self,
        load: ImageLoadToken,
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        realization: ImageRealization,
        colors: ImageColorContext,
        resources: crate::svg::SvgResourceContext,
    ) {
        let load = self.begin_load(load);
        let image = load.image();
        // Query dimensions for the pending-image placeholder.
        if let Some(dims) = Self::query_data_dimensions(data) {
            let (w, h) = constrain_dimensions(dims.width, dims.height);
            self.pending_dimensions.insert(
                image,
                ImageDimensions {
                    width: realization.layout_dimension(w),
                    height: realization.layout_dimension(h),
                },
            );
        }

        // Queue for async decode
        self.states.insert(image, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::Data {
                data: data.to_vec(),
                resources,
            },
            size,
            rotation,
            realization,
            colors,
        });
    }

    /// Load image from file with a pre-allocated ID (for threaded mode)
    /// This allows the calling code to allocate the ID before sending a command.
    pub fn load_file_with_id(
        &mut self,
        load: ImageLoadToken,
        path: &str,
        size: ImageSizeSpec,
        rotation: ImageRotation,
        realization: ImageRealization,
        colors: ImageColorContext,
    ) {
        let load = self.begin_load(load);
        let image = load.image();
        // Query dimensions for the pending-image placeholder.
        if let Some(dims) = Self::query_file_dimensions(path) {
            // Apply max constraints to dimensions
            let (w, h) = constrain_dimensions(dims.width, dims.height);
            self.pending_dimensions.insert(
                image,
                ImageDimensions {
                    width: realization.layout_dimension(w),
                    height: realization.layout_dimension(h),
                },
            );
        }

        // Queue for async decode
        self.states.insert(image, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::File(path.to_string()),
            size,
            rotation,
            realization,
            colors,
        });
    }

    /// Allocate the next available image ID without loading anything.
    /// Used by threaded mode to pre-allocate IDs before sending commands.
    pub fn allocate_id(&self) -> ImageId {
        self.allocate_image_id()
    }

    /// Load image from data (async)
    pub fn load_data(
        &mut self,
        data: &[u8],
        size: ImageSizeSpec,
        rotation: ImageRotation,
        colors: ImageColorContext,
        raster_scale: f32,
    ) -> ImageId {
        let image = self.allocate_image_id();
        let load = self.begin_generated_load(image);

        // Query dimensions for the pending-image placeholder.
        if let Some(dims) = Self::query_data_dimensions(data) {
            let (w, h) = constrain_dimensions(dims.width, dims.height);
            self.pending_dimensions.insert(
                image,
                ImageDimensions {
                    width: w,
                    height: h,
                },
            );
        }

        // Queue for async decode
        self.states.insert(image, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::Data {
                data: data.to_vec(),
                resources: crate::svg::SvgResourceContext::Isolated,
            },
            size,
            rotation,
            realization: ImageRealization::with_device_scale(1.0, raster_scale),
            colors,
        });

        image
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
        size: ImageSizeSpec,
        rotation: ImageRotation,
    ) -> ImageId {
        let image = self.allocate_image_id();
        let load = self.begin_generated_load(image);

        // Store pending dimensions immediately (we know the exact size)
        let (w, h) = constrain_dimensions(width, height);
        self.pending_dimensions.insert(
            image,
            ImageDimensions {
                width: w,
                height: h,
            },
        );

        // Queue for async conversion
        self.states.insert(image, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::RawArgb32 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            size,
            rotation,
            realization: ImageRealization::default(),
            colors: ImageColorContext::default(),
        });

        image
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
        size: ImageSizeSpec,
        rotation: ImageRotation,
    ) -> ImageId {
        let image = self.allocate_image_id();
        let load = self.begin_generated_load(image);

        // Store pending dimensions immediately (we know the exact size)
        let (w, h) = constrain_dimensions(width, height);
        self.pending_dimensions.insert(
            image,
            ImageDimensions {
                width: w,
                height: h,
            },
        );

        // Queue for async conversion
        self.states.insert(image, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            load,
            source: ImageSource::RawRgb24 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            size,
            rotation,
            realization: ImageRealization::default(),
            colors: ImageColorContext::default(),
        });

        image
    }

    /// Load image from raw ARGB32 pixel data with a pre-allocated ID (for threaded mode)
    pub fn load_raw_argb32_with_id(
        &mut self,
        load: ImageLoadToken,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
    ) {
        let load = self.begin_load(load);
        let image = load.image();
        self.pending_dimensions
            .insert(image, ImageDimensions { width, height });
        self.states.insert(image, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            rotation: ImageRotation::None,
            load,
            source: ImageSource::RawArgb32 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            size: ImageSizeSpec::default(),
            realization: ImageRealization::default(),
            colors: ImageColorContext::default(),
        });
    }

    /// Load image from raw RGB24 pixel data with a pre-allocated ID (for threaded mode)
    pub fn load_raw_rgb24_with_id(
        &mut self,
        load: ImageLoadToken,
        data: &[u8],
        width: u32,
        height: u32,
        stride: u32,
    ) {
        let load = self.begin_load(load);
        let image = load.image();
        self.pending_dimensions
            .insert(image, ImageDimensions { width, height });
        self.states.insert(image, ImageState::Pending);
        let _ = self.decode_tx.send(DecodeRequest {
            rotation: ImageRotation::None,
            load,
            source: ImageSource::RawRgb24 {
                data: data.to_vec(),
                width,
                height,
                stride,
            },
            size: ImageSizeSpec::default(),
            realization: ImageRealization::default(),
            colors: ImageColorContext::default(),
        });
    }

    /// Import image from DMA-BUF (zero-copy if supported)
    #[cfg(target_os = "linux")]
    pub fn import_dmabuf(
        &mut self,
        dmabuf: DmaBufBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> ImageId {
        let image = self.allocate_image_id();
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
            self.accounting
                .push(crate::media_budget::MediaAccounting::Registered {
                    media_type: crate::media_budget::MediaType::Image,
                    id: image.get(),
                    size_bytes: memory_size,
                });

            self.textures.insert(
                image,
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
            self.states.insert(image, ImageState::Ready);

            tracing::info!(
                "Imported DMA-BUF image {} ({}x{}) zero-copy",
                image,
                width,
                height
            );
        } else {
            self.states
                .insert(image, ImageState::Failed("DMA-BUF import failed".into()));
            tracing::warn!("DMA-BUF import failed for image {}", image);
        }

        image
    }

    /// Process pending decoded images (call each frame)
    pub fn process_pending(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<ImageCacheEvent> {
        let mut events = Vec::new();
        // Drain decoded images from channel
        while let Ok(outcome) = self.decoded_rx.try_recv() {
            let Some(outcome) = self.loads.take_current(outcome) else {
                continue;
            };
            match outcome {
                WorkerDecodeOutcome::Ready(decoded) => {
                    events.push(ImageCacheEvent::Ready {
                        load: decoded.load,
                        metadata: decoded.metadata,
                    });
                    self.upload_texture(device, queue, decoded);
                }
                WorkerDecodeOutcome::Failed(load) => {
                    let error = "image decode failed".to_owned();
                    self.states
                        .insert(load.image(), ImageState::Failed(error.clone()));
                    self.pending_dimensions.remove(&load.image());
                    events.push(ImageCacheEvent::Failed { load, error });
                }
            }
        }

        // Evict if over memory limit
        self.evict_if_needed(&mut events);
        events
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
                width: decoded.raster_width,
                height: decoded.raster_height,
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
                bytes_per_row: Some(decoded.raster_width * 4),
                rows_per_image: Some(decoded.raster_height),
            },
            wgpu::Extent3d {
                width: decoded.raster_width,
                height: decoded.raster_height,
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

        let memory_size = (decoded.raster_width * decoded.raster_height * 4) as usize;
        self.total_memory += memory_size;
        self.accounting
            .push(crate::media_budget::MediaAccounting::Registered {
                media_type: crate::media_budget::MediaType::Image,
                id: decoded.load.image().get(),
                size_bytes: memory_size,
            });

        self.textures.insert(
            decoded.load.image(),
            CachedImage {
                texture,
                view,
                bind_group,
                width: decoded.raster_width,
                height: decoded.raster_height,
                metadata: Some(decoded.metadata),
                memory_size,
                last_access: Cell::new(self.next_access_stamp()),
            },
        );

        self.states.insert(decoded.load.image(), ImageState::Ready);
        self.pending_dimensions.remove(&decoded.load.image());

        tracing::debug!(
            "Uploaded image {} (layout {}x{}, raster {}x{}, {}KB)",
            decoded.load.image(),
            decoded.metadata.width,
            decoded.metadata.height,
            decoded.raster_width,
            decoded.raster_height,
            memory_size / 1024
        );
    }

    /// Evict least-recently-used textures until under the memory limit.
    fn evict_if_needed(&mut self, events: &mut Vec<ImageCacheEvent>) {
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
                self.accounting
                    .push(crate::media_budget::MediaAccounting::Freed {
                        media_type: crate::media_budget::MediaType::Image,
                        id: id.get(),
                    });
                events.push(ImageCacheEvent::Evicted { image: id });
                tracing::debug!(
                    "Evicted image {} to free {}KB",
                    id,
                    cached.memory_size / 1024
                );
            }
        }
    }

    /// Get cached image if ready. Refreshes the entry's LRU access stamp.
    pub fn get(&self, image: ImageId) -> Option<&CachedImage> {
        let cached = self.textures.get(&image)?;
        cached.last_access.set(self.next_access_stamp());
        Some(cached)
    }

    /// Get image dimensions (pending or loaded)
    pub fn get_dimensions(&self, image: ImageId) -> Option<ImageDimensions> {
        // Check loaded textures first
        if let Some(cached) = self.textures.get(&image) {
            return Some(ImageDimensions {
                width: cached.width,
                height: cached.height,
            });
        }
        // Check pending dimensions
        self.pending_dimensions.get(&image).copied()
    }

    /// Get image state
    pub fn get_state(&self, image: ImageId) -> Option<&ImageState> {
        self.states.get(&image)
    }

    /// Check if image is ready
    pub fn is_ready(&self, image: ImageId) -> bool {
        matches!(self.states.get(&image), Some(ImageState::Ready))
    }

    /// Whether async decode work still needs the render thread to poll its result channel.
    pub fn has_pending(&self) -> bool {
        self.states
            .values()
            .any(|state| matches!(state, ImageState::Pending | ImageState::Decoding))
    }

    /// Free an image from cache
    pub fn free(&mut self, image: ImageId) {
        self.loads.free(image);
        if let Some(cached) = self.textures.remove(&image) {
            self.total_memory -= cached.memory_size;
            self.accounting
                .push(crate::media_budget::MediaAccounting::Freed {
                    media_type: crate::media_budget::MediaType::Image,
                    id: image.get(),
                });
        }
        self.states.remove(&image);
        self.pending_dimensions.remove(&image);
    }

    /// Drain budget accounting events accumulated since the last call.
    pub fn drain_accounting(&mut self) -> Vec<crate::media_budget::MediaAccounting> {
        std::mem::take(&mut self.accounting)
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
