use crate::render_resource::{BindGroupLayout, Buffer};
use crate::renderer::WgpuWrapper;
use crate::wgpu::{self, BindGroupLayoutEntry};
use bevy_ecs::resource::Resource;

/// This GPU device is responsible for the creation of most rendering and compute resources.
///
/// No device is opened (no `RenderApp` exists), so this is never constructed; the methods are the
/// ones the `AsBindGroup` derive's output and [`AsBindGroup`](crate::render_resource::AsBindGroup)
/// call.
#[derive(Resource, Clone)]
pub struct RenderDevice {
    device: WgpuWrapper<wgpu::Device>,
}

impl RenderDevice {
    /// List all [`Features`](wgpu::Features) that may be used with this device.
    ///
    /// Functions may panic if you use unsupported features.
    #[inline]
    pub fn features(&self) -> wgpu::Features {
        match *self.device {}
    }

    /// List all [`Limits`](wgpu::Limits) that were requested of this device.
    ///
    /// If any of these limits are exceeded, functions may panic.
    #[inline]
    pub fn limits(&self) -> wgpu::Limits {
        match *self.device {}
    }

    /// Creates a [`BindGroupLayout`](wgpu::BindGroupLayout).
    #[inline]
    pub fn create_bind_group_layout<'a>(
        &self,
        _label: impl Into<wgpu::Label<'a>>,
        _entries: &'a [BindGroupLayoutEntry],
    ) -> BindGroupLayout {
        match *self.device {}
    }

    /// Creates a [`Buffer`] and initializes it with the specified data.
    pub fn create_buffer_with_data(&self, _desc: &wgpu::util::BufferInitDescriptor) -> Buffer {
        match *self.device {}
    }
}
