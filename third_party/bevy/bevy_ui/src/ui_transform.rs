use crate::Val;
use bevy_derive::Deref;
use bevy_ecs::component::Component;
use bevy_math::Affine2;
use bevy_math::Mat2;
use bevy_math::Rot2;
use bevy_math::Vec2;
use core::ops::Mul;

/// A pair of [`Val`]s used to represent a 2-dimensional size or offset.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Val2 {
    /// Translate the node along the x-axis.
    /// `Val::Percent` values are resolved based on the computed width of the Ui Node.
    /// `Val::Auto` is resolved to `0.`.
    pub x: Val,
    /// Translate the node along the y-axis.
    /// `Val::Percent` values are resolved based on the computed height of the UI Node.
    /// `Val::Auto` is resolved to `0.`.
    pub y: Val,
}

impl Val2 {
    pub const ZERO: Self = Self {
        x: Val::ZERO,
        y: Val::ZERO,
    };

    /// Resolves this [`Val2`] from the given `scale_factor`, `parent_size`,
    /// and `viewport_size`.
    ///
    /// Component values of [`Val::Auto`] are resolved to 0.
    pub fn resolve(&self, scale_factor: f32, base_size: Vec2, viewport_size: Vec2) -> Vec2 {
        Vec2::new(
            self.x
                .resolve(scale_factor, base_size.x, viewport_size)
                .unwrap_or(0.),
            self.y
                .resolve(scale_factor, base_size.y, viewport_size)
                .unwrap_or(0.),
        )
    }
}

impl Default for Val2 {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Relative 2D transform for UI nodes
///
/// [`UiGlobalTransform`] is automatically inserted whenever [`UiTransform`] is inserted.
#[derive(Component, Debug, PartialEq, Clone, Copy)]
#[require(UiGlobalTransform)]
pub struct UiTransform {
    /// Translate the node.
    pub translation: Val2,
    /// Scale the node. A negative value reflects the node in that axis.
    pub scale: Vec2,
    /// Rotate the node clockwise.
    pub rotation: Rot2,
}

impl UiTransform {
    pub const IDENTITY: Self = Self {
        translation: Val2::ZERO,
        scale: Vec2::ONE,
        rotation: Rot2::IDENTITY,
    };

    /// Resolves the translation from the given `scale_factor`, `base_value`, and `target_size`
    /// and returns a 2d affine transform from the resolved translation, and the `UiTransform`'s rotation, and scale.
    pub fn compute_affine(&self, scale_factor: f32, base_size: Vec2, target_size: Vec2) -> Affine2 {
        Affine2::from_mat2_translation(
            Mat2::from(self.rotation) * Mat2::from_diagonal(self.scale),
            self.translation
                .resolve(scale_factor, base_size, target_size),
        )
    }
}

impl Default for UiTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Absolute 2D transform for UI nodes
///
/// [`UiGlobalTransform`]s are updated from [`UiTransform`] and [`Node`](crate::ui_node::Node)
///  in [`ui_layout_system`](crate::layout::ui_layout_system)
#[derive(Component, Debug, PartialEq, Clone, Copy, Deref)]
pub struct UiGlobalTransform(Affine2);

impl Default for UiGlobalTransform {
    fn default() -> Self {
        Self(Affine2::IDENTITY)
    }
}

impl UiGlobalTransform {
    /// If the transform is invertible returns its inverse.
    /// Otherwise returns `None`.
    #[inline]
    pub fn try_inverse(&self) -> Option<Affine2> {
        (self.matrix2.determinant() != 0.).then_some(self.inverse())
    }

    /// Returns the transform as an [`Affine2`]
    #[inline]
    pub fn affine(&self) -> Affine2 {
        self.0
    }
}

impl From<Affine2> for UiGlobalTransform {
    fn from(value: Affine2) -> Self {
        Self(value)
    }
}

impl From<UiGlobalTransform> for Affine2 {
    fn from(value: UiGlobalTransform) -> Self {
        value.0
    }
}

impl From<&UiGlobalTransform> for Affine2 {
    fn from(value: &UiGlobalTransform) -> Self {
        value.0
    }
}

impl Mul for UiGlobalTransform {
    type Output = Self;

    #[inline]
    fn mul(self, value: Self) -> Self::Output {
        Self(self.0 * value.0)
    }
}

impl Mul<Affine2> for UiGlobalTransform {
    type Output = Affine2;

    #[inline]
    fn mul(self, affine2: Affine2) -> Self::Output {
        self.0 * affine2
    }
}

impl Mul<UiGlobalTransform> for Affine2 {
    type Output = Affine2;

    #[inline]
    fn mul(self, transform: UiGlobalTransform) -> Self::Output {
        self * transform.0
    }
}

impl Mul<Vec2> for UiGlobalTransform {
    type Output = Vec2;

    #[inline]
    fn mul(self, value: Vec2) -> Vec2 {
        self.transform_point2(value)
    }
}
