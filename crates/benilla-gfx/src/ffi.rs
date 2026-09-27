//! The gfx DLL's C surface (`gfx_dll/gfx/src/gfx_dll.h`), ported from
//! `wc_clean_new_rs/src/gfx_interop.rs`. Values the library hands back (event types, keys,
//! buttons) are read as raw integers and decoded on the Rust side, so a value this port does not
//! know is dropped rather than being an invalid enum.

#![allow(non_camel_case_types, dead_code, clippy::missing_safety_doc)]

use std::os::raw::{c_char, c_int, c_void};

pub type GfxWindow = *mut c_void;
pub type GfxDevice = *mut c_void;
pub type GfxShader = *mut c_void;
pub type GfxShaderState = *mut c_void;
pub type GfxBuffer = *mut c_void;
pub type GfxTexture = *mut c_void;
pub type GfxInputLayout = *mut c_void;
pub type GfxAttributesState = *mut c_void;
pub type GfxBlendState = *mut c_void;
pub type GfxDepthStencilState = *mut c_void;
pub type GfxRasterizerState = *mut c_void;
pub type GfxPipeline = *mut c_void;
pub type GfxFramebuffer = *mut c_void;
pub type GfxConstantState = *mut c_void;
pub type GfxSamplerState = *mut c_void;
pub type GfxCursor = *mut c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GfxDeviceBackend {
    Gl3 = 0,
    Gl4 = 1,
    Gles3 = 2,
    Vk = 3,
    D3d9 = 4,
    D3d11 = 5,
    D3d12 = 6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GfxWindowBackend {
    X11 = 0,
    Win32 = 1,
    Wayland = 2,
    Glfw = 3,
    Sdl = 4,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxFormat {
    Unknown,
    R8Uint,
    R8Sint,
    R8Unorm,
    R8Snorm,
    R8G8Uint,
    R8G8Sint,
    R8G8Unorm,
    R8G8Snorm,
    R8G8B8A8Uint,
    R8G8B8A8Sint,
    R8G8B8A8Unorm,
    R8G8B8A8Snorm,
    R16Uint,
    R16Sint,
    R16Unorm,
    R16Snorm,
    R16Sfloat,
    R16G16Uint,
    R16G16Sint,
    R16G16Unorm,
    R16G16Snorm,
    R16G16Sfloat,
    R16G16B16A16Uint,
    R16G16B16A16Sint,
    R16G16B16A16Unorm,
    R16G16B16A16Snorm,
    R16G16B16A16Sfloat,
    R32Uint,
    R32Sint,
    R32Sfloat,
    R32G32Uint,
    R32G32Sint,
    R32G32Sfloat,
    R32G32B32Uint,
    R32G32B32Sint,
    R32G32B32Sfloat,
    R32G32B32A32Uint,
    R32G32B32A32Sint,
    R32G32B32A32Sfloat,
    D24UnormS8Uint,
    D16Unorm,
    D32Sfloat,
    R4G4B4A4UnormPack16,
    R5G5B5A1UnormPack16,
    R5G6B5UnormPack16,
    Bc1RgbUnormBlock,
    Bc1RgbaUnormBlock,
    Bc2UnormBlock,
    Bc3UnormBlock,
    Bc4UnormBlock,
    Bc4SnormBlock,
    Bc5UnormBlock,
    Bc5SnormBlock,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxBufferType {
    Vertexes,
    Indices,
    Uniform,
    Indirect,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxBufferUsage {
    Immutable,
    Static,
    Dynamic,
    Stream,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxTextureType {
    Texture2D,
    Texture2DMs,
    Texture2DShadow,
    Texture2DArray,
    Texture2DArrayMs,
    Texture3D,
}

/// `enum gfx_texture_usage` bits, combined into [`GfxTextureCreateInfo::usage`].
pub mod texture_usage {
    pub const RENDER_TARGET: u32 = 0x1;
    pub const SAMPLED: u32 = 0x2;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxShaderType {
    Vertex = 0,
    Fragment = 1,
    Geometry = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxBlendFunction {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxBlendEquation {
    Add,
    Subtract,
    RevSubtract,
    Min,
    Max,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxCompareFunction {
    Never,
    Lower,
    LEqual,
    Equal,
    GEqual,
    Greater,
    NotEqual,
    Always,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxStencilOperation {
    Keep,
    Zero,
    Replace,
    Inc,
    IncWrap,
    Dec,
    DecWrap,
    Inv,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxFiltering {
    None,
    Nearest,
    Linear,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxTextureAddressing {
    Clamp,
    Repeat,
    Mirror,
    Border,
    MirrorOnce,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxFillMode {
    Point,
    Line,
    Solid,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxCullMode {
    None,
    Front,
    Back,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxFrontFace {
    Cw,
    Ccw,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxPrimitiveType {
    Triangles,
    TriangleStrip,
    Points,
    Lines,
    LineStrip,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxIndexType {
    UInt16,
    UInt32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GfxStepMode {
    Vertex,
    Instance,
}

/// `enum gfx_buffer_bit`, the `buffers` mask of `gfx_dll_resolve_framebuffer`.
pub mod buffer_bit {
    pub const COLOR: u32 = 0x1;
    pub const DEPTH: u32 = 0x2;
    pub const STENCIL: u32 = 0x4;
}

/// The blend state's `color_mask` bits.
pub mod color_mask {
    pub const NONE: u32 = 0x0;
    pub const R: u32 = 0x1;
    pub const G: u32 = 0x2;
    pub const B: u32 = 0x4;
    pub const A: u32 = 0x8;
    pub const RGBA: u32 = 0xF;
}

/// `enum gfx_key_mods` bits, as carried by the key and mouse events.
pub mod key_mods {
    pub const CONTROL: u32 = 0x1;
    pub const SHIFT: u32 = 0x2;
    pub const ALT: u32 = 0x4;
    pub const SUPER: u32 = 0x8;
    pub const CAPS_LOCK: u32 = 0x10;
    pub const NUM_LOCK: u32 = 0x20;
}

/// `enum gfx_key_code` (`events.h`), read raw off [`GfxKeyEvent::key`] and
/// [`GfxKeyEvent::physical`].
pub mod key {
    pub const UNKNOWN: u32 = 0;
    pub const A: u32 = 1;
    pub const B: u32 = 2;
    pub const C: u32 = 3;
    pub const D: u32 = 4;
    pub const E: u32 = 5;
    pub const F: u32 = 6;
    pub const G: u32 = 7;
    pub const H: u32 = 8;
    pub const I: u32 = 9;
    pub const J: u32 = 10;
    pub const K: u32 = 11;
    pub const L: u32 = 12;
    pub const M: u32 = 13;
    pub const N: u32 = 14;
    pub const O: u32 = 15;
    pub const P: u32 = 16;
    pub const Q: u32 = 17;
    pub const R: u32 = 18;
    pub const S: u32 = 19;
    pub const T: u32 = 20;
    pub const U: u32 = 21;
    pub const V: u32 = 22;
    pub const W: u32 = 23;
    pub const X: u32 = 24;
    pub const Y: u32 = 25;
    pub const Z: u32 = 26;
    pub const _0: u32 = 27;
    pub const _1: u32 = 28;
    pub const _2: u32 = 29;
    pub const _3: u32 = 30;
    pub const _4: u32 = 31;
    pub const _5: u32 = 32;
    pub const _6: u32 = 33;
    pub const _7: u32 = 34;
    pub const _8: u32 = 35;
    pub const _9: u32 = 36;
    pub const KP_0: u32 = 37;
    pub const KP_1: u32 = 38;
    pub const KP_2: u32 = 39;
    pub const KP_3: u32 = 40;
    pub const KP_4: u32 = 41;
    pub const KP_5: u32 = 42;
    pub const KP_6: u32 = 43;
    pub const KP_7: u32 = 44;
    pub const KP_8: u32 = 45;
    pub const KP_9: u32 = 46;
    pub const KP_DIVIDE: u32 = 47;
    pub const KP_MULTIPLY: u32 = 48;
    pub const KP_SUBTRACT: u32 = 49;
    pub const KP_ADD: u32 = 50;
    pub const KP_EQUAL: u32 = 51;
    pub const KP_DECIMAL: u32 = 52;
    pub const KP_ENTER: u32 = 53;
    pub const F1: u32 = 54;
    pub const F2: u32 = 55;
    pub const F3: u32 = 56;
    pub const F4: u32 = 57;
    pub const F5: u32 = 58;
    pub const F6: u32 = 59;
    pub const F7: u32 = 60;
    pub const F8: u32 = 61;
    pub const F9: u32 = 62;
    pub const F10: u32 = 63;
    pub const F11: u32 = 64;
    pub const F12: u32 = 65;
    pub const F13: u32 = 66;
    pub const F14: u32 = 67;
    pub const F15: u32 = 68;
    pub const F16: u32 = 69;
    pub const F17: u32 = 70;
    pub const F18: u32 = 71;
    pub const F19: u32 = 72;
    pub const F20: u32 = 73;
    pub const F21: u32 = 74;
    pub const F22: u32 = 75;
    pub const F23: u32 = 76;
    pub const F24: u32 = 77;
    pub const LSHIFT: u32 = 78;
    pub const RSHIFT: u32 = 79;
    pub const LCONTROL: u32 = 80;
    pub const RCONTROL: u32 = 81;
    pub const LALT: u32 = 82;
    pub const RALT: u32 = 83;
    pub const LSUPER: u32 = 84;
    pub const RSUPER: u32 = 85;
    pub const LEFT: u32 = 86;
    pub const RIGHT: u32 = 87;
    pub const UP: u32 = 88;
    pub const DOWN: u32 = 89;
    pub const SPACE: u32 = 90;
    pub const BACKSPACE: u32 = 91;
    pub const ENTER: u32 = 92;
    pub const TAB: u32 = 93;
    pub const ESCAPE: u32 = 94;
    pub const PAUSE: u32 = 95;
    pub const DELETE: u32 = 96;
    pub const INSERT: u32 = 97;
    pub const HOME: u32 = 98;
    pub const PAGE_UP: u32 = 99;
    pub const PAGE_DOWN: u32 = 100;
    pub const END: u32 = 101;
    pub const COMMA: u32 = 102;
    pub const PERIOD: u32 = 103;
    pub const SLASH: u32 = 104;
    pub const APOSTROPHE: u32 = 105;
    pub const SEMICOLON: u32 = 106;
    pub const GRAVE: u32 = 107;
    pub const LBRACKET: u32 = 108;
    pub const RBRACKET: u32 = 109;
    pub const BACKSLASH: u32 = 110;
    pub const EQUAL: u32 = 111;
    pub const SUBTRACT: u32 = 112;
    pub const SCROLL_LOCK: u32 = 113;
    pub const NUM_LOCK: u32 = 114;
    pub const CAPS_LOCK: u32 = 115;
    pub const PRINT: u32 = 116;
    pub const LAST: u32 = 117;
}

/// `enum gfx_mouse_button`, read raw off [`GfxMouseEvent::button`].
pub mod mouse_button {
    pub const LEFT: u32 = 0;
    pub const RIGHT: u32 = 1;
    pub const MIDDLE: u32 = 2;
    /// The first of the extra buttons: X11 button 8, `XBUTTON1`, SDL's X1 (back).
    pub const BUTTON_4: u32 = 3;
    pub const LAST: u32 = 8;
}

/// `enum gfx_native_cursor` (`window.h`), for [`gfx_dll_create_native_cursor`].
pub mod native_cursor {
    pub const ARROW: u32 = 0;
    pub const CROSS: u32 = 1;
    pub const HAND: u32 = 2;
    pub const IBEAM: u32 = 3;
    pub const NO: u32 = 4;
    pub const SIZEALL: u32 = 5;
    pub const VRESIZE: u32 = 6;
    pub const HRESIZE: u32 = 7;
    pub const WAIT: u32 = 8;
    pub const BLANK: u32 = 9;
    pub const LAST: u32 = 10;
}

/// `enum gfx_event_type`, read raw off [`GfxEvent::event_type`].
pub mod event_type {
    pub const KEY_DOWN: u32 = 0;
    pub const KEY_PRESS: u32 = 1;
    pub const KEY_UP: u32 = 2;
    pub const CHARACTER: u32 = 3;
    pub const MOUSE_DOWN: u32 = 4;
    pub const MOUSE_UP: u32 = 5;
    pub const MOUSE_MOVE: u32 = 6;
    pub const SCROLL: u32 = 7;
    pub const CURSOR_ENTER: u32 = 8;
    pub const CURSOR_LEAVE: u32 = 9;
    pub const FOCUS_IN: u32 = 10;
    pub const FOCUS_OUT: u32 = 11;
    pub const RESIZE: u32 = 12;
    pub const MOVE: u32 = 13;
    pub const EXPOSE: u32 = 14;
    pub const CLOSE: u32 = 15;
    /// `gfx_benilla`: the device moved by [`super::GfxMotionEvent`], with focus.
    pub const RAW_MOTION: u32 = 16;
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxWindowProperties {
    pub window_backend: GfxWindowBackend,
    pub device_backend: GfxDeviceBackend,
    pub color_format: GfxFormat,
    pub depth_stencil_format: GfxFormat,
    pub samples: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxShaderCreateInfo {
    pub shader_type: GfxShaderType,
    pub data: *const c_void,
    pub size: u32,
}

/// `gfx_dll_shader_attribute`, `_constant` and `_sampler` share this shape: a name and a bind.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxShaderBinding {
    pub name: *const c_char,
    pub bind: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxShaderStateCreateInfo {
    pub vertex_shader: GfxShader,
    pub fragment_shader: GfxShader,
    pub geometry_shader: GfxShader,
    pub attributes: *const GfxShaderBinding,
    pub constants: *const GfxShaderBinding,
    pub samplers: *const GfxShaderBinding,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxBufferCreateInfo {
    pub buffer_type: GfxBufferType,
    pub usage: GfxBufferUsage,
    pub data: *const c_void,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxTextureCreateInfo {
    pub texture_type: GfxTextureType,
    /// [`texture_usage`] bits.
    pub usage: u32,
    pub format: GfxFormat,
    pub levels: u8,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub addressing_s: GfxTextureAddressing,
    pub addressing_t: GfxTextureAddressing,
    pub addressing_r: GfxTextureAddressing,
    pub min_filtering: GfxFiltering,
    pub mag_filtering: GfxFiltering,
    pub mip_filtering: GfxFiltering,
    pub anisotropy: u32,
    pub border_color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxInputLayoutBind {
    pub buffer: u32,
    pub format: GfxFormat,
    pub stride: u32,
    pub offset: u32,
    pub step_mode: GfxStepMode,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxInputLayoutCreateInfo {
    pub shader_state: GfxShaderState,
    pub binds: *const GfxInputLayoutBind,
    pub count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxAttributeBind {
    pub buffer: GfxBuffer,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxAttributesStateCreateInfo {
    pub binds: *const GfxAttributeBind,
    pub count: u32,
    pub index_buffer: GfxBuffer,
    pub index_type: GfxIndexType,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxBlendStateCreateInfo {
    pub enabled: bool,
    pub src_color: GfxBlendFunction,
    pub src_alpha: GfxBlendFunction,
    pub dst_color: GfxBlendFunction,
    pub dst_alpha: GfxBlendFunction,
    pub equation_color: GfxBlendEquation,
    pub equation_alpha: GfxBlendEquation,
    pub color_mask: u32,
    pub constant_color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxStencilState {
    pub fail: GfxStencilOperation,
    pub pass: GfxStencilOperation,
    pub zfail: GfxStencilOperation,
    pub compare_func: GfxCompareFunction,
    pub compare_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxDepthStencilStateCreateInfo {
    pub depth_compare: GfxCompareFunction,
    pub depth_write: bool,
    pub depth_test: bool,
    pub stencil_enabled: bool,
    pub front: GfxStencilState,
    pub back: GfxStencilState,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxRasterizerStateCreateInfo {
    pub fill_mode: GfxFillMode,
    pub cull_mode: GfxCullMode,
    pub front_face: GfxFrontFace,
    pub scissor: bool,
    pub depth_clamp: bool,
    pub multisample: bool,
}

/// With `target_framebuffer`: the library is built with `GFX_ENABLE_TARGET_FRAMEBUFFER`, its
/// CMake default, which grows the C struct by that field.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxPipelineCreateInfo {
    pub shader_state: GfxShaderState,
    pub rasterizer_state: GfxRasterizerState,
    pub depth_stencil_state: GfxDepthStencilState,
    pub blend_state: GfxBlendState,
    pub input_layout: GfxInputLayout,
    pub primitive: GfxPrimitiveType,
    pub target_framebuffer: GfxFramebuffer,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxFramebufferCreateInfo {
    pub color_attachments: *mut GfxTexture,
    pub depth_stencil_attachment: GfxTexture,
    pub color_count: u32,
    pub msaa_samples: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxConstantDescriptor {
    pub buffer: GfxBuffer,
    pub offset: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxConstantStateCreateInfo {
    pub shader_state: GfxShaderState,
    pub descriptors: *mut GfxConstantDescriptor,
    pub count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxSamplerDescriptor {
    pub texture: GfxTexture,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxSamplerStateCreateInfo {
    pub shader_state: GfxShaderState,
    pub descriptors: *mut GfxSamplerDescriptor,
    pub count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GfxClearColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxKeyEvent {
    pub used: bool,
    /// [`key`]: what the keymap makes of the key, falling back to [`Self::physical`].
    pub key: u32,
    pub mods: u32,
    /// [`key`]: the key at this place on a US keyboard, whatever the keymap says.
    pub physical: u32,
    /// The platform's own code: the X11 keycode, the Win32 scancode, the SDL scancode.
    pub scancode: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxCharEvent {
    pub used: bool,
    pub codepoint: u32,
    pub utf8: [u8; 5],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxMouseEvent {
    pub used: bool,
    pub x: i32,
    pub y: i32,
    /// `enum gfx_mouse_button`.
    pub button: u32,
    pub mods: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxScrollEvent {
    pub used: bool,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxPointerEvent {
    pub used: bool,
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxResizeEvent {
    pub used: bool,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxMoveEvent {
    pub used: bool,
    pub x: i32,
    pub y: i32,
}

/// `gfx_benilla`'s raw motion: the device's own movement, not the cursor's position.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfxMotionEvent {
    pub used: bool,
    pub dx: f32,
    pub dy: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union GfxEventData {
    pub key: GfxKeyEvent,
    pub character: GfxCharEvent,
    pub mouse: GfxMouseEvent,
    pub scroll: GfxScrollEvent,
    pub pointer: GfxPointerEvent,
    pub resize: GfxResizeEvent,
    pub mov: GfxMoveEvent,
    pub motion: GfxMotionEvent,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GfxEvent {
    pub window: GfxWindow,
    /// [`event_type`], raw.
    pub event_type: u32,
    pub data: GfxEventData,
}

pub type GfxErrorCallback = unsafe extern "C" fn(*const c_char);
pub type GfxEventCallback = unsafe extern "C" fn(*mut GfxEvent);

#[link(name = "gfx")]
unsafe extern "C" {
    pub fn gfx_dll_set_error_callback(callback: GfxErrorCallback);

    pub fn gfx_dll_has_window_backend(backend: GfxWindowBackend) -> bool;
    pub fn gfx_dll_has_device_backend(backend: GfxDeviceBackend) -> bool;
    pub fn gfx_dll_device_backend_name(backend: GfxDeviceBackend) -> *const c_char;
    pub fn gfx_dll_window_backend_name(backend: GfxWindowBackend) -> *const c_char;

    pub fn gfx_dll_window_properties_init(properties: *mut GfxWindowProperties);
    pub fn gfx_dll_create_window(
        title: *const c_char,
        width: u32,
        height: u32,
        properties: *const GfxWindowProperties,
    ) -> GfxWindow;
    pub fn gfx_dll_delete_window(window: GfxWindow);
    pub fn gfx_dll_get_device(window: GfxWindow) -> GfxDevice;
    pub fn gfx_dll_window_show(window: GfxWindow);
    pub fn gfx_dll_window_hide(window: GfxWindow);
    pub fn gfx_dll_window_poll_events(window: GfxWindow);
    pub fn gfx_dll_window_wait_events(window: GfxWindow);
    pub fn gfx_dll_swap_buffers(window: GfxWindow);
    pub fn gfx_dll_make_current(window: GfxWindow);
    pub fn gfx_dll_set_swap_interval(window: GfxWindow, interval: c_int);
    pub fn gfx_dll_window_set_title(window: GfxWindow, title: *const c_char);
    pub fn gfx_dll_window_resize(window: GfxWindow, width: u32, height: u32);
    pub fn gfx_dll_window_get_width(window: GfxWindow) -> u32;
    pub fn gfx_dll_window_get_height(window: GfxWindow) -> u32;
    pub fn gfx_dll_window_is_close_requested(window: GfxWindow) -> bool;
    pub fn gfx_dll_window_set_close_requested(window: GfxWindow, close_requested: bool);
    pub fn gfx_dll_window_set_event_handler(window: GfxWindow, handler: GfxEventCallback);
    pub fn gfx_dll_is_key_down(window: GfxWindow, key: u32) -> bool;
    pub fn gfx_dll_is_mouse_button_down(window: GfxWindow, button: u32) -> bool;
    pub fn gfx_dll_grab_cursor(window: GfxWindow);
    pub fn gfx_dll_ungrab_cursor(window: GfxWindow);
    pub fn gfx_dll_get_mouse_x(window: GfxWindow) -> i32;
    pub fn gfx_dll_get_mouse_y(window: GfxWindow) -> i32;
    pub fn gfx_dll_window_get_scale_factor(window: GfxWindow) -> f32;
    pub fn gfx_dll_window_set_icon(window: GfxWindow, data: *const c_void, width: u32, height: u32);
    /// `cursor`: a [`native_cursor`] value.
    pub fn gfx_dll_create_native_cursor(window: GfxWindow, cursor: u32) -> GfxCursor;
    /// RGBA8, top row first.
    pub fn gfx_dll_create_cursor(
        window: GfxWindow,
        data: *const c_void,
        width: u32,
        height: u32,
        xhot: u32,
        yhot: u32,
    ) -> GfxCursor;
    pub fn gfx_dll_delete_cursor(window: GfxWindow, cursor: GfxCursor);
    /// A null cursor is the system's default pointer.
    pub fn gfx_dll_set_cursor(window: GfxWindow, cursor: GfxCursor);
    /// Client-area pixels.
    pub fn gfx_dll_set_mouse_position(window: GfxWindow, x: i32, y: i32);

    pub fn gfx_dll_get_uniform_buffer_size(device: GfxDevice, buffer_size: u32) -> u32;

    pub fn gfx_dll_create_shader(
        device: GfxDevice,
        create_info: *const GfxShaderCreateInfo,
        shaderp: *mut GfxShader,
    ) -> bool;
    pub fn gfx_dll_delete_shader(device: GfxDevice, shader: GfxShader);
    pub fn gfx_dll_create_shader_state(
        device: GfxDevice,
        create_info: *const GfxShaderStateCreateInfo,
        statep: *mut GfxShaderState,
    ) -> bool;
    pub fn gfx_dll_delete_shader_state(device: GfxDevice, state: GfxShaderState);

    pub fn gfx_dll_create_buffer(
        device: GfxDevice,
        create_info: *const GfxBufferCreateInfo,
        bufferp: *mut GfxBuffer,
    ) -> bool;
    pub fn gfx_dll_set_buffer_data(
        device: GfxDevice,
        buffer: GfxBuffer,
        data: *const c_void,
        size: u32,
        offset: u32,
    ) -> bool;
    pub fn gfx_dll_delete_buffer(device: GfxDevice, buffer: GfxBuffer);

    pub fn gfx_dll_create_texture(
        device: GfxDevice,
        create_info: *const GfxTextureCreateInfo,
        texturep: *mut GfxTexture,
    ) -> bool;
    pub fn gfx_dll_set_texture_data(
        device: GfxDevice,
        texture: GfxTexture,
        level: u8,
        offset: u32,
        width: u32,
        height: u32,
        depth: u32,
        size: u32,
        data: *const c_void,
    ) -> bool;
    pub fn gfx_dll_delete_texture(device: GfxDevice, texture: GfxTexture);

    pub fn gfx_dll_create_input_layout(
        device: GfxDevice,
        create_info: *const GfxInputLayoutCreateInfo,
        input_layoutp: *mut GfxInputLayout,
    ) -> bool;
    pub fn gfx_dll_delete_input_layout(device: GfxDevice, input_layout: GfxInputLayout);

    pub fn gfx_dll_create_attributes_state(
        device: GfxDevice,
        create_info: *const GfxAttributesStateCreateInfo,
        statep: *mut GfxAttributesState,
    ) -> bool;
    pub fn gfx_dll_bind_attributes_state(
        device: GfxDevice,
        state: GfxAttributesState,
        input_layout: GfxInputLayout,
    );
    pub fn gfx_dll_delete_attributes_state(device: GfxDevice, state: GfxAttributesState);

    pub fn gfx_dll_create_blend_state(
        device: GfxDevice,
        create_info: *const GfxBlendStateCreateInfo,
        statep: *mut GfxBlendState,
    ) -> bool;
    pub fn gfx_dll_delete_blend_state(device: GfxDevice, state: GfxBlendState);

    pub fn gfx_dll_create_depth_stencil_state(
        device: GfxDevice,
        create_info: *const GfxDepthStencilStateCreateInfo,
        statep: *mut GfxDepthStencilState,
    ) -> bool;
    pub fn gfx_dll_delete_depth_stencil_state(device: GfxDevice, state: GfxDepthStencilState);

    pub fn gfx_dll_create_rasterizer_state(
        device: GfxDevice,
        create_info: *const GfxRasterizerStateCreateInfo,
        statep: *mut GfxRasterizerState,
    ) -> bool;
    pub fn gfx_dll_delete_rasterizer_state(device: GfxDevice, state: GfxRasterizerState);

    pub fn gfx_dll_create_pipeline(
        device: GfxDevice,
        create_info: *const GfxPipelineCreateInfo,
        pipelinep: *mut GfxPipeline,
    ) -> bool;
    pub fn gfx_dll_delete_pipeline(device: GfxDevice, pipeline: GfxPipeline);
    pub fn gfx_dll_bind_pipeline(device: GfxDevice, pipeline: GfxPipeline);

    pub fn gfx_dll_create_framebuffer(
        device: GfxDevice,
        create_info: *const GfxFramebufferCreateInfo,
        framebufferp: *mut GfxFramebuffer,
    ) -> bool;
    pub fn gfx_dll_delete_framebuffer(device: GfxDevice, framebuffer: GfxFramebuffer);
    pub fn gfx_dll_bind_framebuffer(device: GfxDevice, framebuffer: GfxFramebuffer);
    pub fn gfx_dll_resolve_framebuffer(
        device: GfxDevice,
        src: GfxFramebuffer,
        dst: GfxFramebuffer,
        buffers: u32,
        src_color: u32,
        dst_color: u32,
    ) -> bool;

    pub fn gfx_dll_bind_constant(
        device: GfxDevice,
        bind: u32,
        buffer: GfxBuffer,
        size: u32,
        offset: u32,
    );
    pub fn gfx_dll_bind_samplers(
        device: GfxDevice,
        start: u32,
        count: u32,
        textures: *mut GfxTexture,
    );

    pub fn gfx_dll_create_constant_state(
        device: GfxDevice,
        create_info: *const GfxConstantStateCreateInfo,
        statep: *mut GfxConstantState,
    ) -> bool;
    pub fn gfx_dll_bind_constant_state(device: GfxDevice, state: GfxConstantState);
    pub fn gfx_dll_update_constant_state(
        device: GfxDevice,
        state: GfxConstantState,
        id: u32,
        descriptor: *const GfxConstantDescriptor,
    );
    pub fn gfx_dll_delete_constant_state(device: GfxDevice, state: GfxConstantState);

    pub fn gfx_dll_create_sampler_state(
        device: GfxDevice,
        create_info: *const GfxSamplerStateCreateInfo,
        statep: *mut GfxSamplerState,
    ) -> bool;
    pub fn gfx_dll_bind_sampler_state(device: GfxDevice, state: GfxSamplerState);
    pub fn gfx_dll_update_sampler_state(
        device: GfxDevice,
        state: GfxSamplerState,
        id: u32,
        descriptor: *const GfxSamplerDescriptor,
    );
    pub fn gfx_dll_delete_sampler_state(device: GfxDevice, state: GfxSamplerState);

    pub fn gfx_dll_set_viewport(
        device: GfxDevice,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        depth_min: f32,
        depth_max: f32,
    );
    pub fn gfx_dll_set_scissor(device: GfxDevice, x: i32, y: i32, width: u32, height: u32);

    pub fn gfx_dll_clear_color(
        device: GfxDevice,
        framebuffer: GfxFramebuffer,
        attachment: u32,
        color: *const GfxClearColor,
    );
    pub fn gfx_dll_clear_depth_stencil(
        device: GfxDevice,
        framebuffer: GfxFramebuffer,
        depth: f32,
        stencil: u8,
    );

    pub fn gfx_dll_draw(device: GfxDevice, count: u32, offset: u32);
    pub fn gfx_dll_draw_indexed(device: GfxDevice, count: u32, offset: u32);
    pub fn gfx_dll_draw_instanced(device: GfxDevice, count: u32, offset: u32, prim_count: u32);
    pub fn gfx_dll_draw_indexed_instanced(
        device: GfxDevice,
        count: u32,
        offset: u32,
        prim_count: u32,
    );

    pub fn gfx_dll_set_line_width(device: GfxDevice, line_width: f32);
    pub fn gfx_dll_set_point_size(device: GfxDevice, point_size: f32);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `gfx_dll_event` as the library builds it (`gfx_dll_print_struct_sizes` on x86_64: 32 bytes,
    /// the union at 12): `gfx_benilla`'s key and motion events fit the union's 20 bytes.
    #[test]
    fn the_event_matches_the_c_layout() {
        assert_eq!(std::mem::size_of::<GfxKeyEvent>(), 20);
        assert_eq!(std::mem::size_of::<GfxMotionEvent>(), 12);
        assert_eq!(std::mem::size_of::<GfxEventData>(), 20);
        assert_eq!(std::mem::offset_of!(GfxEvent, data), 12);
        assert_eq!(std::mem::size_of::<GfxEvent>(), 32);
    }
}
