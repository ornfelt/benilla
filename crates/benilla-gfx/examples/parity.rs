//! One fixed scene drawn through either path, for a numeric A/B of the gfx renderer against
//! wgpu: `parity wgpu` runs Bevy's own winit + wgpu renderer, `parity gfx` swaps in the gfx
//! DLL as benilla does. The camera is set up as benilla's world camera (`Hdr`,
//! `Tonemapping::None`), with MSAA off. The scene covers what the gfx renderer draws so far: an
//! sRGB texture sampled nearest and linear, BC1 blocks, vertex colours, back-face culling, depth,
//! alpha mask, alpha blend and additive blend.
//!
//! The window prints `parity: ready` once the scene has been on screen for a second and exits
//! two seconds later; `.claude/skills/gfx-dll-port/tools/parity.sh` captures it in between and
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
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: format!("benilla parity {path}"),
            // Resizable, as benilla's window is: a tiling window manager then gives both paths the
            // same slot (gfx does not apply `resizable: false` yet).
            resolution: WindowResolution::new(640, 400).with_scale_factor_override(1.0),
            ..default()
        }),
        ..default()
    });
    let plugins = if gfx {
        benilla_gfx::swap_in(plugins.build())
    } else {
        plugins.build()
    };
    App::new()
        .add_plugins(plugins)
        .insert_resource(ClearColor(Color::srgb(0.2, 0.3, 0.45)))
        .add_systems(Startup, scene)
        .add_systems(Update, clock)
        .run()
}

fn clock(mut frames: Local<u32>, time: Res<Time<Real>>, mut exit: MessageWriter<AppExit>) {
    *frames += 1;
    let t = time.elapsed_secs();
    if t >= 1.0 && *frames < u32::MAX / 2 {
        println!("parity: ready");
        *frames = u32::MAX / 2;
    }
    if t >= 3.0 {
        exit.write(AppExit::Success);
    }
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
        Msaa::Off,
        Transform::from_xyz(0.0, 2.2, 5.0).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
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
