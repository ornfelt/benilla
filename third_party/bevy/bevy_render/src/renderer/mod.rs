#[cfg(feature = "raw_vulkan_init")]
pub mod raw_vulkan_init;
mod render_device;
mod wgpu_wrapper;

pub use render_device::*;
pub use wgpu_wrapper::WgpuWrapper;

use crate::{
    diagnostic::{internal::DiagnosticsRecorder, RecordDiagnostics},
    render_phase::TrackedRenderPass,
    render_resource::RenderPassDescriptor,
};
use alloc::sync::Arc;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use wgpu::{Adapter, AdapterInfo, CommandBuffer, CommandEncoder, Instance, Queue};

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

/// The context with all information required to interact with the GPU.
///
/// The [`RenderDevice`] is used to create render resources and the
/// [`CommandEncoder`] is used to record a series of GPU operations.
pub struct RenderContext<'w> {
    render_device: RenderDevice,
    command_encoder: Option<CommandEncoder>,
    command_buffer_queue: Vec<QueuedCommandBuffer<'w>>,
    diagnostics_recorder: Option<Arc<DiagnosticsRecorder>>,
}

impl<'w> RenderContext<'w> {
    /// Creates a new [`RenderContext`] from a [`RenderDevice`].
    pub fn new(
        render_device: RenderDevice,
        diagnostics_recorder: Option<DiagnosticsRecorder>,
    ) -> Self {
        Self {
            render_device,
            command_encoder: None,
            command_buffer_queue: Vec::new(),
            diagnostics_recorder: diagnostics_recorder.map(Arc::new),
        }
    }

    /// Gets the underlying [`RenderDevice`].
    pub fn render_device(&self) -> &RenderDevice {
        &self.render_device
    }

    /// Gets the diagnostics recorder, used to track elapsed time and pipeline statistics
    /// of various render and compute passes.
    pub fn diagnostic_recorder(&self) -> impl RecordDiagnostics + use<> {
        self.diagnostics_recorder.clone()
    }

    /// Gets the current [`CommandEncoder`].
    pub fn command_encoder(&mut self) -> &mut CommandEncoder {
        self.command_encoder.get_or_insert_with(|| {
            self.render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        })
    }

    /// Creates a new [`TrackedRenderPass`] for the context,
    /// configured using the provided `descriptor`.
    pub fn begin_tracked_render_pass<'a>(
        &'a mut self,
        descriptor: RenderPassDescriptor<'_>,
    ) -> TrackedRenderPass<'a> {
        // Cannot use command_encoder() as we need to split the borrow on self
        let command_encoder = self.command_encoder.get_or_insert_with(|| {
            self.render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        });

        let render_pass = command_encoder.begin_render_pass(&descriptor);
        TrackedRenderPass::new(&self.render_device, render_pass)
    }

    /// Append a [`CommandBuffer`] to the command buffer queue.
    ///
    /// If present, this will flush the currently unflushed [`CommandEncoder`]
    /// into a [`CommandBuffer`] into the queue before appending the provided
    /// buffer.
    pub fn add_command_buffer(&mut self, command_buffer: CommandBuffer) {
        self.flush_encoder();

        self.command_buffer_queue
            .push(QueuedCommandBuffer::Ready(command_buffer));
    }

    /// Append a function that will generate a [`CommandBuffer`] to the
    /// command buffer queue, to be ran later.
    ///
    /// If present, this will flush the currently unflushed [`CommandEncoder`]
    /// into a [`CommandBuffer`] into the queue before appending the provided
    /// buffer.
    pub fn add_command_buffer_generation_task(
        &mut self,
        #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
        task: impl FnOnce(RenderDevice) -> CommandBuffer + 'w + Send,
        #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
        task: impl FnOnce(RenderDevice) -> CommandBuffer + 'w,
    ) {
        self.flush_encoder();

        self.command_buffer_queue
            .push(QueuedCommandBuffer::Task(Box::new(task)));
    }

    /// Finalizes and returns the queue of [`CommandBuffer`]s.
    ///
    /// This function will wait until all command buffer generation tasks are complete
    /// by running them in parallel (where supported).
    ///
    /// The [`CommandBuffer`]s will be returned in the order that they were added.
    pub fn finish(
        mut self,
    ) -> (
        Vec<CommandBuffer>,
        RenderDevice,
        Option<DiagnosticsRecorder>,
    ) {
        self.flush_encoder();

        let mut command_buffers = Vec::with_capacity(self.command_buffer_queue.len());

        #[cfg(feature = "trace")]
        let _command_buffer_generation_tasks_span =
            info_span!("command_buffer_generation_tasks").entered();

        #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
        {
            let mut task_based_command_buffers =
                bevy_tasks::ComputeTaskPool::get().scope(|task_pool| {
                    for (i, queued_command_buffer) in
                        self.command_buffer_queue.into_iter().enumerate()
                    {
                        match queued_command_buffer {
                            QueuedCommandBuffer::Ready(command_buffer) => {
                                command_buffers.push((i, command_buffer));
                            }
                            QueuedCommandBuffer::Task(command_buffer_generation_task) => {
                                let render_device = self.render_device.clone();
                                task_pool.spawn(async move {
                                    (i, command_buffer_generation_task(render_device))
                                });
                            }
                        }
                    }
                });
            command_buffers.append(&mut task_based_command_buffers);
        }

        #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
        for (i, queued_command_buffer) in self.command_buffer_queue.into_iter().enumerate() {
            match queued_command_buffer {
                QueuedCommandBuffer::Ready(command_buffer) => {
                    command_buffers.push((i, command_buffer));
                }
                QueuedCommandBuffer::Task(command_buffer_generation_task) => {
                    let render_device = self.render_device.clone();
                    command_buffers.push((i, command_buffer_generation_task(render_device)));
                }
            }
        }

        #[cfg(feature = "trace")]
        drop(_command_buffer_generation_tasks_span);

        command_buffers.sort_unstable_by_key(|(i, _)| *i);

        let mut command_buffers = command_buffers
            .into_iter()
            .map(|(_, cb)| cb)
            .collect::<Vec<CommandBuffer>>();

        let mut diagnostics_recorder = self.diagnostics_recorder.take().map(|v| {
            Arc::try_unwrap(v)
                .ok()
                .expect("diagnostic recorder shouldn't be held longer than necessary")
        });

        if let Some(recorder) = &mut diagnostics_recorder {
            let mut command_encoder = self
                .render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            recorder.resolve(&mut command_encoder);
            command_buffers.push(command_encoder.finish());
        }

        (command_buffers, self.render_device, diagnostics_recorder)
    }

    fn flush_encoder(&mut self) {
        if let Some(encoder) = self.command_encoder.take() {
            self.command_buffer_queue
                .push(QueuedCommandBuffer::Ready(encoder.finish()));
        }
    }
}

enum QueuedCommandBuffer<'w> {
    Ready(CommandBuffer),
    #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
    Task(Box<dyn FnOnce(RenderDevice) -> CommandBuffer + 'w + Send>),
    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    Task(Box<dyn FnOnce(RenderDevice) -> CommandBuffer + 'w>),
}
