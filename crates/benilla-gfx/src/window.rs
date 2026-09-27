//! The window half of bevy_winit 0.18.1 over the gfx window. The runner's [`pump`] turns each
//! polled gfx event into the message winit's would have become and keeps the primary [`Window`]
//! true to the window (`state.rs::window_event`, `device_event`); the [`Last`] systems send
//! `Window`, [`CursorOptions`] and [`CursorIcon`] changes the other way (`system.rs::
//! changed_windows`, `changed_cursor_options`, `cursor::update_cursors`) and release the held keys
//! when focus goes (`check_keyboard_focus_lost`).
//!
//! `WOW_GFX_INPUT_TRACE=<path>` writes one line per message sent and per call made to the window.

use std::fs::File;
use std::io::{LineWriter, Write};
use std::time::Instant;

use bevy::ecs::message::Message;
use bevy::input::keyboard::{Key, KeyboardFocusLost, KeyboardInput, NativeKeyCode};
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::input::ButtonState;
use bevy::math::{DVec2, IVec2};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::window::{
    CursorEntered, CursorGrabMode, CursorIcon, CursorLeft, CursorMoved, CursorOptions,
    CustomCursor, PresentMode, PrimaryWindow, SystemCursorIcon, WindowCreated, WindowEvent,
    WindowFocused, WindowMoved, WindowResized,
};

use crate::backend::Backends;
use crate::context::GfxContext;
use crate::ffi::{
    self, event_type, key_mods, native_cursor, GfxCursor, GfxEvent, GfxWindowBackend,
};
use crate::input::{self, Scancodes};

pub(crate) fn build(app: &mut App) {
    app.insert_resource(InputTrace::from_env()).add_systems(
        Last,
        (
            sync_windows,
            sync_cursor_options,
            sync_cursor_icon,
            check_keyboard_focus_lost,
        )
            .chain(),
    );
}

/// What the gfx window was last told or last reported, so only the app's own changes go back
/// (bevy_winit's `CachedWindow`).
#[derive(Component)]
pub(crate) struct CachedWindow(Window);

/// bevy_winit's `CachedCursorOptions`.
#[derive(Component)]
pub(crate) struct CachedCursorOptions(CursorOptions);

/// The keys held down, with the meaning each was pressed with, released on focus loss (bevy_winit's
/// `WinitWindowPressedKeys`).
#[derive(Component, Default)]
pub(crate) struct PressedKeys(HashMap<KeyCode, Key>);

/// The pointer as the window last showed it: grabbed, hidden, and the cursor it wears.
pub struct CursorState {
    pub grabbed: bool,
    pub visible: bool,
    /// What the pointer shows when visible; null is the system's default pointer.
    pub icon: GfxCursor,
    /// Every cursor made, freed with the window.
    made: HashMap<CursorKey, GfxCursor>,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            grabbed: false,
            visible: true,
            icon: std::ptr::null_mut(),
            made: HashMap::default(),
        }
    }
}

/// The window state gfx last reported, so a repeat of it is not news.
#[derive(Default)]
pub struct Reported {
    pub size: UVec2,
    pub focus: Option<bool>,
}

/// A cursor worth keeping: an image (with the part of it shown), a system shape, or the hidden one.
#[derive(Clone, PartialEq, Eq, Hash)]
enum CursorKey {
    Image {
        id: AssetId<Image>,
        rect: Option<URect>,
        flip_x: bool,
        flip_y: bool,
        hotspot: (u16, u16),
    },
    Native(u32),
    Hidden,
}

impl CursorState {
    /// Frees every cursor made; the window is still alive.
    pub(crate) fn clear(&mut self, window: ffi::GfxWindow) {
        // SAFETY: the live window, and no cursor is shown once the null default is set.
        unsafe {
            ffi::gfx_dll_set_cursor(window, std::ptr::null_mut());
            for (_, cursor) in self.made.drain() {
                ffi::gfx_dll_delete_cursor(window, cursor);
            }
        }
        self.icon = std::ptr::null_mut();
    }

    /// Shows what the state says: the icon, or the hidden cursor.
    fn apply(&mut self, window: ffi::GfxWindow) {
        let shown = if self.visible {
            self.icon
        } else {
            self.hidden(window)
        };
        // SAFETY: the live window and a cursor made on it (or null).
        unsafe { ffi::gfx_dll_set_cursor(window, shown) };
    }

