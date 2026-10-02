//! Components to customize the window cursor.

#[cfg(feature = "custom_cursor")]
mod custom_cursor;
mod system_cursor;

#[cfg(feature = "custom_cursor")]
pub use custom_cursor::*;
pub use system_cursor::*;

use bevy_ecs::component::Component;

#[cfg(feature = "custom_cursor")]
pub use crate::cursor::{CustomCursor, CustomCursorImage};

/// Insert into a window entity to set the cursor for that window.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum CursorIcon {
    #[cfg(feature = "custom_cursor")]
    /// Custom cursor image.
    Custom(CustomCursor),
    /// System provided cursor icon.
    System(SystemCursorIcon),
}

impl Default for CursorIcon {
    fn default() -> Self {
        CursorIcon::System(Default::default())
    }
}

impl From<SystemCursorIcon> for CursorIcon {
    fn from(icon: SystemCursorIcon) -> Self {
        CursorIcon::System(icon)
    }
}
