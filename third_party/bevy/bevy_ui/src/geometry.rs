use bevy_math::{MismatchedUnitsError, StableInterpolate as _, TryStableInterpolate, Vec2};
use core::ops::{Div, DivAssign, Mul, MulAssign, Neg};
use thiserror::Error;

/// Represents the possible value types for layout properties.
///
/// This enum allows specifying values for various [`Node`](crate::Node) properties in different units,
/// such as logical pixels, percentages, or automatically determined values.
#[derive(Copy, Clone, Debug)]
pub enum Val {
    /// Automatically determine the value based on the context and other [`Node`](crate::Node) properties.
    Auto,
    /// Set this value in logical pixels.
    Px(f32),
    /// Set the value as a percentage of its parent node's length along a specific axis.
    ///
    /// If the UI node has no parent, the percentage is calculated based on the window's length
    /// along the corresponding axis.
    ///
    /// The chosen axis depends on the [`Node`](crate::Node) field set:
    /// * For `flex_basis`, the percentage is relative to the main-axis length determined by the `flex_direction`.
    /// * For `gap`, `min_size`, `size`, and `max_size`:
    ///   - `width` is relative to the parent's width.
    ///   - `height` is relative to the parent's height.
    /// * For `margin`, `padding`, and `border` values: the percentage is relative to the parent node's width.
    /// * For positions, `left` and `right` are relative to the parent's width, while `bottom` and `top` are relative to the parent's height.
    Percent(f32),
    /// Set this value in percent of the viewport width
    Vw(f32),
    /// Set this value in percent of the viewport height
    Vh(f32),
    /// Set this value in percent of the viewport's smaller dimension.
    VMin(f32),
    /// Set this value in percent of the viewport's larger dimension.
    VMax(f32),
}

impl PartialEq for Val {
    fn eq(&self, other: &Self) -> bool {
        let same_unit = matches!(
            (self, other),
            (Self::Auto, Self::Auto)
                | (Self::Px(_), Self::Px(_))
                | (Self::Percent(_), Self::Percent(_))
                | (Self::Vw(_), Self::Vw(_))
                | (Self::Vh(_), Self::Vh(_))
                | (Self::VMin(_), Self::VMin(_))
                | (Self::VMax(_), Self::VMax(_))
        );

        let left = match self {
            Self::Auto => None,
            Self::Px(v)
            | Self::Percent(v)
            | Self::Vw(v)
            | Self::Vh(v)
            | Self::VMin(v)
            | Self::VMax(v) => Some(v),
        };

        let right = match other {
            Self::Auto => None,
            Self::Px(v)
            | Self::Percent(v)
            | Self::Vw(v)
            | Self::Vh(v)
            | Self::VMin(v)
            | Self::VMax(v) => Some(v),
        };

        match (same_unit, left, right) {
            (true, a, b) => a == b,
            // All zero-value variants are considered equal.
            (false, Some(&a), Some(&b)) => a == 0. && b == 0.,
            _ => false,
        }
    }
}

impl Val {
    pub const DEFAULT: Self = Self::Auto;
    pub const ZERO: Self = Self::Px(0.0);

    /// Returns a [`UiRect`] with its `left` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).left();
    ///
    /// assert_eq!(ui_rect.left, Val::Px(1.));
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn left(self) -> UiRect {
        UiRect::left(self)
    }

    /// Returns a [`UiRect`] with its `right` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).right();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::Px(1.));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn right(self) -> UiRect {
        UiRect::right(self)
    }

    /// Returns a [`UiRect`] with its `top` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).top();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(1.));
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn top(self) -> UiRect {
        UiRect::top(self)
    }

    /// Returns a [`UiRect`] with its `bottom` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).bottom();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::Px(1.));
    /// ```
    pub const fn bottom(self) -> UiRect {
        UiRect::bottom(self)
    }

    /// Returns a [`UiRect`] with all its fields equal to this value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).all();
    ///
    /// assert_eq!(ui_rect.left, Val::Px(1.));
    /// assert_eq!(ui_rect.right, Val::Px(1.));
    /// assert_eq!(ui_rect.top, Val::Px(1.));
    /// assert_eq!(ui_rect.bottom, Val::Px(1.));
    /// ```
    pub const fn all(self) -> UiRect {
        UiRect::all(self)
    }

    /// Returns a [`UiRect`] with all its `left` and `right` equal to this value,
    /// and its `top` and `bottom` set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).horizontal();
    ///
    /// assert_eq!(ui_rect.left, Val::Px(1.));
    /// assert_eq!(ui_rect.right, Val::Px(1.));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn horizontal(self) -> UiRect {
        UiRect::horizontal(self)
    }

    /// Returns a [`UiRect`] with all its `top` and `bottom` equal to this value,
    /// and its `left` and `right` set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).vertical();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(1.));
    /// assert_eq!(ui_rect.bottom, Val::Px(1.));
    /// ```
    pub const fn vertical(self) -> UiRect {
        UiRect::vertical(self)
    }
}