    /// A fully transparent cursor: hiding it this way works the same on every backend, where
    /// their own blank cursor does not (glfw and sdl have none).
    fn hidden(&mut self, window: ffi::GfxWindow) -> GfxCursor {
        *self.made.entry(CursorKey::Hidden).or_insert_with(|| {
            let pixels = [0u8; 16 * 16 * 4];
            // SAFETY: the live window and a 16×16 RGBA8 buffer, copied by the call.
            unsafe { ffi::gfx_dll_create_cursor(window, pixels.as_ptr().cast(), 16, 16, 0, 0) }
        })
    }

    fn native(&mut self, window: ffi::GfxWindow, shape: u32) -> GfxCursor {
        *self
            .made
            .entry(CursorKey::Native(shape))
            // SAFETY: the live window; `shape` is a `native_cursor` value.
            .or_insert_with(|| unsafe { ffi::gfx_dll_create_native_cursor(window, shape) })
    }
}

/// The `WOW_GFX_INPUT_TRACE` file, if the run asked for one.
#[derive(Resource)]
pub(crate) struct InputTrace {
    out: Option<LineWriter<File>>,
    start: Instant,
    pub(crate) frame: u64,
}

impl InputTrace {
    fn from_env() -> Self {
        let out = std::env::var_os("WOW_GFX_INPUT_TRACE").and_then(|path| {
            File::create(&path)
                .map_err(|e| warn!("gfx: WOW_GFX_INPUT_TRACE {path:?}: {e}"))
                .ok()
                .map(LineWriter::new)
        });
        Self {
            out,
            start: Instant::now(),
            frame: 0,
        }
    }

    fn on(&self) -> bool {
        self.out.is_some()
    }

    fn line(&mut self, what: std::fmt::Arguments) {
        if let Some(out) = &mut self.out {
            let _ = writeln!(
                out,
                "{:.4} f{} {what}",
                self.start.elapsed().as_secs_f64(),
                self.frame
            );
        }
    }
}

/// Writes a message and its [`WindowEvent`] twin, as bevy_winit's `forward_bevy_events` does.
fn send<M: Message + Clone + std::fmt::Debug + Into<WindowEvent>>(world: &mut World, msg: M) {
    let mut trace = world.resource_mut::<InputTrace>();
    if trace.on() {
        trace.line(format_args!("{msg:?}"));
    }
    world.write_message(msg.clone().into());
    world.write_message(msg);
}

/// The scancode family of a window backend: the platform's own on x11, win32 and glfw.
fn scancodes(backends: &Backends) -> Scancodes {
    match backends.window {
        GfxWindowBackend::Sdl => Scancodes::Sdl,
        GfxWindowBackend::Win32 => Scancodes::Windows,
        _ if cfg!(windows) => Scancodes::Windows,
        _ => Scancodes::Xkb,
    }
}

/// Called by the runner once the window is open: the backend's scale factor and real size go into
/// the `Window` (bevy_winit's `create_windows`), and the caches start from them.
pub(crate) fn opened(world: &mut World, entity: Entity) {
    let (scale, size) = {
        let mut ctx = world.non_send_resource_mut::<GfxContext>();
        let size = ctx.size();
        ctx.reported.size = size;
        (ctx.scale_factor(), size)
    };
    let Some(mut window) = world.get_mut::<Window>(entity) else {
        return;
    };
    window.resolution.set_scale_factor(scale);
    window.resolution.set_physical_resolution(size.x, size.y);
    let cached = window.clone();
    let cursor = world
        .get::<CursorOptions>(entity)
        .cloned()
        .unwrap_or_default();
    world.entity_mut(entity).insert((
        CachedWindow(cached),
        CachedCursorOptions(cursor.clone()),
        PressedKeys::default(),
    ));
    info!(
        "gfx: window scale factor {scale} ({}x{} physical)",
        size.x, size.y
    );
    send(world, WindowCreated { window: entity });
    // What the app asked for before the window existed.
    apply_cursor_options(world, entity, &cursor);
}

