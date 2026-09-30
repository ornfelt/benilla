/// A wrapper around `wgpu` types; on benilla's targets it simply contains the wrapped value.
#[derive(Debug, Clone)]
pub struct WgpuWrapper<T>(T);

impl<T> WgpuWrapper<T> {
    /// Constructs a new instance of `WgpuWrapper` which will wrap the specified value.
    pub fn new(t: T) -> Self {
        Self(t)
    }

    /// Unwraps the value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

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