impl Default for Val {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Mul<f32> for Val {
    type Output = Val;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Val::Auto => Val::Auto,
            Val::Px(value) => Val::Px(value * rhs),
            Val::Percent(value) => Val::Percent(value * rhs),
            Val::Vw(value) => Val::Vw(value * rhs),
            Val::Vh(value) => Val::Vh(value * rhs),
            Val::VMin(value) => Val::VMin(value * rhs),
            Val::VMax(value) => Val::VMax(value * rhs),
        }
    }
}

impl MulAssign<f32> for Val {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            Val::Auto => {}
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value) => *value *= rhs,
        }
    }
}

impl Div<f32> for Val {
    type Output = Val;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            Val::Auto => Val::Auto,
            Val::Px(value) => Val::Px(value / rhs),
            Val::Percent(value) => Val::Percent(value / rhs),
            Val::Vw(value) => Val::Vw(value / rhs),
            Val::Vh(value) => Val::Vh(value / rhs),
            Val::VMin(value) => Val::VMin(value / rhs),
            Val::VMax(value) => Val::VMax(value / rhs),
        }
    }
}

impl DivAssign<f32> for Val {
    fn div_assign(&mut self, rhs: f32) {
        match self {
            Val::Auto => {}
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value) => *value /= rhs,
        }
    }
}

impl Neg for Val {
    type Output = Val;

    fn neg(self) -> Self::Output {
        match self {
            Val::Px(value) => Val::Px(-value),
            Val::Percent(value) => Val::Percent(-value),
            Val::Vw(value) => Val::Vw(-value),
            Val::Vh(value) => Val::Vh(-value),
            Val::VMin(value) => Val::VMin(-value),
            Val::VMax(value) => Val::VMax(-value),
            _ => self,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Error)]
pub enum ValArithmeticError {
    #[error("the given variant of Val is not evaluable (non-numeric)")]
    NonEvaluable,
}

impl Val {
    /// Resolves this [`Val`] to a value in physical pixels from the given `scale_factor`, `physical_base_value`,
    /// and `physical_target_size` context values.
    ///
    /// Returns a [`ValArithmeticError::NonEvaluable`] if the [`Val`] is impossible to resolve into a concrete value.
    pub const fn resolve(
        self,
        scale_factor: f32,
        physical_base_value: f32,
        physical_target_size: Vec2,
    ) -> Result<f32, ValArithmeticError> {
        match self {
            Val::Percent(value) => Ok(physical_base_value * value / 100.0),
            Val::Px(value) => Ok(value * scale_factor),
            Val::Vw(value) => Ok(physical_target_size.x * value / 100.0),
            Val::Vh(value) => Ok(physical_target_size.y * value / 100.0),
            Val::VMin(value) => {
                Ok(physical_target_size.x.min(physical_target_size.y) * value / 100.0)
            }
            Val::VMax(value) => {
                Ok(physical_target_size.x.max(physical_target_size.y) * value / 100.0)
            }
            Val::Auto => Err(ValArithmeticError::NonEvaluable),
        }
    }
}

impl TryStableInterpolate for Val {
    type Error = MismatchedUnitsError;