/// Hands one poll's events to the app, in order.
pub(crate) fn pump(world: &mut World, events: &[GfxEvent]) {
    let mut primary = world.query_filtered::<Entity, With<PrimaryWindow>>();
    let Ok(entity) = primary.single(world) else {
        return;
    };
    let (family, grabbed) = {
        let ctx = world.non_send_resource::<GfxContext>();
        (scancodes(&ctx.backends), ctx.cursor.grabbed)
    };
    let mut touched = false;
    let mut i = 0;
    while i < events.len() {
        let evt = &events[i];
        i += 1;
        match evt.event_type {
            event_type::KEY_DOWN | event_type::KEY_PRESS | event_type::KEY_UP => {
                // SAFETY: the type says the union holds a key event.
                let k = unsafe { evt.data.key };
                let pressed = evt.event_type != event_type::KEY_UP;
                // The text a press typed follows it (every backend orders it so); it is the
                // press's `text`, as winit's `KeyEvent::text`.
                let mut text_buf = None;
                if pressed {
                    if let Some(next) = events.get(i) {
                        if next.event_type == event_type::CHARACTER {
                            // SAFETY: the type says the union holds a character event.
                            text_buf = Some(unsafe { next.data.character }.utf8);
                            i += 1;
                        }
                    }
                }
                let text = text_buf.as_ref().and_then(input::character_text);
                let key_code = input::physical_key(k.physical, k.key, k.scancode, family);
                let shift = k.mods & key_mods::SHIFT != 0;
                let logical = {
                    let mut held = world.get_mut::<PressedKeys>(entity);
                    let held = held.as_deref_mut();
                    if pressed {
                        let logical = input::logical_key(k.key, text, shift);
                        if let Some(held) = held {
                            held.0.insert(key_code, logical.clone());
                        }
                        logical
                    } else {
                        // A release means what its press did, as winit reports it.
                        held.and_then(|h| h.0.remove(&key_code))
                            .unwrap_or_else(|| input::logical_key(k.key, None, shift))
                    }
                };
                send(
                    world,
                    KeyboardInput {
                        key_code,
                        logical_key: logical,
                        state: if pressed {
                            ButtonState::Pressed
                        } else {
                            ButtonState::Released
                        },
                        text: text.map(Into::into),
                        repeat: evt.event_type == event_type::KEY_PRESS,
                        window: entity,
                    },
                );
            }
            event_type::CHARACTER => {
                // Text no key press claimed (an input method's commit): typed, and let go.
                // SAFETY: the type says the union holds a character event.
                let utf8 = unsafe { evt.data.character }.utf8;
                if let Some(text) = input::character_text(&utf8) {
                    for state in [ButtonState::Pressed, ButtonState::Released] {
                        send(
                            world,
                            KeyboardInput {
                                key_code: KeyCode::Unidentified(NativeKeyCode::Unidentified),
                                logical_key: Key::Character(text.into()),
                                state,
                                text: (state == ButtonState::Pressed).then(|| text.into()),
                                repeat: false,
                                window: entity,
                            },
                        );
                    }
                }
            }
            event_type::MOUSE_DOWN | event_type::MOUSE_UP => {
                // SAFETY: the type says the union holds a mouse event.
                let m = unsafe { evt.data.mouse };
                send(
                    world,
                    MouseButtonInput {
                        button: input::mouse_button(m.button),
                        state: if evt.event_type == event_type::MOUSE_DOWN {
                            ButtonState::Pressed
                        } else {
                            ButtonState::Released
                        },
                        window: entity,
                    },
                );
            }
            event_type::MOUSE_MOVE => {
                // While grabbed the backends report a virtual position that runs off the window;
                // the pointer itself stays where the grab took it, as a locked winit cursor does.
                if !grabbed {
                    // SAFETY: the type says the union holds a pointer event.
                    let p = unsafe { evt.data.pointer };
                    touched |= cursor_moved(world, entity, p.x, p.y);
                }
            }
            event_type::RAW_MOTION => {
                // SAFETY: the type says the union holds a motion event.
                let m = unsafe { evt.data.motion };
                send(
                    world,
                    MouseMotion {
                        delta: Vec2::new(m.dx, m.dy),
                    },
                );
            }
            event_type::SCROLL => {
                // SAFETY: the type says the union holds a scroll event.
                let s = unsafe { evt.data.scroll };
                // Whole notches: gfx's wheel is in lines, up and right positive like winit's.
                send(
                    world,
                    MouseWheel {
                        unit: MouseScrollUnit::Line,
                        x: s.x as f32,
                        y: s.y as f32,
                        window: entity,
                    },
                );
            }
            event_type::CURSOR_ENTER => send(world, CursorEntered { window: entity }),
            event_type::CURSOR_LEAVE => {
                // A grab can report the pointer leaving while it is held in place.
                if !grabbed {
                    if let Some(mut window) = world.get_mut::<Window>(entity) {
                        window.set_physical_cursor_position(None);
                        touched = true;
                    }
                }
                send(world, CursorLeft { window: entity });
            }
            event_type::FOCUS_IN | event_type::FOCUS_OUT => {
                let focused = evt.event_type == event_type::FOCUS_IN;
                // Only a change is news: glfw repeats a focus it already reported, winit does not.
                {
                    let mut ctx = world.non_send_resource_mut::<GfxContext>();
                    if ctx.reported.focus == Some(focused) {
                        continue;
                    }
                    ctx.reported.focus = Some(focused);
                }
                let Some(mut window) = world.get_mut::<Window>(entity) else {
                    continue;
                };
                window.focused = focused;
                touched = true;
                send(
                    world,
                    WindowFocused {
                        window: entity,
                        focused,
                    },
                );
            }
            event_type::RESIZE => {
                // SAFETY: the type says the union holds a resize event.
                let r = unsafe { evt.data.resize };
                // A size the window already reported is no resize (glfw reports its first twice).
                {
                    let size = UVec2::new(r.width, r.height);
                    let mut ctx = world.non_send_resource_mut::<GfxContext>();
                    if ctx.reported.size == size {
                        continue;
                    }
                    ctx.reported.size = size;
                }
                let Some(mut window) = world.get_mut::<Window>(entity) else {
                    continue;
                };
                window.resolution.set_physical_resolution(r.width, r.height);
                let (width, height) = (window.width(), window.height());
                touched = true;
                send(
                    world,
                    WindowResized {
                        window: entity,
                        width,
                        height,
                    },
                );
            }
            event_type::MOVE => {
                // SAFETY: the type says the union holds a move event.
                let m = unsafe { evt.data.mov };
                let position = IVec2::new(m.x, m.y);
                if let Some(mut window) = world.get_mut::<Window>(entity) {
                    window.position.set(position);
                    touched = true;
                }
                send(
                    world,
                    WindowMoved {
                        window: entity,
                        position,
                    },
                );
            }
            // Every frame is drawn anyway; the close request is the runner's.
            _ => {}
        }
    }
    // What the window reported is not the app's change to send back (bevy_winit refreshes its
    // `CachedWindow` after each event).
    if touched {
        let current = world.get::<Window>(entity).cloned();
        if let (Some(current), Some(mut cache)) = (current, world.get_mut::<CachedWindow>(entity)) {
            cache.0 = current;
        }
    }
}

