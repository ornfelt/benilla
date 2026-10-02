/// This module contains components that are used to track the interaction state of UI widgets.
///
/// Their observers mirrored the state into the entity's `AccessibilityNode`; no AccessKit adapter
/// exists, so they are stand-ins that keep the same observers registered.
use bevy_ecs::{
    component::Component,
    lifecycle::{Add, Remove},
    observer::On,
};

/// A component indicating that a widget is disabled and should be "grayed out".
/// This is used to prevent user interaction with the widget. It should not, however, prevent
/// the widget from being updated or rendered, or from acquiring keyboard focus.
///
/// For apps which support a11y: if a widget (such as a slider) contains multiple entities,
/// the `InteractionDisabled` component should be added to the root entity of the widget - the
/// same entity that contains the `AccessibilityNode` component. This will ensure that
/// the a11y tree is updated correctly.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct InteractionDisabled;

/// Stand-in for the observer that marked the `AccessibilityNode` disabled.
pub(crate) fn on_add_disabled(_add: On<Add, InteractionDisabled>) {}

/// Stand-in for the observer that cleared the `AccessibilityNode`'s disabled flag.
pub(crate) fn on_remove_disabled(_remove: On<Remove, InteractionDisabled>) {}

/// Component that indicates whether a checkbox or radio button is in a checked state.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Checked;

/// Stand-in for the observer that set the `AccessibilityNode`'s toggled state.
pub(crate) fn on_add_checkable(_add: On<Add, Checked>) {}

/// Stand-in for the observer that cleared the `AccessibilityNode`'s toggled state (Bevy 0.18.1
/// registers it on `Add<Checked>`, kept).
pub(crate) fn on_remove_checkable(_add: On<Add, Checked>) {}

/// Stand-in for the observer that set the `AccessibilityNode` toggled on.
pub(crate) fn on_add_checked(_add: On<Add, Checked>) {}

/// Stand-in for the observer that set the `AccessibilityNode` toggled off.
pub(crate) fn on_remove_checked(_remove: On<Remove, Checked>) {}
