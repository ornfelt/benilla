//! gfx's key, character and button codes to Bevy's, as bevy_winit's `converters.rs` turns winit's:
//! [`KeyCode`] is where the key sits (the gfx key's `physical` code), [`Key`] what it means (the
//! keymap's key and the text it typed).

use bevy::input::keyboard::{Key, NativeKeyCode};
use bevy::input::mouse::MouseButton;
use bevy::prelude::KeyCode;

use crate::ffi::{key, mouse_button};

/// The position a gfx key names, as a [`KeyCode`]; `None` for `GFX_KEY_UNKNOWN` or a value past
/// the enum.
pub fn key_code(gfx: u32) -> Option<KeyCode> {
    use KeyCode as K;
    Some(match gfx {
        key::A => K::KeyA,
        key::B => K::KeyB,
        key::C => K::KeyC,
        key::D => K::KeyD,
        key::E => K::KeyE,
        key::F => K::KeyF,
        key::G => K::KeyG,
        key::H => K::KeyH,
        key::I => K::KeyI,
        key::J => K::KeyJ,
        key::K => K::KeyK,
        key::L => K::KeyL,
        key::M => K::KeyM,
        key::N => K::KeyN,
        key::O => K::KeyO,
        key::P => K::KeyP,
        key::Q => K::KeyQ,
        key::R => K::KeyR,
        key::S => K::KeyS,
        key::T => K::KeyT,
        key::U => K::KeyU,
        key::V => K::KeyV,
        key::W => K::KeyW,
        key::X => K::KeyX,
        key::Y => K::KeyY,
        key::Z => K::KeyZ,
        key::_0 => K::Digit0,
        key::_1 => K::Digit1,
        key::_2 => K::Digit2,
        key::_3 => K::Digit3,
        key::_4 => K::Digit4,
        key::_5 => K::Digit5,
        key::_6 => K::Digit6,
        key::_7 => K::Digit7,
        key::_8 => K::Digit8,
        key::_9 => K::Digit9,
        key::KP_0 => K::Numpad0,
        key::KP_1 => K::Numpad1,
        key::KP_2 => K::Numpad2,
        key::KP_3 => K::Numpad3,
        key::KP_4 => K::Numpad4,
        key::KP_5 => K::Numpad5,
        key::KP_6 => K::Numpad6,
        key::KP_7 => K::Numpad7,
        key::KP_8 => K::Numpad8,
        key::KP_9 => K::Numpad9,
        key::KP_DIVIDE => K::NumpadDivide,
        key::KP_MULTIPLY => K::NumpadMultiply,
        key::KP_SUBTRACT => K::NumpadSubtract,
        key::KP_ADD => K::NumpadAdd,
        key::KP_EQUAL => K::NumpadEqual,
        key::KP_DECIMAL => K::NumpadDecimal,
        key::KP_ENTER => K::NumpadEnter,
        key::F1 => K::F1,
        key::F2 => K::F2,
        key::F3 => K::F3,
        key::F4 => K::F4,
        key::F5 => K::F5,
        key::F6 => K::F6,
        key::F7 => K::F7,
        key::F8 => K::F8,
        key::F9 => K::F9,
        key::F10 => K::F10,
        key::F11 => K::F11,
        key::F12 => K::F12,
        key::F13 => K::F13,
        key::F14 => K::F14,
        key::F15 => K::F15,
        key::F16 => K::F16,
        key::F17 => K::F17,
        key::F18 => K::F18,
        key::F19 => K::F19,
        key::F20 => K::F20,
        key::F21 => K::F21,
        key::F22 => K::F22,
        key::F23 => K::F23,
        key::F24 => K::F24,
        key::LSHIFT => K::ShiftLeft,
        key::RSHIFT => K::ShiftRight,
        key::LCONTROL => K::ControlLeft,
        key::RCONTROL => K::ControlRight,
        key::LALT => K::AltLeft,
        key::RALT => K::AltRight,
        key::LSUPER => K::SuperLeft,
        key::RSUPER => K::SuperRight,
        key::LEFT => K::ArrowLeft,
        key::RIGHT => K::ArrowRight,
        key::UP => K::ArrowUp,
        key::DOWN => K::ArrowDown,
        key::SPACE => K::Space,
        key::BACKSPACE => K::Backspace,
        key::ENTER => K::Enter,
        key::TAB => K::Tab,
        key::ESCAPE => K::Escape,
        key::PAUSE => K::Pause,
        key::DELETE => K::Delete,
        key::INSERT => K::Insert,
        key::HOME => K::Home,
        key::PAGE_UP => K::PageUp,
        key::PAGE_DOWN => K::PageDown,
        key::END => K::End,
        key::COMMA => K::Comma,
        key::PERIOD => K::Period,
        key::SLASH => K::Slash,
        key::APOSTROPHE => K::Quote,
        key::SEMICOLON => K::Semicolon,
        key::GRAVE => K::Backquote,
        key::LBRACKET => K::BracketLeft,
        key::RBRACKET => K::BracketRight,
        key::BACKSLASH => K::Backslash,
        key::EQUAL => K::Equal,
        key::SUBTRACT => K::Minus,
        key::SCROLL_LOCK => K::ScrollLock,
        key::NUM_LOCK => K::NumLock,
        key::CAPS_LOCK => K::CapsLock,
        key::PRINT => K::PrintScreen,
        _ => return None,
    })
}