/// A pointer at `(x, y)` physical: `CursorMoved` with the logical position and the delta from the
/// last one (bevy_winit's `CursorMoved` arm).
fn cursor_moved(world: &mut World, entity: Entity, x: i32, y: i32) -> bool {
    let Some(mut window) = world.get_mut::<Window>(entity) else {
        return false;
    };
    let physical = DVec2::new(f64::from(x), f64::from(y));
    let last = window.physical_cursor_position();
    let scale = window.resolution.scale_factor();
    let delta = last.map(|last| (physical.as_vec2() - last) / scale);
    window.set_physical_cursor_position(Some(physical));
    let position = (physical / f64::from(scale)).as_vec2();
    send(
        world,
        CursorMoved {
            window: entity,
            position,
            delta,
        },
    );
    true
}

/// The swap interval a present mode asks for: vsync unless it names an uncapped mode.
pub(crate) fn swap_interval(mode: PresentMode) -> i32 {
    match mode {
        PresentMode::AutoNoVsync | PresentMode::Immediate | PresentMode::Mailbox => 0,
        PresentMode::AutoVsync | PresentMode::Fifo | PresentMode::FifoRelaxed => 1,
    }
}

/// `Window` changes to the gfx window (bevy_winit's `changed_windows`). What gfx has no call for
/// is named once in the log.
fn sync_windows(
    ctx: NonSend<GfxContext>,
    mut windows: Query<(&Window, &mut CachedWindow), Changed<Window>>,
    mut trace: ResMut<InputTrace>,
    mut unsupported: Local<HashSet<&'static str>>,
) {
    for (window, mut cache) in &mut windows {
        let gfx = ctx.window;
        if window.title != cache.0.title {
            let title = std::ffi::CString::new(window.title.replace('\0', "")).unwrap_or_default();
            // SAFETY: the live window and a NUL-terminated title, copied by the call.
            unsafe { ffi::gfx_dll_window_set_title(gfx, title.as_ptr()) };
            trace.line(format_args!("gfx set_title {:?}", window.title));
        }
        let size = window.resolution.physical_size();
        if size != cache.0.resolution.physical_size() && size.x > 0 && size.y > 0 {
            // The window answers with a resize event, which sets the size it really got.
            // SAFETY: the live window.
            unsafe { ffi::gfx_dll_window_resize(gfx, size.x, size.y) };
            trace.line(format_args!("gfx resize {}x{}", size.x, size.y));
        }
        if window.physical_cursor_position() != cache.0.physical_cursor_position() {
            if let Some(p) = window.physical_cursor_position() {
                let (x, y) = (p.x.round() as i32, p.y.round() as i32);
                // SAFETY: the live window.
                unsafe { ffi::gfx_dll_set_mouse_position(gfx, x, y) };
                trace.line(format_args!("gfx set_mouse_position {x} {y}"));
            }
        }
        if window.present_mode != cache.0.present_mode {
            let interval = swap_interval(window.present_mode);
            // SAFETY: the live window.
            unsafe { ffi::gfx_dll_set_swap_interval(gfx, interval) };
            trace.line(format_args!("gfx set_swap_interval {interval}"));
        }
        if window.visible != cache.0.visible {
            // SAFETY: the live window.
            unsafe {
                if window.visible {
                    ffi::gfx_dll_window_show(gfx);
                } else {
                    ffi::gfx_dll_window_hide(gfx);
                }
            }
        }
        for (field, changed) in [
            ("mode (fullscreen)", window.mode != cache.0.mode),
            ("window_level", window.window_level != cache.0.window_level),
            ("position", window.position != cache.0.position),
            ("decorations", window.decorations != cache.0.decorations),
            ("resizable", window.resizable != cache.0.resizable),
            (
                "focus request",
                window.focused != cache.0.focused && window.focused,
            ),
        ] {
            if changed && unsupported.insert(field) {
                info!("gfx: Window {field} is not applied to the gfx window yet");
            }
        }
        cache.0 = window.clone();
    }
}

