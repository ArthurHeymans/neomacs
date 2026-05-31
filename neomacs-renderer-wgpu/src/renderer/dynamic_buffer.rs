use bytemuck::Pod;
use std::marker::PhantomData;

pub struct FrameVertexArena<T: Pod> {
    buffer: Option<wgpu::Buffer>,
    capacity_bytes: wgpu::BufferAddress,
    cursor_bytes: wgpu::BufferAddress,
    retired: Vec<wgpu::Buffer>,
    label: &'static str,
    _marker: PhantomData<T>,
}

pub struct VertexUpload {
    pub offset_bytes: wgpu::BufferAddress,
    pub len_bytes: wgpu::BufferAddress,
    pub vertex_count: u32,
}

impl VertexUpload {
    pub fn vertex_range(&self) -> std::ops::Range<u32> {
        let stride = self.len_bytes / self.vertex_count as wgpu::BufferAddress;
        let start = (self.offset_bytes / stride) as u32;
        start..start + self.vertex_count
    }
}

const ALIGN: wgpu::BufferAddress = 4;

fn align_up(offset: wgpu::BufferAddress, align: wgpu::BufferAddress) -> wgpu::BufferAddress {
    (offset + align - 1) & !(align - 1)
}

impl<T: Pod> FrameVertexArena<T> {
    pub fn new(label: &'static str) -> Self {
        Self {
            buffer: None,
            capacity_bytes: 0,
            cursor_bytes: 0,
            retired: Vec::new(),
            label,
            _marker: PhantomData,
        }
    }

    pub fn begin_frame(&mut self) {
        self.cursor_bytes = 0;
        self.retired.clear();
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[T],
    ) -> Option<VertexUpload> {
        if vertices.is_empty() {
            return None;
        }

        let bytes = bytemuck::cast_slice(vertices);
        let len = bytes.len() as wgpu::BufferAddress;
        let offset = align_up(self.cursor_bytes, ALIGN);
        let end = offset + len;

        self.ensure_capacity(device, end);

        let buffer = self.buffer.as_ref().unwrap();
        queue.write_buffer(buffer, offset, bytes);
        self.cursor_bytes = end;

        Some(VertexUpload {
            offset_bytes: offset,
            len_bytes: len,
            vertex_count: vertices.len() as u32,
        })
    }

    pub fn slice(&self, upload: &VertexUpload) -> wgpu::BufferSlice<'_> {
        let buffer = self.buffer.as_ref().unwrap();
        buffer.slice(upload.offset_bytes..upload.offset_bytes + upload.len_bytes)
    }

    fn ensure_capacity(&mut self, device: &wgpu::Device, needed_bytes: wgpu::BufferAddress) {
        if needed_bytes <= self.capacity_bytes {
            return;
        }

        let new_capacity = if self.capacity_bytes == 0 {
            needed_bytes.max(4096)
        } else {
            let mut c = self.capacity_bytes;
            while c < needed_bytes {
                c *= 2;
            }
            c
        };

        if let Some(old) = self.buffer.take() {
            self.retired.push(old);
        }

        self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: new_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.capacity_bytes = new_capacity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_upload_range_calculation() {
        let upload = VertexUpload {
            offset_bytes: 0,
            len_bytes: 48,
            vertex_count: 2,
        };
        assert_eq!(upload.vertex_range(), 0..2);
    }

    #[test]
    fn vertex_upload_range_with_offset() {
        let upload = VertexUpload {
            offset_bytes: 48,
            len_bytes: 48,
            vertex_count: 2,
        };
        assert_eq!(upload.vertex_range(), 2..4);
    }

    #[test]
    fn align_up_basic() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(48, 4), 48);
        assert_eq!(align_up(49, 4), 52);
    }
}
