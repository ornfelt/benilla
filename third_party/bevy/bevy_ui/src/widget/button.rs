use crate::{FocusPolicy, Interaction, Node};
use bevy_ecs::component::Component;

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[require(Node, FocusPolicy::Block, Interaction)]
pub struct Button;