/// `CursorOptions` changes to the pointer (bevy_winit's `changed_cursor_options`), checked each
/// change and not against the cache for the grab, as winit's is.
fn sync_cursor_options(
    world: &mut World,
    changed: &mut QueryState<(Entity, &CursorOptions), Changed<CursorOptions>>,
) {
    let changed: Vec<(Entity, CursorOptions)> =
        changed.iter(world).map(|(e, o)| (e, o.clone())).collect();
    for (entity, options) in changed {
        apply_cursor_options(world, entity, &options);
    }
}

fn apply_cursor_options(world: &mut World, entity: Entity, options: &CursorOptions) {
    let cached = world
        .get::<CachedCursorOptions>(entity)
        .map(|c| c.0.clone())
        .unwrap_or_default();
    let mut applied = options.clone();
    let mut lines = Vec::new();
    {
        let mut ctx = world.non_send_resource_mut::<GfxContext>();
        let gfx = ctx.window;
        // gfx's grab hides the pointer and reports the device's movement: winit's `Locked`.
        // `Confined` (a visible pointer kept inside) has no gfx call; like a grab winit refuses,
        // it is reverted to what the window has.
        let grab = match options.grab_mode {
            CursorGrabMode::None => false,
            CursorGrabMode::Locked => true,
            CursorGrabMode::Confined => {
                warn!("gfx: CursorGrabMode::Confined is not supported; the grab is left as it is");
                applied.grab_mode = cached.grab_mode;
                ctx.cursor.grabbed
            }
        };
        if grab != ctx.cursor.grabbed {
            // SAFETY: the live window, on its thread.
            unsafe {
                if grab {
                    ffi::gfx_dll_grab_cursor(gfx);
                } else {
                    ffi::gfx_dll_ungrab_cursor(gfx);
                }
            }
            ctx.cursor.grabbed = grab;
            lines.push(format!("gfx grab {grab}"));
        }
        if options.visible != ctx.cursor.visible {
            ctx.cursor.visible = options.visible;
            ctx.cursor.apply(gfx);
            lines.push(format!("gfx cursor visible {}", options.visible));
        }
    }
    let mut trace = world.resource_mut::<InputTrace>();
    for line in lines {
        trace.line(format_args!("{line}"));
    }
    if applied.grab_mode != options.grab_mode {
        if let Some(mut live) = world.get_mut::<CursorOptions>(entity) {
            live.bypass_change_detection().grab_mode = applied.grab_mode;
        }
    }
    if let Some(mut cache) = world.get_mut::<CachedCursorOptions>(entity) {
        cache.0 = applied;
    }
}