    /// # Example
    ///
    /// ```
    /// # use bevy_ui::Val;
    /// # use bevy_math::TryStableInterpolate;
    /// assert!(matches!(Val::Px(0.0).try_interpolate_stable(&Val::Px(10.0), 0.5), Ok(Val::Px(5.0))));
    /// ```
    fn try_interpolate_stable(&self, other: &Self, t: f32) -> Result<Self, Self::Error> {
        match (self, other) {
            (Val::Px(a), Val::Px(b)) => Ok(Val::Px(a.interpolate_stable(b, t))),
            (Val::Percent(a), Val::Percent(b)) => Ok(Val::Percent(a.interpolate_stable(b, t))),
            (Val::Vw(a), Val::Vw(b)) => Ok(Val::Vw(a.interpolate_stable(b, t))),
            (Val::Vh(a), Val::Vh(b)) => Ok(Val::Vh(a.interpolate_stable(b, t))),
            (Val::VMin(a), Val::VMin(b)) => Ok(Val::VMin(a.interpolate_stable(b, t))),
            (Val::VMax(a), Val::VMax(b)) => Ok(Val::VMax(a.interpolate_stable(b, t))),
            (Val::Auto, Val::Auto) => Ok(Val::Auto),
            _ => Err(MismatchedUnitsError),
        }
    }
}

/// A type which is commonly used to define margins, paddings and borders.
///
/// # Examples
///
/// ## Margin
///
/// A margin is used to create space around UI elements, outside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let margin = UiRect::all(Val::Auto); // Centers the UI element
/// ```
///
/// ## Padding
///
/// A padding is used to create space around UI elements, inside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let padding = UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
///
/// ## Borders
///
/// A border is used to define the width of the border of a UI element.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let border = UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct UiRect {
    /// The value corresponding to the left side of the UI rect.
    pub left: Val,
    /// The value corresponding to the right side of the UI rect.
    pub right: Val,
    /// The value corresponding to the top side of the UI rect.
    pub top: Val,
    /// The value corresponding to the bottom side of the UI rect.
    pub bottom: Val,
}

impl UiRect {
    pub const DEFAULT: Self = Self::all(Val::ZERO);
    pub const ZERO: Self = Self::all(Val::ZERO);
    pub const AUTO: Self = Self::all(Val::Auto);

