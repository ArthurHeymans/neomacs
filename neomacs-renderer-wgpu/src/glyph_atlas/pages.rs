use std::num::NonZeroU32;

use super::allocator::{Allocation, ShelfAllocator};
use super::types::*;

pub(crate) struct AtlasPage<M: GlyphMaterial> {
    pub id: PageId<M>,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub allocator: ShelfAllocator,
    pub last_accessed_generation: u64,
}

pub(crate) struct PageAllocResult<M: GlyphMaterial> {
    pub page_id: PageId<M>,
    pub allocation: Allocation,
}

pub(crate) struct GlyphAtlasPages {
    pub alpha: Vec<AtlasPage<AlphaMask>>,
    pub subpixel: Vec<AtlasPage<SubpixelMask>>,
    pub color: Vec<AtlasPage<ColorRgba>>,
    next_page_id: u32,
    config: GlyphAtlasConfig,
}

impl GlyphAtlasPages {
    pub fn new(config: GlyphAtlasConfig) -> Self {
        Self {
            alpha: Vec::new(),
            subpixel: Vec::new(),
            color: Vec::new(),
            next_page_id: 1,
            config,
        }
    }

    pub fn clear(&mut self) {
        self.alpha.clear();
        self.subpixel.clear();
        self.color.clear();
        self.next_page_id = 1;
    }

    fn next_page_id_raw(&mut self) -> NonZeroU32 {
        let id = NonZeroU32::new(self.next_page_id).unwrap_or_else(|| {
            self.next_page_id = 2;
            NonZeroU32::new(1).unwrap()
        });
        self.next_page_id += 1;
        id
    }

    pub fn page_counts(&self) -> (usize, usize, usize) {
        (self.alpha.len(), self.subpixel.len(), self.color.len())
    }

    fn create_page_texture(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        page_size: u32,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: page_size,
                height: page_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        view: &wgpu::TextureView,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn allocate_alpha(
        &mut self,
        size: PixelSize,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Option<PageAllocResult<AlphaMask>> {
        for page in &mut self.alpha {
            if let Some(allocation) = page.allocator.allocate(size) {
                return Some(PageAllocResult {
                    page_id: page.id,
                    allocation,
                });
            }
        }
        if self.alpha.len() >= self.config.max_pages_per_material {
            return None;
        }
        let id = PageId::new(self.next_page_id_raw());
        let (texture, view) = Self::create_page_texture(
            device,
            AlphaMask::TEXTURE_FORMAT,
            self.config.page_size,
            "Atlas Alpha Page",
        );
        let bind_group = Self::create_bind_group(
            device,
            layout,
            sampler,
            &view,
            "Atlas Alpha Page Bind Group",
        );
        let allocator = ShelfAllocator::new(self.config.page_size, self.config.padding);
        self.alpha.push(AtlasPage {
            id,
            texture,
            view,
            bind_group,
            allocator,
            last_accessed_generation: 0,
        });
        let page = self.alpha.last_mut().unwrap();
        let allocation = page.allocator.allocate(size)?;
        Some(PageAllocResult {
            page_id: id,
            allocation,
        })
    }

    pub fn allocate_subpixel(
        &mut self,
        size: PixelSize,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Option<PageAllocResult<SubpixelMask>> {
        for page in &mut self.subpixel {
            if let Some(allocation) = page.allocator.allocate(size) {
                return Some(PageAllocResult {
                    page_id: page.id,
                    allocation,
                });
            }
        }
        if self.subpixel.len() >= self.config.max_pages_per_material {
            return None;
        }
        let id = PageId::new(self.next_page_id_raw());
        let (texture, view) = Self::create_page_texture(
            device,
            SubpixelMask::TEXTURE_FORMAT,
            self.config.page_size,
            "Atlas Subpixel Page",
        );
        let bind_group = Self::create_bind_group(
            device,
            layout,
            sampler,
            &view,
            "Atlas Subpixel Page Bind Group",
        );
        let allocator = ShelfAllocator::new(self.config.page_size, self.config.padding);
        self.subpixel.push(AtlasPage {
            id,
            texture,
            view,
            bind_group,
            allocator,
            last_accessed_generation: 0,
        });
        let page = self.subpixel.last_mut().unwrap();
        let allocation = page.allocator.allocate(size)?;
        Some(PageAllocResult {
            page_id: id,
            allocation,
        })
    }

    pub fn allocate_color(
        &mut self,
        size: PixelSize,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) -> Option<PageAllocResult<ColorRgba>> {
        for page in &mut self.color {
            if let Some(allocation) = page.allocator.allocate(size) {
                return Some(PageAllocResult {
                    page_id: page.id,
                    allocation,
                });
            }
        }
        if self.color.len() >= self.config.max_pages_per_material {
            return None;
        }
        let id = PageId::new(self.next_page_id_raw());
        let (texture, view) = Self::create_page_texture(
            device,
            ColorRgba::TEXTURE_FORMAT,
            self.config.page_size,
            "Atlas Color Page",
        );
        let bind_group = Self::create_bind_group(
            device,
            layout,
            sampler,
            &view,
            "Atlas Color Page Bind Group",
        );
        let allocator = ShelfAllocator::new(self.config.page_size, self.config.padding);
        self.color.push(AtlasPage {
            id,
            texture,
            view,
            bind_group,
            allocator,
            last_accessed_generation: 0,
        });
        let page = self.color.last_mut().unwrap();
        let allocation = page.allocator.allocate(size)?;
        Some(PageAllocResult {
            page_id: id,
            allocation,
        })
    }

    pub fn alpha_page(&self, id: PageId<AlphaMask>) -> Option<&AtlasPage<AlphaMask>> {
        self.alpha.iter().find(|p| p.id == id)
    }

    pub fn subpixel_page(&self, id: PageId<SubpixelMask>) -> Option<&AtlasPage<SubpixelMask>> {
        self.subpixel.iter().find(|p| p.id == id)
    }

    pub fn color_page(&self, id: PageId<ColorRgba>) -> Option<&AtlasPage<ColorRgba>> {
        self.color.iter().find(|p| p.id == id)
    }

    pub fn touch_alpha(&mut self, id: PageId<AlphaMask>, generation: u64) {
        if let Some(page) = self.alpha.iter_mut().find(|p| p.id == id) {
            page.last_accessed_generation = generation;
        }
    }

    pub fn touch_subpixel(&mut self, id: PageId<SubpixelMask>, generation: u64) {
        if let Some(page) = self.subpixel.iter_mut().find(|p| p.id == id) {
            page.last_accessed_generation = generation;
        }
    }

    pub fn touch_color(&mut self, id: PageId<ColorRgba>, generation: u64) {
        if let Some(page) = self.color.iter_mut().find(|p| p.id == id) {
            page.last_accessed_generation = generation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_counts_start_at_zero() {
        let pages = GlyphAtlasPages::new(GlyphAtlasConfig::default());
        assert_eq!(pages.page_counts(), (0, 0, 0));
    }

    #[test]
    fn next_page_id_increments() {
        let mut pages = GlyphAtlasPages::new(GlyphAtlasConfig::default());
        let id1 = pages.next_page_id_raw();
        let id2 = pages.next_page_id_raw();
        assert!(id2.get() > id1.get());
    }

    #[test]
    fn clear_resets_page_id_counter() {
        let mut pages = GlyphAtlasPages::new(GlyphAtlasConfig::default());
        let _ = pages.next_page_id_raw();
        let _ = pages.next_page_id_raw();
        pages.clear();
        let id_after = pages.next_page_id_raw();
        assert_eq!(id_after.get(), 1);
    }
}
