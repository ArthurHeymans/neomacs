use bytemuck::Pod;
use std::marker::PhantomData;

pub struct DynamicVertexBuffer<T: Pod> {
    buffer: Option<wgpu::Buffer>,
    capacity: usize,
    label: &'static str,
    _marker: PhantomData<T>,
}

impl<T: Pod> DynamicVertexBuffer<T> {
    pub fn new(label: &'static str) -> Self {
        Self {
            buffer: None,
            capacity: 0,
            label,
            _marker: PhantomData,
        }
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[T],
    ) -> wgpu::BufferSlice<'_> {
        let needed = vertices.len();
        let byte_len = std::mem::size_of::<T>() * needed.max(1);

        if needed > self.capacity {
            let new_capacity = if self.capacity == 0 {
                needed.max(64)
            } else {
                let mut c = self.capacity;
                while c < needed {
                    c *= 2;
                }
                c
            };

            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: (std::mem::size_of::<T>() * new_capacity) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.capacity = new_capacity;
        }

        let buffer = self.buffer.as_ref().unwrap();
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(vertices));
        buffer.slice(..byte_len as wgpu::BufferAddress)
    }
}
