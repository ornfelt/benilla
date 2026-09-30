//! Provides `Instant` for all platforms.

pub use time::Instant;

crate::cfg::switch! {
    crate::cfg::std => {
        use std::time;
    }
    _ => {
        mod fallback;

        use fallback as time;
    }
}
