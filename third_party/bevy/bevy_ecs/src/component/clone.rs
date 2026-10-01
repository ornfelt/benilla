/// The clone behavior of a [`Component`](crate::component::Component).
///
/// Without an entity cloner, the one reader is scene writing, which skips [`Ignore`](Self::Ignore)
/// components.
#[derive(Clone, Debug, Default)]
pub enum ComponentCloneBehavior {
    /// Uses the default behavior.
    #[default]
    Default,
    /// Do not clone/move this component.
    Ignore,
}
