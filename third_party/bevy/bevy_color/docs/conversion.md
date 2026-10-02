# Conversion

Conversion between the various color spaces is achieved using Rust's native [From] trait. Because certain color spaces are defined by their transformation to and from another space, these [From] implementations reflect that set of definitions.

```rust
# use bevy_color::{Srgba, LinearRgba};
let color = Srgba::rgb(0.5, 0.5, 0.5);

// Using From explicitly
let linear_color = LinearRgba::from(color);

// Using Into
let linear_color: LinearRgba = color.into();
```

For example, the [sRGB](crate::Srgba) space is defined by its relationship with [Linear RGB](crate::LinearRgba), so it is the responsibility of [sRGB](crate::Srgba) to provide [From] implementations for [Linear RGB](crate::LinearRgba).