/// The gfx shape nearest a system cursor; those gfx has no shape for are the arrow.
fn native_shape(icon: SystemCursorIcon) -> u32 {
    use SystemCursorIcon as S;
    match icon {
        S::Crosshair => native_cursor::CROSS,
        S::Pointer | S::Grab | S::Grabbing => native_cursor::HAND,
        S::Text | S::VerticalText => native_cursor::IBEAM,
        S::NotAllowed | S::NoDrop => native_cursor::NO,
        S::Move | S::AllScroll => native_cursor::SIZEALL,
        S::NsResize | S::NResize | S::SResize | S::RowResize => native_cursor::VRESIZE,
        S::EwResize | S::EResize | S::WResize | S::ColResize => native_cursor::HRESIZE,
        S::Wait | S::Progress => native_cursor::WAIT,
        _ => native_cursor::ARROW,
    }
}

/// `CursorIcon` changes to the pointer (bevy_winit's `update_cursors`): an image cursor is made
/// from the image's pixels once it is loaded, and kept; a removed icon is the default pointer.
fn sync_cursor_icon(
    mut ctx: NonSendMut<GfxContext>,
    windows: Query<(Entity, Ref<CursorIcon>), With<Window>>,
    mut removed: RemovedComponents<CursorIcon>,
    images: Res<Assets<Image>>,
    atlases: Option<Res<Assets<TextureAtlasLayout>>>,
    mut trace: ResMut<InputTrace>,
    mut waiting: Local<HashSet<Entity>>,
) {
    let gfx = ctx.window;
    for entity in removed.read() {
        if windows.get(entity).is_err() {
            ctx.cursor.icon = std::ptr::null_mut();
            ctx.cursor.apply(gfx);
            trace.line(format_args!("gfx cursor default"));
        }
    }
    for (entity, icon) in &windows {
        if !(waiting.remove(&entity) || icon.is_changed()) {
            continue;
        }
        let made = match icon.as_ref() {
            CursorIcon::System(SystemCursorIcon::Default) => Some(std::ptr::null_mut()),
            CursorIcon::System(shape) => Some(ctx.cursor.native(gfx, native_shape(*shape))),
            CursorIcon::Custom(CustomCursor::Image(c)) => {
                let key = CursorKey::Image {
                    id: c.handle.id(),
                    rect: c.rect,
                    flip_x: c.flip_x,
                    flip_y: c.flip_y,
                    hotspot: c.hotspot,
                };
                if let Some(&cursor) = ctx.cursor.made.get(&key) {
                    Some(cursor)
                } else if let Some(image) = images.get(&c.handle) {
                    let rect = c.rect.or_else(|| {
                        let atlas = c.texture_atlas.as_ref()?;
                        atlases
                            .as_ref()?
                            .get(&atlas.layout)?
                            .textures
                            .get(atlas.index)
                            .copied()
                    });
                    match cursor_pixels(image, rect, c.flip_x, c.flip_y) {
                        Some((w, h, rgba)) => {
                            let (hx, hy) = hotspot(c.hotspot, w, h, c.flip_x, c.flip_y);
                            // SAFETY: the live window and a w×h RGBA8 buffer, copied by the call.
                            let cursor = unsafe {
                                ffi::gfx_dll_create_cursor(gfx, rgba.as_ptr().cast(), w, h, hx, hy)
                            };
                            if cursor.is_null() {
                                warn!("gfx: cursor image {:?} was refused", c.handle);
                                None
                            } else {
                                ctx.cursor.made.insert(key, cursor);
                                Some(cursor)
                            }
                        }
                        None => {
                            warn!(
                                "gfx: cursor image {:?} is not RGBA8 with CPU-side data",
                                c.handle
                            );
                            None
                        }
                    }
                } else {
                    // Not loaded yet: asked again next frame, as bevy_winit's queue does.
                    waiting.insert(entity);
                    None
                }
            }
            CursorIcon::Custom(CustomCursor::Url(_)) => None,
        };
        if let Some(cursor) = made {
            ctx.cursor.icon = cursor;
            ctx.cursor.apply(gfx);
            trace.line(format_args!("gfx cursor icon {:?}", icon.as_ref()));
        }
    }
}