    /// Creates a new [`UiRect`] from the values specified.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::new(
    ///     Val::Px(10.0),
    ///     Val::Px(20.0),
    ///     Val::Px(30.0),
    ///     Val::Px(40.0),
    /// );
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(30.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(40.0));
    /// ```
    pub const fn new(left: Val, right: Val, top: Val, bottom: Val) -> Self {
        UiRect {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates a new [`UiRect`] where all sides have the same value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub const fn all(value: Val) -> Self {
        UiRect {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Creates a new [`UiRect`] from the values specified in logical pixels.
    ///
    /// This is a shortcut for [`UiRect::new()`], applying [`Val::Px`] to all arguments.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::px(10., 20., 30., 40.);
    /// assert_eq!(ui_rect.left, Val::Px(10.));
    /// assert_eq!(ui_rect.right, Val::Px(20.));
    /// assert_eq!(ui_rect.top, Val::Px(30.));
    /// assert_eq!(ui_rect.bottom, Val::Px(40.));
    /// ```
    pub const fn px(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        UiRect {
            left: Val::Px(left),
            right: Val::Px(right),
            top: Val::Px(top),
            bottom: Val::Px(bottom),
        }
    }

    /// Creates a new [`UiRect`] from the values specified in percentages.
    ///
    /// This is a shortcut for [`UiRect::new()`], applying [`Val::Percent`] to all arguments.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::percent(5., 10., 2., 1.);
    /// assert_eq!(ui_rect.left, Val::Percent(5.));
    /// assert_eq!(ui_rect.right, Val::Percent(10.));
    /// assert_eq!(ui_rect.top, Val::Percent(2.));
    /// assert_eq!(ui_rect.bottom, Val::Percent(1.));
    /// ```
    pub const fn percent(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        UiRect {
            left: Val::Percent(left),
            right: Val::Percent(right),
            top: Val::Percent(top),
            bottom: Val::Percent(bottom),
        }
    }

    /// Creates a new [`UiRect`] where `left` and `right` take the given value,
    /// and `top` and `bottom` set to zero `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::horizontal(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn horizontal(value: Val) -> Self {
        Self {
            left: value,
            right: value,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `top` and `bottom` take the given value,
    /// and `left` and `right` are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::vertical(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub const fn vertical(value: Val) -> Self {
        Self {
            top: value,
            bottom: value,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where both `left` and `right` take the value of `horizontal`, and both `top` and `bottom` take the value of `vertical`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::axes(Val::Px(10.0), Val::Percent(15.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Percent(15.0));
    /// assert_eq!(ui_rect.bottom, Val::Percent(15.0));
    /// ```
    pub const fn axes(horizontal: Val, vertical: Val) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    /// Creates a new [`UiRect`] where `left` takes the given value, and
    /// the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::left(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn left(left: Val) -> Self {
        Self {
            left,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `right` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::right(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn right(right: Val) -> Self {
        Self {
            right,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `top` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::top(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn top(top: Val) -> Self {
        Self {
            top,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `bottom` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::bottom(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub const fn bottom(bottom: Val) -> Self {
        Self {
            bottom,
            ..Self::DEFAULT
        }
    }
}

impl Default for UiRect {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<Val> for UiRect {
    fn from(value: Val) -> Self {
        UiRect::all(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::*;
    use bevy_math::vec2;

    #[test]
    fn val_evaluate() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let result = Val::Percent(80.).resolve(1., size, viewport_size).unwrap();

        assert_eq!(result, size * 0.8);
    }

    #[test]
    fn val_resolve_px() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let result = Val::Px(10.).resolve(1., size, viewport_size).unwrap();

        assert_eq!(result, 10.);
    }

    #[test]
    fn val_resolve_viewport_coords() {
        let size = 250.;
        let viewport_size = vec2(500., 500.);

        for value in (-10..10).map(|value| value as f32) {
            // for a square viewport there should be no difference between `Vw` and `Vh` and between `Vmin` and `Vmax`.
            assert_eq!(
                Val::Vw(value).resolve(1., size, viewport_size),
                Val::Vh(value).resolve(1., size, viewport_size)
            );
            assert_eq!(
                Val::VMin(value).resolve(1., size, viewport_size),
                Val::VMax(value).resolve(1., size, viewport_size)
            );
            assert_eq!(
                Val::VMin(value).resolve(1., size, viewport_size),
                Val::Vw(value).resolve(1., size, viewport_size)
            );
        }

        let viewport_size = vec2(1000., 500.);
        assert_eq!(
            Val::Vw(100.).resolve(1., size, viewport_size).unwrap(),
            1000.
        );
        assert_eq!(
            Val::Vh(100.).resolve(1., size, viewport_size).unwrap(),
            500.
        );
        assert_eq!(Val::Vw(60.).resolve(1., size, viewport_size).unwrap(), 600.);
        assert_eq!(Val::Vh(40.).resolve(1., size, viewport_size).unwrap(), 200.);
        assert_eq!(
            Val::VMin(50.).resolve(1., size, viewport_size).unwrap(),
            250.
        );
        assert_eq!(
            Val::VMax(75.).resolve(1., size, viewport_size).unwrap(),
            750.
        );
    }

    #[test]
    fn val_auto_is_non_evaluable() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let resolve_auto = Val::Auto.resolve(1., size, viewport_size);

        assert_eq!(resolve_auto, Err(ValArithmeticError::NonEvaluable));
    }

    #[test]
    fn val_arithmetic_error_messages() {
        assert_eq!(
            format!("{}", ValArithmeticError::NonEvaluable),
            "the given variant of Val is not evaluable (non-numeric)"
        );
    }

    #[test]
    fn default_val_equals_const_default_val() {
        assert_eq!(Val::default(), Val::DEFAULT);
    }

    #[test]
    fn uirect_default_equals_const_default() {
        assert_eq!(UiRect::default(), UiRect::all(Val::ZERO));
        assert_eq!(UiRect::default(), UiRect::DEFAULT);
    }

    #[test]
    fn test_uirect_axes() {
        let x = Val::Px(1.);
        let y = Val::Vw(4.);
        let r = UiRect::axes(x, y);
        let h = UiRect::horizontal(x);
        let v = UiRect::vertical(y);

        assert_eq!(r.top, v.top);
        assert_eq!(r.bottom, v.bottom);
        assert_eq!(r.left, h.left);
        assert_eq!(r.right, h.right);
    }

    #[test]
    fn uirect_px() {
        let r = UiRect::px(3., 5., 20., 999.);
        assert_eq!(r.left, Val::Px(3.));
        assert_eq!(r.right, Val::Px(5.));
        assert_eq!(r.top, Val::Px(20.));
        assert_eq!(r.bottom, Val::Px(999.));
    }

    #[test]
    fn uirect_percent() {
        let r = UiRect::percent(3., 5., 20., 99.);
        assert_eq!(r.left, Val::Percent(3.));
        assert_eq!(r.right, Val::Percent(5.));
        assert_eq!(r.top, Val::Percent(20.));
        assert_eq!(r.bottom, Val::Percent(99.));
    }
}
