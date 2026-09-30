use crate::define_atomic_id;
use crate::renderer::WgpuWrapper;
use crate::wgpu;
use core::ops::Deref;

define_atomic_id!(BufferId);

#[derive(Clone, Debug)]
pub struct Buffer {
    id: BufferId,
    value: WgpuWrapper<wgpu::Buffer>,
}

impl Buffer {
    #[inline]
    pub fn id(&self) -> BufferId {
        self.id
    }
}

impl From<wgpu::Buffer> for Buffer {
    fn from(value: wgpu::Buffer) -> Self {
        Buffer {
            id: BufferId::new(),
            value: WgpuWrapper::new(value),
        }
    }
}

impl Deref for Buffer {
    type Target = wgpu::Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
