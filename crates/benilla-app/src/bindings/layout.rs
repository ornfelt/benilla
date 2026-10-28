//! What the active keyboard layout names each letter, digit and punctuation key, which 1.12 binds
//! by. The Windows client names a key from the virtual-key code the layout produced: the
//! translator `0x42d800` keeps `VK_0`-`VK_9` as themselves (`0x42d81c`) and sends every other key
//! outside its fixed table, `VK_A`-`VK_Z` and `VK_OEM_*` among them, to
//! `MapVirtualKeyA(vk, MAPVK_VK_TO_CHAR)` (`0x42da39`), which returns `VK_A`-`VK_Z`'s own letter
//! whatever the layout types there. The Mac client fills its letter and punctuation keys from the
//! live layout, `KeyTranslate` (`0x8a3c3`) on the current `KCHR` (re-read when it changes,
//! `0x8a345`), and fixes its digits by position (`0x5bf320`). Bevy's `KeyboardInput` carries the
//! physical key and a `logical_key` with Shift applied, so the name is read off the gfx window's
//! raw key messages here ([`benilla_gfx::GfxRawKey`]), before any `Update` reader.

use std::collections::HashMap;

use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

/// A layout-named key's place on a US keyboard, with the US name there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Position {
    Letter(char),
    Digit(char),
    /// The ISO and JIS extra keys and the keypad comma have no US name.
    Punctuation(Option<char>),
}

impl Position {
    fn us(self) -> Option<char> {
        match self {
            Position::Letter(c) | Position::Digit(c) => Some(c),
            Position::Punctuation(c) => c,
        }
    }
}

/// The keys a layout names: the letters, the digits, the punctuation, the ISO and JIS extras and
/// the keypad comma. Windows sends `VK_OEM_102`, `VK_ABNT_C2` and `VK_SEPARATOR` to the layout arm
/// like any `VK_OEM_*` key, and the Mac table leaves the ISO key `0x0A` and the keypad comma
/// `0x5F` to `KeyTranslate` (`-1` at `0x5bf320`). winit 0.30 cannot deliver those on a Mac, as on
/// main: it reports `0x0A` as `Backquote` (whose scancode it gives back as `0x32`) and leaves
/// `0x5E` and `0x5F` unidentified, so the `IntlBackslash`, `IntlRo` and `NumpadComma` arms never
/// fire there.
pub(crate) fn position(k: KeyCode) -> Option<Position> {
    use KeyCode::*;
    use Position::{Digit, Letter, Punctuation};
    Some(match k {
        KeyA => Letter('A'),
        KeyB => Letter('B'),
        KeyC => Letter('C'),
        KeyD => Letter('D'),
        KeyE => Letter('E'),
        KeyF => Letter('F'),
        KeyG => Letter('G'),
        KeyH => Letter('H'),
        KeyI => Letter('I'),
        KeyJ => Letter('J'),
        KeyK => Letter('K'),
        KeyL => Letter('L'),
        KeyM => Letter('M'),
        KeyN => Letter('N'),
        KeyO => Letter('O'),
        KeyP => Letter('P'),
        KeyQ => Letter('Q'),
        KeyR => Letter('R'),
        KeyS => Letter('S'),
        KeyT => Letter('T'),
        KeyU => Letter('U'),
        KeyV => Letter('V'),
        KeyW => Letter('W'),
        KeyX => Letter('X'),
        KeyY => Letter('Y'),
        KeyZ => Letter('Z'),
        Digit1 => Digit('1'),
        Digit2 => Digit('2'),
        Digit3 => Digit('3'),
        Digit4 => Digit('4'),
        Digit5 => Digit('5'),
        Digit6 => Digit('6'),
        Digit7 => Digit('7'),
        Digit8 => Digit('8'),
        Digit9 => Digit('9'),
        Digit0 => Digit('0'),
        Minus => Punctuation(Some('-')),
        Equal => Punctuation(Some('=')),
        BracketLeft => Punctuation(Some('[')),
        BracketRight => Punctuation(Some(']')),
        Backslash => Punctuation(Some('\\')),
        Semicolon => Punctuation(Some(';')),
        Quote => Punctuation(Some('\'')),
        Comma => Punctuation(Some(',')),
        Period => Punctuation(Some('.')),
        Slash => Punctuation(Some('/')),
        Backquote => Punctuation(Some('`')),
        IntlBackslash | IntlRo | IntlYen | NumpadComma => Punctuation(None),
        _ => return None,
    })
}

