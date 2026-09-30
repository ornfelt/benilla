use bevy_ecs::{define_label, intern::Interned};

pub use bevy_ecs::label::DynEq;
pub use bevy_render_macros::{RenderLabel, RenderSubGraph};

define_label!(
    #[diagnostic::on_unimplemented(
        note = "consider annotating `{Self}` with `#[derive(RenderLabel)]`"
    )]
    /// A strongly-typed class of labels used to identify a node in a render graph.
    RenderLabel,
    RENDER_LABEL_INTERNER
);

/// A shorthand for `Interned<dyn RenderLabel>`.
pub type InternedRenderLabel = Interned<dyn RenderLabel>;

define_label!(
    #[diagnostic::on_unimplemented(
        note = "consider annotating `{Self}` with `#[derive(RenderSubGraph)]`"
    )]
    /// A strongly-typed class of labels used to identify a sub-graph in a render graph.
    RenderSubGraph,
    RENDER_SUB_GRAPH_INTERNER
);

/// A shorthand for `Interned<dyn RenderSubGraph>`.
pub type InternedRenderSubGraph = Interned<dyn RenderSubGraph>;
