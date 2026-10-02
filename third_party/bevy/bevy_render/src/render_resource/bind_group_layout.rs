use crate::{define_atomic_id, renderer::WgpuWrapper, wgpu};
use core::ops::Deref;

define_atomic_id!(BindGroupLayoutId);

/// Bind group layouts define the interface of resources (e.g. buffers, textures, samplers)
/// for a shader. The actual resource binding is done via a [`BindGroup`](super::BindGroup).
///
/// This is a lightweight thread-safe wrapper around wgpu's own [`BindGroupLayout`](wgpu::BindGroupLayout),
/// which can be cloned as needed to workaround lifetime management issues. It may be converted
/// from and dereferences to wgpu's [`BindGroupLayout`](wgpu::BindGroupLayout).
///
/// Can be created via [`RenderDevice::create_bind_group_layout`](crate::renderer::RenderDevice::create_bind_group_layout).
#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    value: WgpuWrapper<wgpu::BindGroupLayout>,
}

impl PartialEq for BindGroupLayout {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for BindGroupLayout {}

impl core::hash::Hash for BindGroupLayout {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.0.hash(state);
    }
}

impl Deref for BindGroupLayout {
    type Target = wgpu::BindGroupLayout;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
