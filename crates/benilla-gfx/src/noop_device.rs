//! A stand-in `RenderDevice` and `RenderQueue` for the main world, on wgpu's `noop` backend: no
//! GPU, no driver, no surface. Main-world code that makes wgpu handles at startup (the shared light
//! buffer in `WorldAssets` and the material extensions, the model-frame tile buffers) keeps
//! working unchanged; what it creates is never drawn, and a queue write goes nowhere. The gfx
//! renderer draws from the CPU-side data those systems also keep.
//!
//! Inserted by the runner after every plugin's `finish`, so a plugin that reads the device at
//! `finish` (the BC probe, `view`'s sample counts) sees what a headless app sees: no device.

use bevy::prelude::*;
use bevy::render::renderer::{RenderDevice, RenderQueue, WgpuWrapper};
use std::sync::Arc;

pub fn insert(world: &mut World) {
    let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor {
        label: Some("benilla-gfx noop"),
        ..default()
    });
    world.insert_resource(RenderDevice::from(device));
    world.insert_resource(RenderQueue(Arc::new(WgpuWrapper::new(queue))));
}