/// Which platform family a window backend's scancodes belong to, for a key gfx has no name for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scancodes {
    /// X11 keycodes (the x11 backend, and glfw on X11): winit's `NativeKeyCode::Xkb`.
    Xkb,
    /// Win32 scancodes with the extended bit at 0x100 (win32, and glfw on Windows).
    Windows,
    /// SDL's USB HID usages, which winit has no native form for.
    Sdl,
}

/// The [`KeyCode`] of a key event: its position, else the keymap's key read as one, else the
/// platform code the way winit leaves an unknown key (`PhysicalKey::Unidentified`).
pub fn physical_key(physical: u32, key: u32, scancode: u32, family: Scancodes) -> KeyCode {
    key_code(physical)
        .or_else(|| key_code(key))
        .unwrap_or(KeyCode::Unidentified(match family {
            Scancodes::Xkb => NativeKeyCode::Xkb(scancode),
            Scancodes::Windows => NativeKeyCode::Windows(scancode as u16),
            Scancodes::Sdl => NativeKeyCode::Unidentified,
        }))
}

/// A key that means itself whatever it types, as winit names it (`NamedKey`, which bevy_winit
/// flattens into [`Key`]). The numpad's digits are named only when they type nothing (Num Lock
/// off), which [`logical_key`] decides.
fn named_key(gfx: u32) -> Option<Key> {
    Some(match gfx {
        key::ESCAPE => Key::Escape,
        key::ENTER | key::KP_ENTER => Key::Enter,
        key::TAB => Key::Tab,
        key::SPACE => Key::Space,
        key::BACKSPACE => Key::Backspace,
        key::DELETE => Key::Delete,
        key::INSERT => Key::Insert,
        key::HOME => Key::Home,
        key::END => Key::End,
        key::PAGE_UP => Key::PageUp,
        key::PAGE_DOWN => Key::PageDown,
        key::LEFT => Key::ArrowLeft,
        key::RIGHT => Key::ArrowRight,
        key::UP => Key::ArrowUp,
        key::DOWN => Key::ArrowDown,
        key::LSHIFT | key::RSHIFT => Key::Shift,
        key::LCONTROL | key::RCONTROL => Key::Control,
        key::LALT | key::RALT => Key::Alt,
        key::LSUPER | key::RSUPER => Key::Super,
        key::CAPS_LOCK => Key::CapsLock,
        key::NUM_LOCK => Key::NumLock,
        key::SCROLL_LOCK => Key::ScrollLock,
        key::PAUSE => Key::Pause,
        key::PRINT => Key::PrintScreen,
        key::F1 => Key::F1,
        key::F2 => Key::F2,
        key::F3 => Key::F3,
        key::F4 => Key::F4,
        key::F5 => Key::F5,
        key::F6 => Key::F6,
        key::F7 => Key::F7,
        key::F8 => Key::F8,
        key::F9 => Key::F9,
        key::F10 => Key::F10,
        key::F11 => Key::F11,
        key::F12 => Key::F12,
        key::F13 => Key::F13,
        key::F14 => Key::F14,
        key::F15 => Key::F15,
        key::F16 => Key::F16,
        key::F17 => Key::F17,
        key::F18 => Key::F18,
        key::F19 => Key::F19,
        key::F20 => Key::F20,
        key::F21 => Key::F21,
        key::F22 => Key::F22,
        key::F23 => Key::F23,
        key::F24 => Key::F24,
        _ => return None,
    })
}