/// A layout-named key's name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LayoutName {
    Char(char),
    /// None: `MapVirtualKeyA` returned 0 and the reference drops the key's message (`0x42da49`).
    Dropped,
}

/// Each layout-named key's name under the active layout, as its platform last reported it: an
/// entry changes with the next message of its key after a layout switch, so a press is named under
/// the layout it was typed in. Empty where no window feeds it (a headless run, a test).
#[derive(Resource, Default, Debug)]
pub(crate) struct LayoutNames(HashMap<KeyCode, LayoutName>);

impl LayoutNames {
    /// The name of `key` if a layout names it, `None` for a key the fixed table names. A key never
    /// reported (a headless run) keeps its US name.
    ///
    /// Deviation: on the Mac and Linux a key its platform reports no character for keeps its US
    /// name too, where the Mac reference drops it (`0x8a4aa`), and a Linux dead key not yet pressed
    /// bare stays bindable.
    pub(crate) fn name(&self, key: KeyCode) -> Option<LayoutName> {
        let position = position(key)?;
        Some(match self.0.get(&key) {
            Some(name) => *name,
            None => position.us().map_or(LayoutName::Dropped, LayoutName::Char),
        })
    }

    /// Set what the layout names `key`; `None` clears it back to the US name.
    pub(crate) fn set(&mut self, key: KeyCode, name: Option<LayoutName>) {
        match name {
            Some(name) => self.0.insert(key, name),
            None => self.0.remove(&key),
        };
    }
}

/// Keeps [`LayoutNames`] current from the gfx window's raw key messages (Windows).
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LayoutNames>();
    #[cfg(target_os = "windows")]
    app.add_systems(PreUpdate, record_gfx_layout_names);
}

/// Names each key the gfx window's key messages report, on Windows: by its scancode, the
/// reference's path (`windows::scancode_name`). Elsewhere the gfx window reports no unshifted
/// character, so a key keeps its US name (`GFX.md`, "Known differences").
#[cfg(target_os = "windows")]
fn record_gfx_layout_names(
    mut raw: MessageReader<benilla_gfx::GfxRawKey>,
    mut names: ResMut<LayoutNames>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    for key in raw.read() {
        if position(key.key_code).is_none() {
            continue;
        }
        // An SDL window's scancodes are USB usages, not Win32's: that key keeps its US name.
        if let Some(scancode) = key.windows_scancode() {
            names.set(key.key_code, windows::scancode_name(scancode));
        }
    }
}

/// The reference's name for the virtual key `vk`, given `MapVirtualKeyExW(vk, MAPVK_VK_TO_CHAR)`;
/// `None` for a key `0x42d800`'s fixed table names.
///
/// Deviation: a character above U+00FF names the key by itself. The reference's `MapVirtualKeyA`
/// returns an ANSI code-page byte, which `0x42da4b` drops past `0xff` and the namer's UTF-8
/// encoder (`0x41abb0`) otherwise reads as Latin-1, so such a key is mislabelled or dropped; named
/// by what it types, it stays bindable and readable.
#[cfg(any(target_os = "windows", test))]
fn windows_name(vk: u32, to_char: u32) -> Option<LayoutName> {
    match vk {
        // `VK_A`-`VK_Z`, named by their letter, and `VK_0`-`VK_9`, kept as themselves.
        0x41..=0x5a | 0x30..=0x39 => Some(LayoutName::Char(char::from(vk as u8))),
        _ if fixed_vk(vk) => None,
        // The dynamic arm: `and eax,0xffff` (`0x42da42`) drops the dead-key bit, and 0 drops the
        // key (`0x42da49`). U+F000-U+F002 are a layout table's `WCH_NONE`, `WCH_DEAD` and
        // `WCH_LGTR` markers, no character, should the call ever return one.
        _ => Some(match char::from_u32(to_char & 0xffff) {
            Some(c)
                if !c.is_control()
                    && !c.is_whitespace()
                    && !('\u{f000}'..='\u{f002}').contains(&c) =>
            {
                LayoutName::Char(c)
            }
            _ => LayoutName::Dropped,
        }),
    }
}

