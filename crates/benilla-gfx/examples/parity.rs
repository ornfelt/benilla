//! One fixed scene drawn through either path, for a numeric A/B of the gfx renderer against
//! wgpu: `parity wgpu` runs Bevy's own winit + wgpu renderer, `parity gfx` swaps in the gfx
//! DLL as benilla does. The camera is set up as benilla's world camera (`Hdr`,
//! `Tonemapping::None`), with MSAA off (`PARITY_MSAA=2|4|8` turns it on). The scene covers what the gfx renderer draws so far: an
//! sRGB texture sampled nearest and linear, BC1 blocks, vertex colours, back-face culling, depth,
//! alpha mask, alpha blend, additive blend, the rasterizer depth bias and gizmo lines (a list and a
//! strip through the default config: depth-tested, translucent, one clipped by the near plane).
//! Built with `--features egui`, `PARITY_EGUI=1` adds benilla's debug-panel overlay: an egui
//! context on a `Camera2d` above the scene, set up as `debug_panel::spawn_egui_camera` does, with
//! a panel of text, widgets, a filled rect and a clipped scroll area.
//!
//! `PARITY_WINDOW` sets the window's frame, sizing and place, for a window-manager A/B of both
//! paths (`xprop`, `xwininfo`): a comma list of `nodeco`, `fixed`, `center` and `at=<x>:<y>`,
//! applied at creation, or after 1.5 s with a leading `later,`.
//!
//! The window prints `parity: ready` once the scene has been on screen for a second and exits
//! two seconds later (with `WOW_GPU_MS=1` under gfx, also the gfx GPU meter's frame time then); `.claude/skills/gfx-dll-port/tools/parity.sh` captures it in between and
//! diffs the two captures.

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::Hdr;
use bevy::window::WindowResolution;

fn main() -> AppExit {
    let path = std::env::args().nth(1).unwrap_or_default();
    let gfx = match path.as_str() {
        "gfx" => true,
        "wgpu" => false,
        _ => {
            eprintln!("usage: parity <wgpu|gfx>");
            return AppExit::error();
        }
    };
    let spec = std::env::var("PARITY_WINDOW").unwrap_or_default();
    let later = spec.starts_with("later,");
    let mut window = Window {
        title: format!("benilla parity {path}"),
        // Resizable, as benilla's window is: a tiling window manager then gives both paths the
        // same slot.
        resolution: WindowResolution::new(640, 400).with_scale_factor_override(1.0),
        ..default()
    };
    if !later {
        window_spec(&mut window, &spec);
    }
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    });
    let plugins = if gfx {
        benilla_gfx::swap_in(plugins.build())
    } else {
        plugins.build()
    };
    let mut app = App::new();
    app.add_plugins(plugins);
    if gfx && std::env::var("WOW_GPU_MS").as_deref() == Ok("1") {
        app.insert_resource(benilla_gfx::GfxGpuMeter(Default::default()));
    }
    #[cfg(feature = "egui")]
    if std::env::var("PARITY_EGUI").as_deref() == Ok("1") {
        panel::add(&mut app, gfx);
    }
    #[cfg(not(feature = "egui"))]
    let _ = gfx;
    app.insert_resource(ClearColor(Color::srgb(0.2, 0.3, 0.45)))
        .add_systems(Startup, scene)
        .add_systems(Update, (clock, lines))
        .add_systems(Update, window_later.run_if(move || later))
        .run()
}

/// `PARITY_WINDOW`'s settings onto `window`.
fn window_spec(window: &mut Window, spec: &str) {
    for part in spec.split(',') {
        match part {
            "nodeco" => window.decorations = false,
            "fixed" => window.resizable = false,
            "center" => window.position = WindowPosition::Centered(MonitorSelection::Current),
            _ => {
                if let Some((x, y)) = part.strip_prefix("at=").and_then(|p| p.split_once(':')) {
                    if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                        window.position = WindowPosition::At(IVec2::new(x, y));
                    }
                }
            }
        }
    }
}

/// `PARITY_WINDOW=later,...`: the settings applied once, 1.5 s in.
fn window_later(time: Res<Time<Real>>, mut windows: Query<&mut Window>, mut done: Local<bool>) {
    if *done || time.elapsed_secs() < 1.5 {
        return;
    }
    *done = true;
    let spec = std::env::var("PARITY_WINDOW").unwrap_or_default();
    for mut window in &mut windows {
        window_spec(&mut window, &spec);
    }
}

fn clock(
    mut frames: Local<u32>,
    time: Res<Time<Real>>,
    meter: Option<Res<benilla_gfx::GfxGpuMeter>>,
    mut exit: MessageWriter<AppExit>,
) {
    *frames += 1;
    let t = time.elapsed_secs();
    if t >= 1.0 && *frames < u32::MAX / 2 {
        if let Some(meter) = meter {
            let ns = meter.0.load(std::sync::atomic::Ordering::Relaxed);
            println!("parity: gpu {:.3} ms", ns as f64 / 1e6);
        }
        println!("parity: ready");
        *frames = u32::MAX / 2;
    }
    if t >= 3.0 {
        exit.write(AppExit::Success);
    }
}

