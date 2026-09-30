mod render_device;
mod wgpu_wrapper;

pub use render_device::*;
pub use wgpu_wrapper::WgpuWrapper;

use alloc::sync::Arc;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use wgpu::{Adapter, AdapterInfo, Instance, Queue};

/// This queue is used to enqueue tasks for the GPU to execute asynchronously.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderQueue(pub Arc<WgpuWrapper<Queue>>);

/// The handle to the physical device being used for rendering.
/// See [`Adapter`] for more info.
#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct RenderAdapter(pub Arc<WgpuWrapper<Adapter>>);

/// The GPU instance is used to initialize the [`RenderQueue`] and [`RenderDevice`],
/// as well as to create [`WindowSurfaces`](crate::view::window::WindowSurfaces).
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderInstance(pub Arc<WgpuWrapper<Instance>>);

/// The [`AdapterInfo`] of the adapter in use by the renderer.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct RenderAdapterInfo(pub WgpuWrapper<AdapterInfo>);
