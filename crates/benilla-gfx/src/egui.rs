//! bevy_egui's output through gfx, behind the `egui` feature: what bevy_egui 0.39.1's render
//! systems (`render/systems.rs`) take from each context camera's `EguiRenderOutput` becomes that
//! camera's [`GfxOverlayFrame`], drawn by [`crate::overlay`]. egui's textures are already
//! `Assets<Image>` (`update_egui_textures_system`, main world), so they reach the device as every
//! image does; a partial update is a whole re-upload.
//!
//! Not drawn, each logged once: paint callbacks (`EguiBevyPaintCallback`, wgpu render code) and
//! user textures (`EguiUserTextures` keeps no public map from texture id to image). benilla uses
//! neither.

use bevy::math::URect;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::{EguiContextSettings, EguiGlobalSettings, EguiManagedTextures, EguiRenderOutput};

use crate::overlay::{GfxOverlayFrame, GfxOverlays, OVERLAY_VERTEX};
use crate::render::{GfxRender, GfxRenderSystems};

/// Draws bevy_egui's contexts through gfx. Add it after `EguiPlugin`.
pub struct GfxEguiPlugin;

impl Plugin for GfxEguiPlugin {
    fn build(&self, app: &mut App) {
        // gfx has no IME, and bevy_egui's IME system asks a winit window for one every frame.
        if let Some(mut settings) = app.world_mut().get_resource_mut::<EguiGlobalSettings>() {
            settings.enable_ime = false;
        }
        app.add_systems(GfxRender, collect.in_set(GfxRenderSystems::Collect));
    }
}

/// Each active context camera's paint jobs as its overlay frame: `prepare_egui_transforms_system`'s
/// transform and `prepare_egui_render_target_data_system`'s draws (clip rects in physical pixels,
/// a primitive whose clip misses the viewport skipped).
fn collect(
    mut overlays: ResMut<GfxOverlays>,
    contexts: Query<(
        Entity,
        &Camera,
        &EguiRenderOutput,
        Option<&EguiContextSettings>,
    )>,
    managed: Option<Res<EguiManagedTextures>>,
    mut vertices: Local<Vec<u8>>,
) {
    overlays.0.clear();
    for (entity, camera, output, settings) in &contexts {
        if !camera.is_active || output.paint_jobs.is_empty() {
            continue;
        }
        let (Some(scale), Some(target), Some(viewport)) = (
            camera.target_scaling_factor(),
            camera.physical_target_size(),
            camera.physical_viewport_rect(),
        ) else {
            continue;
        };
        let ppp = scale * settings.map_or(1.0, |s| s.scale_factor);
        let logical = target.as_vec2() / ppp;
        let mut frame = GfxOverlayFrame {
            transform: [2.0 / logical.x, -2.0 / logical.y, -1.0, 1.0],
            ..default()
        };
        for egui::ClippedPrimitive {
            clip_rect,
            primitive,
        } in &output.paint_jobs
        {
            let clip = URect {
                min: UVec2::new(
                    (clip_rect.min.x * ppp).round() as u32,
                    (clip_rect.min.y * ppp).round() as u32,
                ),
                max: UVec2::new(
                    (clip_rect.max.x * ppp).round() as u32,
                    (clip_rect.max.y * ppp).round() as u32,
                ),
            };
            let scissor = clip.intersect(viewport);
            if scissor.is_empty() {
                continue;
            }
            let egui::epaint::Primitive::Mesh(mesh) = primitive else {
                warn_once!("gfx: an egui paint callback is not drawn");
                continue;
            };
            let image = match mesh.texture_id {
                egui::TextureId::Managed(id) => managed
                    .as_ref()
                    .and_then(|m| m.get(&(entity, id)))
                    .map(|t| t.handle.id()),
                egui::TextureId::User(_) => {
                    warn_once!("gfx: an egui user texture is not drawn");
                    None
                }
            };
            let Some(image) = image else {
                continue;
            };
            vertices.clear();
            vertices.reserve(mesh.vertices.len() * OVERLAY_VERTEX);
            for v in &mesh.vertices {
                for f in [v.pos.x, v.pos.y, v.uv.x, v.uv.y] {
                    vertices.extend_from_slice(&f.to_le_bytes());
                }
                vertices.extend_from_slice(&v.color.to_array());
            }
            frame.push(&vertices, &mesh.indices, image, scissor);
        }
        overlays.0.insert(entity, frame);
    }
}
