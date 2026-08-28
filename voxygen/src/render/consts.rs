use super::buffer::DynamicBuffer;
use bytemuck::Pod;

/// A handle to a series of constants sitting on the GPU. This is used to hold
/// information used in the rendering process that does not change throughout a
/// single render pass.
pub struct Consts<T: Copy + Pod> {
    buf: DynamicBuffer<T>,
    cpu_values: Vec<T>,
}

impl<T: Copy + Pod> Consts<T> {
    /// Create a new `Const<T>`.
    pub fn new(device: &wgpu::Device, len: usize) -> Self {
        Self {
            // TODO: examine if all our consts need to be updatable
            buf: DynamicBuffer::new(device, len, wgpu::BufferUsages::UNIFORM),
            cpu_values: vec![T::zeroed(); len],
        }
    }

    /// Update the GPU-side value represented by this constant handle.
    pub fn update(&mut self, queue: &wgpu::Queue, vals: &[T], offset: usize) {
        self.buf.update(queue, vals, offset);
        if let Some(target) = self
            .cpu_values
            .get_mut(offset..offset.saturating_add(vals.len()))
        {
            target.copy_from_slice(vals);
        }
    }

    pub fn buf(&self) -> &wgpu::Buffer { &self.buf.buf }

    pub(crate) fn values(&self) -> &[T] { &self.cpu_values }
}