/// The RGBA8 pixels of the part of `image` a cursor shows, flipped as asked: width, height, bytes.
fn cursor_pixels(
    image: &Image,
    rect: Option<URect>,
    flip_x: bool,
    flip_y: bool,
) -> Option<(u32, u32, Vec<u8>)> {
    use bevy::render::render_resource::TextureFormat;
    let format = image.texture_descriptor.format;
    if !matches!(
        format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
    ) {
        return None;
    }
    let data = image.data.as_ref()?;
    let (iw, ih) = (image.width(), image.height());
    let rect = rect.unwrap_or(URect::new(0, 0, iw, ih));
    let (x0, y0) = (rect.min.x.min(iw), rect.min.y.min(ih));
    let (w, h) = (rect.max.x.min(iw) - x0, rect.max.y.min(ih) - y0);
    if w == 0 || h == 0 || data.len() < (iw * ih * 4) as usize {
        return None;
    }
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for oy in 0..h {
        let sy = y0 + if flip_y { h - 1 - oy } else { oy };
        for ox in 0..w {
            let sx = x0 + if flip_x { w - 1 - ox } else { ox };
            let i = ((sy * iw + sx) * 4) as usize;
            out.extend_from_slice(&data[i..i + 4]);
        }
    }
    Some((w, h, out))
}

/// The hotspot within a flipped cursor (bevy_winit's `transform_hotspot`).
fn hotspot(hotspot: (u16, u16), w: u32, h: u32, flip_x: bool, flip_y: bool) -> (u32, u32) {
    let (x, y) = (
        u32::from(hotspot.0).min(w - 1),
        u32::from(hotspot.1).min(h - 1),
    );
    (
        if flip_x { w - 1 - x } else { x },
        if flip_y { h - 1 - y } else { y },
    )
}

/// On focus loss, every held key is released (bevy_winit's `check_keyboard_focus_lost`): the
/// window never sees their release.
fn check_keyboard_focus_lost(
    mut focus: MessageReader<WindowFocused>,
    mut lost: MessageWriter<KeyboardFocusLost>,
    mut keys: MessageWriter<KeyboardInput>,
    mut window_events: MessageWriter<WindowEvent>,
    mut pressed: Query<&mut PressedKeys>,
    mut trace: ResMut<InputTrace>,
) {
    let mut gone = Vec::new();
    let mut gained = false;
    for e in focus.read() {
        if e.focused {
            gained = true;
        } else {
            gone.push(e.window);
        }
    }
    if gained || gone.is_empty() {
        return;
    }
    window_events.write(WindowEvent::KeyboardFocusLost(KeyboardFocusLost));
    lost.write(KeyboardFocusLost);
    trace.line(format_args!("KeyboardFocusLost"));
    for window in gone {
        let Ok(mut held) = pressed.get_mut(window) else {
            continue;
        };
        for (key_code, logical_key) in held.0.drain() {
            let event = KeyboardInput {
                key_code,
                logical_key,
                state: ButtonState::Released,
                repeat: false,
                window,
                text: None,
            };
            trace.line(format_args!("{event:?}"));
            window_events.write(WindowEvent::KeyboardInput(event.clone()));
            keys.write(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    fn image(w: u32, h: u32) -> Image {
        let data = (0..w * h).flat_map(|i| [i as u8, 0, 0, 255]).collect();
        Image::new(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    #[test]
    fn a_cursor_is_the_rect_it_names_flipped_as_asked() {
        let img = image(4, 2);
        let (w, h, px) = cursor_pixels(&img, None, false, false).unwrap();
        assert_eq!((w, h), (4, 2));
        assert_eq!(
            px.iter().step_by(4).copied().collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 5, 6, 7]
        );
        let (w, h, px) = cursor_pixels(&img, Some(URect::new(1, 0, 3, 2)), true, true).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(
            px.iter().step_by(4).copied().collect::<Vec<_>>(),
            [6, 5, 2, 1]
        );
        assert_eq!(hotspot((0, 0), 2, 2, true, true), (1, 1));
    }

    #[test]
    fn the_swap_interval_follows_the_present_mode() {
        assert_eq!(swap_interval(PresentMode::AutoVsync), 1);
        assert_eq!(swap_interval(PresentMode::Fifo), 1);
        assert_eq!(swap_interval(PresentMode::AutoNoVsync), 0);
        assert_eq!(swap_interval(PresentMode::Immediate), 0);
    }
}