/// `0x42d800`'s fixed table, read from its byte remap at `0x42dafc` and jump table at `0x42da60`:
/// the keys it names itself, never by character. F1-F12 take their own arm (`0x42d800`).
#[cfg(any(target_os = "windows", test))]
fn fixed_vk(vk: u32) -> bool {
    matches!(
        vk,
        0x08 | 0x09
            | 0x0d
            | 0x10..=0x14
            | 0x1b
            | 0x20..=0x28
            | 0x2c..=0x2e
            | 0x60..=0x6b
            | 0x6d..=0x7b
            | 0x90
            | 0x91
    )
}

#[cfg(target_os = "windows")]
mod windows {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyExW, MAPVK_VK_TO_CHAR, MAPVK_VSC_TO_VK_EX,
    };

    /// The reference's own path for the key at `scancode` (winit's form, the extended keys at
    /// `0xE0xx`, as [`benilla_gfx::GfxRawKey::windows_scancode`] gives it): the virtual key the
    /// window thread's layout gives it, then that key's name (`super::windows_name`).
    pub(super) fn scancode_name(scancode: u32) -> Option<super::LayoutName> {
        // SAFETY: plain Win32 calls with no pointers. `GetKeyboardLayout(0)` is the calling
        // thread's layout; the caller runs on the main thread, the window's, so it is the layout
        // the window's key messages were made under.
        let (vk, to_char) = unsafe {
            let hkl = GetKeyboardLayout(0);
            let vk = MapVirtualKeyExW(scancode, MAPVK_VSC_TO_VK_EX, hkl);
            (vk, MapVirtualKeyExW(vk, MAPVK_VK_TO_CHAR, hkl))
        };
        if vk == 0 {
            return None;
        }
        super::windows_name(vk, to_char)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A layout key's name under `names`, its ASCII letters folded up as the lookup folds them.
    fn named(names: &LayoutNames, k: KeyCode) -> Option<char> {
        match names.name(k) {
            Some(LayoutName::Char(c)) => Some(c.to_ascii_uppercase()),
            _ => None,
        }
    }

    const VK_OEM_1: u32 = 0xba;
    const VK_OEM_COMMA: u32 = 0xbc;
    const VK_OEM_3: u32 = 0xc0;
    const VK_OEM_6: u32 = 0xdd;
    const VK_OEM_7: u32 = 0xde;

    /// Windows: the reference's path on rows of the layouts' own tables (the key, its virtual
    /// key, its unshifted character), as `MapVirtualKeyExW` returns them.
    #[test]
    fn windows_names_a_key_by_its_virtual_key_as_the_reference_does() {
        let vk = |c: u8| u32::from(c);
        let rows: &[(&str, KeyCode, u32, u32, char)] = &[
            // Hebrew: the W key types `'` but is `VK_W`; Q types `/` and is `VK_Q`.
            ("hebrew W", KeyCode::KeyW, vk(b'W'), '\''.into(), 'W'),
            ("hebrew Q", KeyCode::KeyQ, vk(b'Q'), '/'.into(), 'Q'),
            ("greek Q", KeyCode::KeyQ, vk(b'Q'), ';'.into(), 'Q'),
            ("lithuanian Q", KeyCode::KeyQ, vk(b'Q'), 'ą'.into(), 'Q'),
            ("lithuanian W", KeyCode::KeyW, vk(b'W'), 'ž'.into(), 'W'),
            ("turkish ı", KeyCode::KeyI, vk(b'I'), 'ı'.into(), 'I'),
            // Turkish Q's `VK_OEM_7` types `i`: one name with `I`, as the lookup folds it.
            ("turkish i", KeyCode::Quote, VK_OEM_7, 'i'.into(), 'I'),
            // Bulgarian: `VK_Q` on the Period key, `VK_OEM_COMMA` on Q.
            ("bulgarian .", KeyCode::Period, vk(b'Q'), 'л'.into(), 'Q'),
            ("bulgarian Q", KeyCode::KeyQ, VK_OEM_COMMA, ','.into(), ','),
            // Hungarian: `VK_0` on Backquote, `VK_OEM_3` `ö` on the 0 key.
            ("hungarian `", KeyCode::Backquote, vk(b'0'), '0'.into(), '0'),
            ("hungarian 0", KeyCode::Digit0, VK_OEM_3, 'ö'.into(), 'ö'),
            // Dvorak's Q position is `VK_OEM_7`, `'`.
            ("dvorak Q", KeyCode::KeyQ, VK_OEM_7, '\''.into(), '\''),
            ("us ;", KeyCode::Semicolon, VK_OEM_1, ';'.into(), ';'),
            // A dead key's bit (`0x80000000`) is masked off: French `^`.
            ("french ^", KeyCode::BracketLeft, VK_OEM_6, 0x8000_005e, '^'),
            // Above U+00FF the key is named by what it types (the deviation).
            ("russian ж", KeyCode::Semicolon, VK_OEM_1, 'ж'.into(), 'ж'),
            // The ISO key, German `VK_OEM_102`; the keypad comma, Brazilian ABNT's `VK_ABNT_C2`
            // and a `VK_SEPARATOR` keypad.
            ("german <", KeyCode::IntlBackslash, 0xe2, '<'.into(), '<'),
            ("abnt keypad .", KeyCode::NumpadComma, 0xc2, '.'.into(), '.'),
            ("keypad ,", KeyCode::NumpadComma, 0x6c, ','.into(), ','),
        ];
        let mut names = LayoutNames::default();
        for &(what, key, vk, to_char, expected) in rows {
            names.set(key, windows_name(vk, to_char));
            assert_eq!(named(&names, key), Some(expected), "{what}");
        }
        // `MapVirtualKeyA` returning 0 drops the key (`0x42da49`): Japanese 106's
        // `VK_OEM_AUTO` on Backquote.
        assert_eq!(windows_name(0xf3, 0), Some(LayoutName::Dropped));
        names.set(KeyCode::Backquote, windows_name(0xf3, 0));
        assert_eq!(names.name(KeyCode::Backquote), Some(LayoutName::Dropped));
        // A layout table's `WCH_NONE`/`WCH_DEAD`/`WCH_LGTR` marker is no character.
        for marker in 0xf000..=0xf002 {
            assert_eq!(
                windows_name(VK_OEM_1, marker),
                Some(LayoutName::Dropped),
                "{marker:#x}"
            );
        }
        // A key the fixed table names is no layout key's: F1, Space, Numpad 0.
        for vk in [0x70, 0x20, 0x60] {
            assert_eq!(windows_name(vk, 0), None, "{vk:#x}");
        }
    }

    /// The fixed table is `0x42d800`'s, decoded from its remap bytes: the 60 entries less the
    /// digits and F1-F12, which take their own arms.
    #[test]
    fn the_fixed_virtual_keys_are_the_translators_table() {
        let fixed: Vec<u32> = (0..=0xffu32)
            .filter(|&vk| fixed_vk(vk) && !(0x70..=0x7b).contains(&vk))
            .collect();
        assert_eq!(
            fixed,
            [
                0x08, 0x09, 0x0d, 0x10, 0x11, 0x12, 0x13, 0x14, 0x1b, 0x20, 0x21, 0x22, 0x23, 0x24,
                0x25, 0x26, 0x27, 0x28, 0x2c, 0x2d, 0x2e, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
                0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6d, 0x6e, 0x6f, 0x90, 0x91,
            ]
        );
    }

    #[test]
    fn a_layout_switch_renames_a_key_at_its_next_message() {
        let mut names = LayoutNames::default();
        names.set(KeyCode::KeyW, Some(LayoutName::Char('z')));
        assert_eq!(named(&names, KeyCode::KeyW), Some('Z'));
        names.set(KeyCode::KeyW, Some(LayoutName::Char('w')));
        assert_eq!(
            named(&names, KeyCode::KeyW),
            Some('W'),
            "back on a US layout"
        );
        // A layout that makes no character there falls back to the US name.
        names.set(KeyCode::KeyQ, Some(LayoutName::Char('a')));
        names.set(KeyCode::KeyQ, None);
        assert_eq!(named(&names, KeyCode::KeyQ), Some('Q'));
        // Digits and punctuation name themselves until a layout says otherwise; an ISO key no
        // layout reported has no name.
        assert_eq!(named(&names, KeyCode::Digit1), Some('1'));
        assert_eq!(named(&names, KeyCode::Quote), Some('\''));
        for key in [KeyCode::IntlBackslash, KeyCode::NumpadComma] {
            assert_eq!(names.name(key), Some(LayoutName::Dropped), "{key:?}");
        }
        assert_eq!(names.name(KeyCode::F1), None, "the fixed table's");
    }
}