/// Gizmo lines as benilla draws them (the bowstring's two segments, the fishing line's sagging
/// strip), plus a translucent one, one through the cube and one running behind the camera.
fn lines(mut gizmos: Gizmos) {
    gizmos.line(
        Vec3::new(-2.0, 0.05, 1.5),
        Vec3::new(2.0, 1.6, -1.5),
        Color::srgb(0.12, 0.10, 0.08),
    );
    gizmos.line(
        Vec3::new(-2.2, 1.2, 0.0),
        Vec3::new(2.2, 0.3, 0.0),
        Color::srgba(1.0, 0.9, 0.2, 0.5),
    );
    gizmos.line_gradient(
        Vec3::new(1.5, 0.5, 8.0),
        Vec3::new(-1.0, 0.2, -2.0),
        Color::srgb(1.0, 0.1, 0.1),
        Color::srgb(0.1, 1.0, 0.1),
    );
    let (near, far) = (Vec3::new(-1.8, 1.8, -1.0), Vec3::new(1.6, 0.2, 1.2));
    gizmos.linestrip(
        (0..=64).map(|i| {
            let t = i as f32 / 64.0;
            near.lerp(far, t) - Vec3::Y * (0.5 * (std::f32::consts::PI * t).sin())
        }),
        Color::srgb(0.6, 0.8, 1.0),
    );
}

fn scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::None,
        match std::env::var("PARITY_MSAA").as_deref() {
            Ok("2") => Msaa::Sample2,
            Ok("4") => Msaa::Sample4,
            Ok("8") => Msaa::Sample8,
            _ => Msaa::Off,
        },
        Transform::from_xyz(0.0, 2.2, 5.0).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
        // The lit shapes' only light, over the global one, bright enough to read at the default
        // exposure.
        AmbientLight {
            brightness: 6000.0,
            ..default()
        },
    ));

    // A floor with a vertex-colour gradient.
    let mut floor = Plane3d::default()
        .mesh()
        .size(5.0, 5.0)
        .subdivisions(4)
        .build();
    let colors: Vec<[f32; 4]> = floor
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|p| p.as_float3())
        .unwrap_or_default()
        .iter()
        .map(|p| [0.5 + p[0] / 5.0, 0.3, 0.5 - p[2] / 5.0, 1.0])
        .collect();
    floor.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    commands.spawn((
        Mesh3d(meshes.add(floor)),
        MeshMaterial3d(materials.add(unlit(Color::WHITE))),
    ));

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    // An 8x8 sRGB checker, sampled nearest, on a tinted cube turned to show three faces.
    let checker = images.add(rgba8(8, 8, true, |x, y| {
        if (x + y) % 2 == 0 {
            [230, 60, 40, 255]
        } else {
            [240, 230, 200, 255]
        }
    }));
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(checker),
            ..unlit(Color::srgb(1.0, 0.9, 0.8))
        })),
        Transform::from_xyz(-1.4, 0.5, 0.0).with_rotation(Quat::from_rotation_y(0.5)),
    ));

    // BC1 sRGB blocks: four solid 4x4 blocks.
    commands.spawn((
        Mesh3d(cube),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(bc1_quadrants())),
            ..unlit(Color::WHITE)
        })),
        Transform::from_xyz(0.1, 0.5, -0.2).with_rotation(Quat::from_rotation_y(-0.4)),
    ));

    // A 2x2 sRGB texture magnified with linear filtering: where decode-after-filter shows.
    let mut gradient = rgba8(2, 2, true, |x, y| match (x, y) {
        (0, 0) => [255, 0, 0, 255],
        (1, 0) => [0, 255, 0, 255],
        (0, 1) => [0, 0, 255, 255],
        _ => [255, 255, 255, 255],
    });
    gradient.sampler = ImageSampler::linear();
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    commands.spawn((
        Mesh3d(quad.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(gradient)),
            ..unlit(Color::WHITE)
        })),
        Transform::from_xyz(1.5, 0.7, 0.2),
    ));

    // Alpha mask: a checker of holes, double-sided.
    let holes = images.add(rgba8(4, 4, true, |x, y| {
        [250, 250, 90, if (x + y) % 2 == 0 { 255 } else { 40 }]
    }));
    commands.spawn((
        Mesh3d(quad.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(holes),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            ..unlit(Color::WHITE)
        })),
        Transform::from_xyz(-0.5, 1.5, -0.9),
    ));

    // Alpha blend over the floor and the BC1 cube.
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.45).mesh().uv(24, 16))),
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..unlit(Color::srgba(0.2, 0.8, 1.0, 0.5))
        })),
        Transform::from_xyz(0.6, 0.5, 1.1),
    ));

    // Two squares coplanar with the floor, triangulated unlike it, so without a raster bias each
    // z-fights it: `StandardMaterial::depth_bias` +32768 (`Rung::DECAL_RASTER`'s size) wins
    // everywhere, -32768 loses everywhere.
    let decal = meshes.add(Plane3d::default().mesh().size(0.8, 0.8));
    for (x, bias, color) in [
        (-0.4, 32768.0, Color::srgb(0.9, 0.1, 0.8)),
        (1.4, -32768.0, Color::srgb(0.1, 0.9, 0.2)),
    ] {
        commands.spawn((
            Mesh3d(decal.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                depth_bias: bias,
                ..unlit(color)
            })),
            Transform::from_xyz(x, 0.0, 1.7).with_rotation(Quat::from_rotation_y(0.3)),
        ));
    }

    // Lit under the ambient alone: a cube as `entities.rs`'s fallback, and a metallic sphere with
    // an emissive, whose normals sweep N.V.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.6, 0.6))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.1, 0.85, 0.9),
            perceptual_roughness: 0.7,
            ..default()
        })),
        Transform::from_xyz(1.3, 1.7, -1.0).with_rotation(Quat::from_rotation_y(0.7)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.5, 0.2),
            metallic: 0.6,
            perceptual_roughness: 0.3,
            emissive: LinearRgba::rgb(0.05, 0.0, 0.1),
            ..default()
        })),
        Transform::from_xyz(-1.7, 1.6, -1.0),
    ));

    // Additive over the sky and the mask quad.
    commands.spawn((
        Mesh3d(quad),
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Add,
            ..unlit(Color::srgba(1.0, 0.5, 0.1, 0.8))
        })),
        Transform::from_xyz(0.4, 1.6, -0.6),
    ));
}

fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    }
}

fn rgba8(w: u32, h: u32, srgb: bool, texel: impl Fn(u32, u32) -> [u8; 4]) -> Image {
    let data = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| texel(x, y))
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        if srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// An 8x8 BC1 image whose four 4x4 blocks are solid colours (both endpoints the colour, every
/// index 0).
fn bc1_quadrants() -> Image {
    let rgb565 = |r: u16, g: u16, b: u16| (r >> 3) << 11 | (g >> 2) << 5 | (b >> 3);
    let colors = [
        rgb565(200, 40, 200),
        rgb565(40, 200, 80),
        rgb565(248, 200, 40),
        rgb565(40, 120, 248),
    ];
    let data = colors
        .iter()
        .flat_map(|c| {
            let [lo, hi] = c.to_le_bytes();
            [lo, hi, lo, hi, 0, 0, 0, 0]
        })
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Bc1RgbaUnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// benilla's egui overlay over the scene (`PARITY_EGUI=1`).
#[cfg(feature = "egui")]
mod panel {
    use bevy::camera::visibility::RenderLayers;
    use bevy::camera::{CameraOutputMode, ClearColorConfig};
    use bevy::prelude::*;
    use bevy::render::render_resource::BlendState;
    use bevy_egui::PrimaryEguiContext;
    use bevy_egui::{egui, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass};

    pub(super) fn add(app: &mut App, gfx: bool) {
        app.add_plugins(EguiPlugin::default());
        app.world_mut()
            .resource_mut::<EguiGlobalSettings>()
            .auto_create_primary_context = false;
        if gfx {
            app.add_plugins(benilla_gfx::GfxEguiPlugin);
        }
        app.add_systems(Startup, camera)
            .add_systems(EguiPrimaryContextPass, ui);
    }

    /// `debug_panel::spawn_egui_camera`'s camera, at order 1 over the scene's.
    fn camera(mut commands: Commands) {
        commands.spawn((
            PrimaryEguiContext,
            Camera2d,
            bevy::render::view::Msaa::Off,
            RenderLayers::none(),
            Camera {
                order: 1,
                output_mode: CameraOutputMode::Write {
                    blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    clear_color: ClearColorConfig::None,
                },
                clear_color: ClearColorConfig::Custom(Color::NONE),
                msaa_writeback: bevy::camera::MsaaWriteback::Off,
                ..default()
            },
        ));
    }

    fn ui(mut contexts: EguiContexts, mut value: Local<f32>) -> Result {
        let ctx = contexts.ctx_mut()?;
        *value = 0.35;
        egui::Window::new("parity")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0))
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(224))
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(10, 8)),
            )
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("benilla parity").strong());
                ui.label(
                    egui::RichText::new("dim text 0123456789").color(egui::Color32::from_gray(180)),
                );
                ui.label(egui::RichText::new("monospace 1.5  -2.25").monospace());
                ui.add(egui::Slider::new(&mut *value, 0.0..=1.0).text("grade"));
                let mut on = true;
                ui.checkbox(&mut on, "a checkbox");
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(120.0, 16.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 2.0, egui::Color32::from_rgb(200, 120, 40));
                ui.painter().rect_filled(
                    rect.shrink(4.0),
                    0.0,
                    egui::Color32::from_rgba_premultiplied(0, 80, 160, 128),
                );
                egui::ScrollArea::vertical()
                    .max_height(60.0)
                    .show(ui, |ui| {
                        for i in 0..12 {
                            ui.label(format!("clipped row {i}"));
                        }
                    });
            });
        Ok(())
    }
}