/// The numpad key's meaning with Num Lock off: the navigation key printed under the digit.
fn numpad_navigation(gfx: u32) -> Option<Key> {
    Some(match gfx {
        key::KP_0 => Key::Insert,
        key::KP_1 => Key::End,
        key::KP_2 => Key::ArrowDown,
        key::KP_3 => Key::PageDown,
        key::KP_4 => Key::ArrowLeft,
        key::KP_5 => Key::Clear,
        key::KP_6 => Key::ArrowRight,
        key::KP_7 => Key::Home,
        key::KP_8 => Key::ArrowUp,
        key::KP_9 => Key::PageUp,
        key::KP_DECIMAL => Key::Delete,
        _ => return None,
    })
}

/// The character a gfx key types on a US keyboard, shifted or not, for a press whose own text is
/// missing or a control character (Ctrl+A types `\x01`; winit's logical key ignores Ctrl).
fn us_character(gfx: u32, shift: bool) -> Option<char> {
    let (plain, shifted) = match gfx {
        key::A..=key::Z => {
            let c = char::from(b'a' + (gfx - key::A) as u8);
            (c, c.to_ascii_uppercase())
        }
        key::_0..=key::_9 => {
            let d = (gfx - key::_0) as usize;
            (
                char::from(b'0' + d as u8),
                [')', '!', '@', '#', '$', '%', '^', '&', '*', '('][d],
            )
        }
        key::KP_0..=key::KP_9 => {
            let c = char::from(b'0' + (gfx - key::KP_0) as u8);
            (c, c)
        }
        key::KP_DIVIDE => ('/', '/'),
        key::KP_MULTIPLY => ('*', '*'),
        key::KP_SUBTRACT => ('-', '-'),
        key::KP_ADD => ('+', '+'),
        key::KP_EQUAL => ('=', '='),
        key::KP_DECIMAL => ('.', '.'),
        key::COMMA => (',', '<'),
        key::PERIOD => ('.', '>'),
        key::SLASH => ('/', '?'),
        key::APOSTROPHE => ('\'', '"'),
        key::SEMICOLON => (';', ':'),
        key::GRAVE => ('`', '~'),
        key::LBRACKET => ('[', '{'),
        key::RBRACKET => (']', '}'),
        key::BACKSLASH => ('\\', '|'),
        key::EQUAL => ('=', '+'),
        key::SUBTRACT => ('-', '_'),
        _ => return None,
    };
    Some(if shift { shifted } else { plain })
}

/// What a press means, as winit's `logical_key`: a named key by name; otherwise the text it typed
/// when that is printable, else the character the key would type with no Ctrl held (`shift` from
/// the event's modifiers). A numpad key that typed nothing is its navigation key.
pub fn logical_key(gfx: u32, text: Option<&str>, shift: bool) -> Key {
    if let Some(named) = named_key(gfx) {
        return named;
    }
    if let Some(text) = text.filter(|t| !t.is_empty() && !t.chars().any(char::is_control)) {
        return Key::Character(text.into());
    }
    if let Some(nav) = text.is_none().then(|| numpad_navigation(gfx)).flatten() {
        return nav;
    }
    match us_character(gfx, shift) {
        Some(c) => Key::Character(c.to_string().into()),
        None => Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
    }
}

/// A gfx mouse button as winit reports the same button (bevy_winit `convert_mouse_button`): the
/// fourth is Back and the fifth Forward on every backend; winit numbers further X11 buttons by
/// their X button (the sixth gfx button is X button 10).
pub fn mouse_button(gfx: u32) -> MouseButton {
    match gfx {
        mouse_button::LEFT => MouseButton::Left,
        mouse_button::RIGHT => MouseButton::Right,
        mouse_button::MIDDLE => MouseButton::Middle,
        mouse_button::BUTTON_4 => MouseButton::Back,
        4 => MouseButton::Forward,
        n => MouseButton::Other((n + 5) as u16),
    }
}

