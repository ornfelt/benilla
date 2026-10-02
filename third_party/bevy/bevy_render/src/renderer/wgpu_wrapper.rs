/// A wrapper around `wgpu` types; on benilla's targets it simply contains the wrapped value.
#[derive(Debug, Clone)]
pub struct WgpuWrapper<T>(T);

impl<T> core::ops::Deref for WgpuWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> core::ops::DerefMut for WgpuWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
