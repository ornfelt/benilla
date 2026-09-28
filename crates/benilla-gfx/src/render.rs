//! The gfx frame: after each `app.update()` the runner runs [`GfxRender`] on the main world and
//! presents. [`GfxRenderSystems`] orders it: main-world data is packed into the data textures,
//! asset and data changes reach the device stores, the
//! registered materials list what is visible, the cameras draw into the scene target, and the
//! target is encoded into the window.

use bevy::ecs::schedule::{ExecutorKind, ScheduleLabel};
use bevy::prelude::*;

use crate::draw::{self, DrawList};

/// The schedule that draws one frame through gfx, run by the runner after `Update`'s frame.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GfxRender;

/// The ordered stages of [`GfxRender`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GfxRenderSystems {
    /// Main-world state the shaders read by index packed into [`crate::data::GfxDataTextures`],
    /// as the render world's upload systems write their storage buffers.
    Pack,
    /// Asset changes applied to the device stores; the draw list reset.
    Prepare,
    /// The material collectors fill the draw list ([`crate::material::GfxMaterialPlugin`]).
    Collect,
    /// Every camera draws into the scene target.
    Draw,
    /// The scene target is encoded into the window; later passes draw over it.
    Present,
}

pub(crate) fn build(app: &mut App) {
    let mut schedule = Schedule::new(GfxRender);
    // Every system here holds the non-send context; one thread, in order.
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    app.add_schedule(schedule)
        .init_resource::<DrawList>()
        .init_resource::<crate::overlay::GfxOverlays>()
        .init_resource::<crate::data::GfxDataTextures>()
        .init_resource::<crate::images::GfxTextureWrites>()
        .init_resource::<crate::probe::GfxDepthProbe>()
        .init_resource::<crate::probe::GfxPhaseRecord>()
        .configure_sets(
            GfxRender,
            (
                GfxRenderSystems::Pack,
                GfxRenderSystems::Prepare,
                GfxRenderSystems::Collect,
                GfxRenderSystems::Draw,
                GfxRenderSystems::Present,
            )
                .chain(),
        )
        .add_systems(
            GfxRender,
            (
                (
                    crate::screenshot::collect,
                    crate::probe::collect_depth,
                    draw::prepare,
                )
                    .chain()
                    .in_set(GfxRenderSystems::Prepare),
                draw::draw_views.in_set(GfxRenderSystems::Draw),
                (draw::present, crate::screenshot::capture, draw::log_stats)
                    .chain()
                    .in_set(GfxRenderSystems::Present),
            ),
        );
}