/// The text of a character event: the NUL-terminated UTF-8 in its five bytes.
pub fn character_text(utf8: &[u8; 5]) -> Option<&str> {
    let len = utf8.iter().position(|&b| b == 0).unwrap_or(utf8.len());
    std::str::from_utf8(&utf8[..len])
        .ok()
        .filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every gfx key names a Bevy key code, and no two share one.
    #[test]
    fn every_gfx_key_has_its_own_key_code() {
        let mut seen = std::collections::HashSet::new();
        assert_eq!(key_code(key::UNKNOWN), None);
        assert_eq!(key_code(key::LAST), None);
        for gfx in 1..key::LAST {
            let code = key_code(gfx).unwrap_or_else(|| panic!("gfx key {gfx} has no KeyCode"));
            assert!(seen.insert(code), "gfx key {gfx} repeats {code:?}");
        }
    }

    /// The position wins over the keymap: an AZERTY `A` (keysym `a` on the US `Q` key) is `KeyQ`,
    /// as winit's scancode says; a key with no position falls back to the keymap, then the code.
    #[test]
    fn the_key_code_is_where_the_key_sits() {
        assert_eq!(
            physical_key(key::Q, key::A, 24, Scancodes::Xkb),
            KeyCode::KeyQ
        );
        assert_eq!(
            physical_key(key::UNKNOWN, key::A, 0, Scancodes::Sdl),
            KeyCode::KeyA
        );
        assert_eq!(
            physical_key(key::UNKNOWN, key::UNKNOWN, 94, Scancodes::Xkb),
            KeyCode::Unidentified(NativeKeyCode::Xkb(94))
        );
        assert_eq!(
            physical_key(key::UNKNOWN, key::UNKNOWN, 0x56, Scancodes::Windows),
            KeyCode::Unidentified(NativeKeyCode::Windows(0x56))
        );
    }

    #[test]
    fn logical_keys_follow_winit() {
        assert_eq!(
            logical_key(key::A, Some("a"), false),
            Key::Character("a".into())
        );
        assert_eq!(
            logical_key(key::A, Some("A"), true),
            Key::Character("A".into())
        );
        // Ctrl+A types `\x01`; the logical key is still `a`.
        assert_eq!(
            logical_key(key::A, Some("\u{1}"), false),
            Key::Character("a".into())
        );
        assert_eq!(logical_key(key::_1, None, true), Key::Character("!".into()));
        assert_eq!(logical_key(key::SPACE, Some(" "), false), Key::Space);
        assert_eq!(logical_key(key::ESCAPE, Some("\u{1b}"), false), Key::Escape);
        assert_eq!(logical_key(key::KP_ENTER, None, false), Key::Enter);
        assert_eq!(logical_key(key::LSHIFT, None, true), Key::Shift);
        // Num Lock on types the digit; off, the key is the navigation key under it.
        assert_eq!(
            logical_key(key::KP_1, Some("1"), false),
            Key::Character("1".into())
        );
        assert_eq!(logical_key(key::KP_1, None, false), Key::End);
        // An AZERTY digit-row key types `&`: the text is the meaning.
        assert_eq!(
            logical_key(key::_1, Some("&"), false),
            Key::Character("&".into())
        );
    }

    /// The 1.12 tokens `BUTTON4`/`BUTTON5` (`bindings::chord::mouse_token`) sit on winit's
    /// `Forward`/`Back`, so gfx's fourth and fifth buttons must land where winit puts them.
    #[test]
    fn extra_buttons_land_where_winit_puts_them() {
        assert_eq!(mouse_button(0), MouseButton::Left);
        assert_eq!(mouse_button(1), MouseButton::Right);
        assert_eq!(mouse_button(2), MouseButton::Middle);
        assert_eq!(mouse_button(3), MouseButton::Back);
        assert_eq!(mouse_button(4), MouseButton::Forward);
        assert_eq!(mouse_button(5), MouseButton::Other(10));
    }

    #[test]
    fn character_text_stops_at_the_nul() {
        assert_eq!(character_text(b"a\0\0\0\0"), Some("a"));
        assert_eq!(character_text(&[0xc3, 0xa9, 0, 0, 0]), Some("é"));
        assert_eq!(character_text(&[0; 5]), None);
        assert_eq!(character_text(&[0xff, 0, 0, 0, 0]), None);
    }
}
