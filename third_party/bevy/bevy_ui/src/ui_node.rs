use crate::{
    ui_transform::{UiGlobalTransform, UiTransform},
    FocusPolicy, UiRect, Val,
};
use bevy_camera::{visibility::Visibility, Camera, RenderTarget};
use bevy_color::{Alpha, Color};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::{BVec2, Rect, UVec2, Vec2, Vec4, Vec4Swizzles};
use bevy_sprite::BorderRect;
use bevy_utils::once;
use bevy_window::{PrimaryWindow, WindowRef};
use core::f32;
use tracing::warn;

/// Provides the computed size and layout properties of the node.
///
/// Fields in this struct are public but should not be modified under most circumstances.
/// For example, in a scrollbar you may want to derive the handle's size from the proportion of
/// scrollable content in-view. You can directly modify `ComputedNode` after layout to set the
/// handle size without any delays.
#[derive(Component, Debug, Copy, Clone, PartialEq)]
pub struct ComputedNode {
    /// The order of the node in the UI layout.
    /// Nodes with a higher stack index are drawn on top of and receive interactions before nodes with lower stack indices.
    ///
    /// Automatically calculated in [`UiSystems::Stack`](`super::UiSystems::Stack`).
    pub stack_index: u32,
    /// The size of the node as width and height in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub size: Vec2,
    /// Size of this node's content.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub content_size: Vec2,
    /// Space allocated for scrollbars.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub scrollbar_size: Vec2,
    /// Resolved offset of scrolled content
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub scroll_position: Vec2,
    /// The width of this node's outline.
    /// If this value is `Auto`, negative or `0.` then no outline will be rendered.
    /// Outline updates bypass change detection.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub outline_width: f32,
    /// The amount of space between the outline and the edge of the node.
    /// Outline updates bypass change detection.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub outline_offset: f32,
    /// The unrounded size of the node as width and height in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub unrounded_size: Vec2,
    /// Resolved border values in physical pixels.
    /// Border updates bypass change detection.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub border: BorderRect,
    /// Resolved border radius values in physical pixels.
    /// Border radius updates bypass change detection.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub border_radius: ResolvedBorderRadius,
    /// Resolved padding values in physical pixels.
    /// Padding updates bypass change detection.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub padding: BorderRect,
    /// Inverse scale factor for this Node.
    /// Multiply physical coordinates by the inverse scale factor to give logical coordinates.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    pub inverse_scale_factor: f32,
}

impl ComputedNode {
    /// The calculated node size as width and height in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn size(&self) -> Vec2 {
        self.size
    }

    /// The calculated node content size as width and height in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn content_size(&self) -> Vec2 {
        self.content_size
    }

    /// Check if the node is empty.
    /// A node is considered empty if it has a zero or negative extent along either of its axes.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.size.x <= 0. || self.size.y <= 0.
    }

    /// The order of the node in the UI layout.
    /// Nodes with a higher stack index are drawn on top of and receive interactions before nodes with lower stack indices.
    ///
    /// Automatically calculated in [`UiSystems::Stack`](super::UiSystems::Stack).
    pub const fn stack_index(&self) -> u32 {
        self.stack_index
    }

    /// The calculated node size as width and height in physical pixels before rounding.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn unrounded_size(&self) -> Vec2 {
        self.unrounded_size
    }

    /// Returns the thickness of the UI node's outline in physical pixels.
    /// If this value is negative or `0.` then no outline will be rendered.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn outline_width(&self) -> f32 {
        self.outline_width
    }

    /// Returns the amount of space between the outline and the edge of the node in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn outline_offset(&self) -> f32 {
        self.outline_offset
    }

    /// Returns the size of the node when including its outline.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn outlined_node_size(&self) -> Vec2 {
        let offset = 2. * (self.outline_offset + self.outline_width);
        Vec2::new(self.size.x + offset, self.size.y + offset)
    }

    /// Returns the border radius for each corner of the outline
    /// An outline's border radius is derived from the node's border-radius
    /// so that the outline wraps the border equally at all points.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn outline_radius(&self) -> ResolvedBorderRadius {
        let outer_distance = self.outline_width + self.outline_offset;
        const fn compute_radius(radius: f32, outer_distance: f32) -> f32 {
            if radius > 0. {
                radius + outer_distance
            } else {
                0.
            }
        }
        ResolvedBorderRadius {
            top_left: compute_radius(self.border_radius.top_left, outer_distance),
            top_right: compute_radius(self.border_radius.top_right, outer_distance),
            bottom_right: compute_radius(self.border_radius.bottom_right, outer_distance),
            bottom_left: compute_radius(self.border_radius.bottom_left, outer_distance),
        }
    }

    /// Returns the thickness of the node's border on each edge in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn border(&self) -> BorderRect {
        self.border
    }

    /// Returns the border radius for each of the node's corners in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn border_radius(&self) -> ResolvedBorderRadius {
        self.border_radius
    }

    /// Returns the inner border radius for each of the node's corners in physical pixels.
    pub fn inner_radius(&self) -> ResolvedBorderRadius {
        fn clamp_corner(r: f32, size: Vec2, offset: Vec2) -> f32 {
            let s = 0.5 * size + offset;
            let sm = s.x.min(s.y);
            r.min(sm)
        }
        let b = Vec4::from((self.border.min_inset, self.border.max_inset));
        let s = self.size() - b.xy() - b.zw();
        ResolvedBorderRadius {
            top_left: clamp_corner(self.border_radius.top_left, s, b.xy()),
            top_right: clamp_corner(self.border_radius.top_right, s, b.zy()),
            bottom_right: clamp_corner(self.border_radius.bottom_left, s, b.xw()),
            bottom_left: clamp_corner(self.border_radius.bottom_right, s, b.zw()),
        }
    }

    /// Returns the thickness of the node's padding on each edge in physical pixels.
    ///
    /// Automatically calculated by [`ui_layout_system`](`super::layout::ui_layout_system`).
    #[inline]
    pub const fn padding(&self) -> BorderRect {
        self.padding
    }

    /// Returns the combined inset on each edge including both padding and border thickness in physical pixels.
    #[inline]
    pub fn content_inset(&self) -> BorderRect {
        let mut content_inset = self.border + self.padding;
        content_inset.max_inset += self.scrollbar_size;
        content_inset
    }

    /// Returns the inverse of the scale factor for this node.
    /// To convert from physical coordinates to logical coordinates multiply by this value.
    #[inline]
    pub const fn inverse_scale_factor(&self) -> f32 {
        self.inverse_scale_factor
    }

    // Returns true if `point` within the node.
    //
    // Matches the sdf function in `ui.wgsl` that is used by the UI renderer to draw rounded rectangles.
    pub fn contains_point(&self, transform: UiGlobalTransform, point: Vec2) -> bool {
        let Some(local_point) = transform
            .try_inverse()
            .map(|transform| transform.transform_point2(point))
        else {
            return false;
        };
        let [top, bottom] = if local_point.x < 0. {
            [self.border_radius.top_left, self.border_radius.bottom_left]
        } else {
            [
                self.border_radius.top_right,
                self.border_radius.bottom_right,
            ]
        };
        let r = if local_point.y < 0. { top } else { bottom };
        let corner_to_point = local_point.abs() - 0.5 * self.size;
        let q = corner_to_point + r;
        let l = q.max(Vec2::ZERO).length();
        let m = q.max_element().min(0.);
        l + m - r < 0.
    }

    /// Transform a point to normalized node space with the center of the node at the origin and the corners at [+/-0.5, +/-0.5]
    pub fn normalize_point(&self, transform: UiGlobalTransform, point: Vec2) -> Option<Vec2> {
        self.size
            .cmpgt(Vec2::ZERO)
            .all()
            .then(|| transform.try_inverse())
            .flatten()
            .map(|transform| transform.transform_point2(point) / self.size)
    }

    /// Resolve the node's clipping rect in local space
    pub fn resolve_clip_rect(
        &self,
        overflow: Overflow,
        overflow_clip_margin: OverflowClipMargin,
    ) -> Rect {
        let mut clip_rect = Rect::from_center_size(Vec2::ZERO, self.size);

        let clip_inset = match overflow_clip_margin.visual_box {
            OverflowClipBox::BorderBox => BorderRect::ZERO,
            OverflowClipBox::ContentBox => self.content_inset(),
            OverflowClipBox::PaddingBox => self.border(),
        };

        clip_rect.min += clip_inset.min_inset;
        clip_rect.max -= clip_inset.max_inset;

        if overflow.x == OverflowAxis::Visible {
            clip_rect.min.x = -f32::INFINITY;
            clip_rect.max.x = f32::INFINITY;
        }
        if overflow.y == OverflowAxis::Visible {
            clip_rect.min.y = -f32::INFINITY;
            clip_rect.max.y = f32::INFINITY;
        }

        clip_rect
    }
}

impl ComputedNode {
    pub const DEFAULT: Self = Self {
        stack_index: 0,
        size: Vec2::ZERO,
        content_size: Vec2::ZERO,
        scrollbar_size: Vec2::ZERO,
        scroll_position: Vec2::ZERO,
        outline_width: 0.,
        outline_offset: 0.,
        unrounded_size: Vec2::ZERO,
        border_radius: ResolvedBorderRadius::ZERO,
        border: BorderRect::ZERO,
        padding: BorderRect::ZERO,
        inverse_scale_factor: 1.,
    };
}

impl Default for ComputedNode {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The scroll position of the node. Values are in logical pixels, increasing from top-left to bottom-right.
///
/// Increasing the x-coordinate causes the scrolled content to visibly move left on the screen, while increasing the y-coordinate causes the scrolled content to move up.
/// This might seem backwards, however what's really happening is that
/// the scroll position is moving the visible "window" in the local coordinate system of the scrolled content -
/// moving the window down causes the content to move up.
///
/// Updating the values of `ScrollPosition` will reposition the children of the node by the offset amount in logical pixels.
/// `ScrollPosition` may be updated by the layout system when a layout change makes a previously valid `ScrollPosition` invalid.
/// Changing this does nothing on a `Node` without setting at least one `OverflowAxis` to `OverflowAxis::Scroll`.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut)]
pub struct ScrollPosition(pub Vec2);

impl ScrollPosition {
    pub const DEFAULT: Self = Self(Vec2::ZERO);
}

impl From<Vec2> for ScrollPosition {
    fn from(value: Vec2) -> Self {
        Self(value)
    }
}

/// Controls whether a UI element ignores its parent's [`ScrollPosition`] along specific axes.
///
/// When an axis is set to `true`, the node will not have the parent’s scroll position applied
/// on that axis. This can be used to keep an element visually fixed along one or both axes
/// even when its parent UI element is scrolled.
#[derive(Component, Debug, Clone, Default, Deref, DerefMut)]
pub struct IgnoreScroll(pub BVec2);

impl From<BVec2> for IgnoreScroll {
    fn from(value: BVec2) -> Self {
        Self(value)
    }
}

/// The base component for UI entities. It describes UI layout and style properties.
///
/// When defining new types of UI entities, require [`Node`] to make them behave like UI nodes.
///
/// Nodes are laid out using Flexbox or block layout.
///
/// See below for general learning resources and for documentation on the individual style properties.
///
/// ### Flexbox
///
/// - [MDN: Basic Concepts of Flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Flexible_Box_Layout/Basic_Concepts_of_Flexbox)
/// - [A Complete Guide To Flexbox](https://css-tricks.com/snippets/css/a-guide-to-flexbox/) by CSS Tricks. This is detailed guide with illustrations and comprehensive written explanation of the different Flexbox properties and how they work.
/// - [Flexbox Froggy](https://flexboxfroggy.com/). An interactive tutorial/game that teaches the essential parts of Flexbox in a fun engaging way.
///
/// # See also
///
/// - [`Interaction`](crate::Interaction) to obtain the interaction state of this node

#[derive(Component, Clone, PartialEq, Debug)]
#[require(
    ComputedNode,
    ComputedUiTargetCamera,
    ComputedUiRenderTargetInfo,
    UiTransform,
    BackgroundColor,
    BorderColor,
    FocusPolicy,
    ScrollPosition,
    Visibility,
    ZIndex
)]
pub struct Node {
    /// Which layout algorithm to use when laying out this node's contents:
    ///   - [`Display::Flex`]: Use the Flexbox layout algorithm
    ///   - [`Display::None`]: Hide this node and perform layout as if it does not exist.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display>
    pub display: Display,

    /// Which part of a Node's box length styles like width and height control
    ///   - [`BoxSizing::BorderBox`]: They refer to the "border box" size (size including padding and border)
    ///   - [`BoxSizing::ContentBox`]: They refer to the "content box" size (size excluding padding and border)
    ///
    /// `BoxSizing::BorderBox` is generally considered more intuitive and is the default in Bevy even though it is not on the web.
    ///
    /// See: <https://developer.mozilla.org/en-US/docs/Web/CSS/box-sizing>
    pub box_sizing: BoxSizing,

    /// Whether a node should be laid out in-flow with, or independently of its siblings:
    ///  - [`PositionType::Relative`]: Layout this node in-flow with other nodes using the usual (flexbox) layout algorithm.
    ///  - [`PositionType::Absolute`]: Layout this node on top and independently of other nodes.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/position>
    pub position_type: PositionType,

    /// Whether overflowing content should be displayed or clipped.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow>
    pub overflow: Overflow,

    /// How much space in logical pixels should be reserved for scrollbars when overflow is set to scroll or auto on an axis.
    pub scrollbar_width: f32,

    /// How the bounds of clipped content should be determined
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-clip-margin>
    pub overflow_clip_margin: OverflowClipMargin,

    /// The horizontal position of the left edge of the node.
    ///  - For relatively positioned nodes, this is relative to the node's position as computed during regular layout.
    ///  - For absolutely positioned nodes, this is relative to the *parent* node's bounding box.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/left>
    pub left: Val,

    /// The horizontal position of the right edge of the node.
    ///  - For relatively positioned nodes, this is relative to the node's position as computed during regular layout.
    ///  - For absolutely positioned nodes, this is relative to the *parent* node's bounding box.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/right>
    pub right: Val,

    /// The vertical position of the top edge of the node.
    ///  - For relatively positioned nodes, this is relative to the node's position as computed during regular layout.
    ///  - For absolutely positioned nodes, this is relative to the *parent* node's bounding box.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/top>
    pub top: Val,

    /// The vertical position of the bottom edge of the node.
    ///  - For relatively positioned nodes, this is relative to the node's position as computed during regular layout.
    ///  - For absolutely positioned nodes, this is relative to the *parent* node's bounding box.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bottom>
    pub bottom: Val,

    /// The ideal width of the node. `width` is used when it is within the bounds defined by `min_width` and `max_width`.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/width>
    pub width: Val,

    /// The ideal height of the node. `height` is used when it is within the bounds defined by `min_height` and `max_height`.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/height>
    pub height: Val,

    /// The minimum width of the node. `min_width` is used if it is greater than `width` and/or `max_width`.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-width>
    pub min_width: Val,

    /// The minimum height of the node. `min_height` is used if it is greater than `height` and/or `max_height`.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-height>
    pub min_height: Val,

    /// The maximum width of the node. `max_width` is used if it is within the bounds defined by `min_width` and `width`.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-width>
    pub max_width: Val,

    /// The maximum height of the node. `max_height` is used if it is within the bounds defined by `min_height` and `height`.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-height>
    pub max_height: Val,

    /// The aspect ratio of the node (defined as `width / height`)
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/aspect-ratio>
    pub aspect_ratio: Option<f32>,

    /// Used to control how each individual item is aligned by default within the space they're given.
    /// - For Flexbox containers, sets default cross axis alignment of the child items.
    ///
    /// This value is overridden if [`AlignSelf`] on the child node is set.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-items>
    pub align_items: AlignItems,

    /// Used to control how the specified item is aligned within the space it's given.
    /// - For Flexbox items, controls cross axis alignment of the item.
    ///
    /// If set to `Auto`, alignment is inherited from the value of [`AlignItems`] set on the parent node.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-self>
    pub align_self: AlignSelf,

    /// Used to control how items are distributed.
    /// - For Flexbox containers, controls alignment of lines if `flex_wrap` is set to [`FlexWrap::Wrap`] and there are multiple lines of items.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-content>
    pub align_content: AlignContent,

    /// Used to control how items are distributed.
    /// - For Flexbox containers, controls alignment of items in the main axis.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content>
    pub justify_content: JustifyContent,

    /// The amount of space around a node outside its border.
    ///
    /// If a percentage value is used, the percentage is calculated based on the width of the parent node.
    ///
    /// # Example
    /// ```
    /// # use bevy_ui::{Node, UiRect, Val};
    /// let node = Node {
    ///     margin: UiRect {
    ///         left: Val::Percent(10.),
    ///         right: Val::Percent(10.),
    ///         top: Val::Percent(15.),
    ///         bottom: Val::Percent(15.)
    ///     },
    ///     ..Default::default()
    /// };
    /// ```
    /// A node with this style and a parent with dimensions of 100px by 300px will have calculated margins of 10px on both left and right edges, and 15px on both top and bottom edges.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin>
    pub margin: UiRect,

    /// The amount of space between the edges of a node and its contents.
    ///
    /// If a percentage value is used, the percentage is calculated based on the width of the parent node.
    ///
    /// # Example
    /// ```
    /// # use bevy_ui::{Node, UiRect, Val};
    /// let node = Node {
    ///     padding: UiRect {
    ///         left: Val::Percent(1.),
    ///         right: Val::Percent(2.),
    ///         top: Val::Percent(3.),
    ///         bottom: Val::Percent(4.)
    ///     },
    ///     ..Default::default()
    /// };
    /// ```
    /// A node with this style and a parent with dimensions of 300px by 100px will have calculated padding of 3px on the left, 6px on the right, 9px on the top and 12px on the bottom.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding>
    pub padding: UiRect,

    /// The amount of space between the margins of a node and its padding.
    ///
    /// If a percentage value is used, the percentage is calculated based on the width of the parent node.
    ///
    /// The size of the node will be expanded if there are constraints that prevent the layout algorithm from placing the border within the existing node boundary.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
    pub border: UiRect,

    /// Used to add rounded corners to a UI node. You can set a UI node to have uniformly
    /// rounded corners or specify different radii for each corner. If a given radius exceeds half
    /// the length of the smallest dimension between the node's height or width, the radius will
    /// calculated as half the smallest dimension.
    ///
    /// Elliptical nodes are not supported yet. Percentage values are based on the node's smallest
    /// dimension, either width or height.
    ///
    /// # Example
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ui::prelude::*;
    /// # use bevy_color::palettes::basic::{BLUE};
    /// fn setup_ui(mut commands: Commands) {
    ///     commands.spawn((
    ///         Node {
    ///             width: Val::Px(100.),
    ///             height: Val::Px(100.),
    ///             border: UiRect::all(Val::Px(2.)),
    ///             border_radius: BorderRadius::new(
    ///                 // top left
    ///                 Val::Px(10.),
    ///                 // top right
    ///                 Val::Px(20.),
    ///                 // bottom right
    ///                 Val::Px(30.),
    ///                 // bottom left
    ///                 Val::Px(40.),
    ///             ),
    ///             ..Default::default()
    ///         },
    ///         BackgroundColor(BLUE.into()),
    ///     ));
    /// }
    /// ```
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius>
    pub border_radius: BorderRadius,

    /// Whether a Flexbox container should be a row or a column.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-direction>
    pub flex_direction: FlexDirection,

    /// Whether a Flexbox container should wrap its contents onto multiple lines if they overflow.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-wrap>
    pub flex_wrap: FlexWrap,

    /// Defines how much a flexbox item should grow if there's space available. Defaults to 0 (don't grow at all).
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-grow>
    pub flex_grow: f32,

    /// Defines how much a flexbox item should shrink if there's not enough space available. Defaults to 1.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-shrink>
    pub flex_shrink: f32,

    /// The initial length of a flexbox in the main axis, before flex growing/shrinking properties are applied.
    ///
    /// `flex_basis` overrides `width` (if the main axis is horizontal) or `height` (if the main axis is vertical) when both are set, but it obeys the constraints defined by `min_width`/`min_height` and `max_width`/`max_height`.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-basis>
    pub flex_basis: Val,

    /// The size of the gutters between items in a vertical flexbox layout or between wrapped lines in a horizontal flexbox layout.
    ///
    /// Note: Values of `Val::Auto` are not valid and are treated as zero.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/row-gap>
    pub row_gap: Val,

    /// The size of the gutters between items in a horizontal flexbox layout or between wrapped lines in a vertical flexbox layout.
    ///
    /// Note: Values of `Val::Auto` are not valid and are treated as zero.
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-gap>
    pub column_gap: Val,
}

impl Node {
    pub const DEFAULT: Self = Self {
        display: Display::DEFAULT,
        box_sizing: BoxSizing::DEFAULT,
        position_type: PositionType::DEFAULT,
        left: Val::Auto,
        right: Val::Auto,
        top: Val::Auto,
        bottom: Val::Auto,
        flex_direction: FlexDirection::DEFAULT,
        flex_wrap: FlexWrap::DEFAULT,
        align_items: AlignItems::DEFAULT,
        align_self: AlignSelf::DEFAULT,
        align_content: AlignContent::DEFAULT,
        justify_content: JustifyContent::DEFAULT,
        margin: UiRect::DEFAULT,
        padding: UiRect::DEFAULT,
        border: UiRect::DEFAULT,
        border_radius: BorderRadius::DEFAULT,
        flex_grow: 0.0,
        flex_shrink: 1.0,
        flex_basis: Val::Auto,
        width: Val::Auto,
        height: Val::Auto,
        min_width: Val::Auto,
        min_height: Val::Auto,
        max_width: Val::Auto,
        max_height: Val::Auto,
        aspect_ratio: None,
        overflow: Overflow::DEFAULT,
        overflow_clip_margin: OverflowClipMargin::DEFAULT,
        scrollbar_width: 0.,
        row_gap: Val::ZERO,
        column_gap: Val::ZERO,
    };
}

impl Default for Node {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Used to control how each individual item is aligned by default within the space they're given.
/// - For Flexbox containers, sets default cross axis alignment of the child items.
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-items>
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AlignItems {
    /// The items are packed in their default position as if no alignment was applied.
    Default,
    /// The items are packed towards the start of the axis.
    Start,
    /// The items are packed towards the end of the axis.
    End,
    /// The items are packed towards the start of the axis, unless the flex direction is reversed;
    /// then they are packed towards the end of the axis.
    FlexStart,
    /// The items are packed towards the end of the axis, unless the flex direction is reversed;
    /// then they are packed towards the start of the axis.
    FlexEnd,
    /// The items are packed along the center of the axis.
    Center,
    /// The items are packed such that their baselines align.
    Baseline,
    /// The items are stretched to fill the space they're given.
    Stretch,
}

impl AlignItems {
    pub const DEFAULT: Self = Self::Default;
}

impl Default for AlignItems {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Used to control how the specified item is aligned within the space it's given.
/// - For Flexbox items, controls cross axis alignment of the item.
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-self>
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AlignSelf {
    /// Use the parent node's [`AlignItems`] value to determine how this item should be aligned.
    Auto,
    /// This item will be aligned with the start of the axis.
    Start,
    /// This item will be aligned with the end of the axis.
    End,
    /// This item will be aligned with the start of the axis, unless the flex direction is reversed;
    /// then it will be aligned with the end of the axis.
    FlexStart,
    /// This item will be aligned with the end of the axis, unless the flex direction is reversed;
    /// then it will be aligned with the start of the axis.
    FlexEnd,
    /// This item will be aligned along the center of the axis.
    Center,
    /// This item will be aligned at the baseline.
    Baseline,
    /// This item will be stretched to fill the container.
    Stretch,
}

impl AlignSelf {
    pub const DEFAULT: Self = Self::Auto;
}

impl Default for AlignSelf {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Used to control how items are distributed.
/// - For Flexbox containers, controls alignment of lines if `flex_wrap` is set to [`FlexWrap::Wrap`] and there are multiple lines of items.
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-content>
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AlignContent {
    /// The items are packed in their default position as if no alignment was applied.
    Default,
    /// The items are packed towards the start of the axis.
    Start,
    /// The items are packed towards the end of the axis.
    End,
    /// The items are packed towards the start of the axis, unless the flex direction is reversed;
    /// then the items are packed towards the end of the axis.
    FlexStart,
    /// The items are packed towards the end of the axis, unless the flex direction is reversed;
    /// then the items are packed towards the start of the axis.
    FlexEnd,
    /// The items are packed along the center of the axis.
    Center,
    /// The items are stretched to fill the container along the axis.
    Stretch,
    /// The items are distributed such that the gap between any two items is equal.
    SpaceBetween,
    /// The items are distributed such that the gap between and around any two items is equal.
    SpaceEvenly,
    /// The items are distributed such that the gap between and around any two items is equal, with half-size gaps on either end.
    SpaceAround,
}

impl AlignContent {
    pub const DEFAULT: Self = Self::Default;
}

impl Default for AlignContent {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Used to control how items are distributed.
/// - For Flexbox containers, controls alignment of items in the main axis.
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content>
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum JustifyContent {
    /// The items are packed in their default position as if no alignment was applied.
    Default,
    /// The items are packed towards the start of the axis.
    Start,
    /// The items are packed towards the end of the axis.
    End,
    /// The items are packed towards the start of the axis, unless the flex direction is reversed;
    /// then the items are packed towards the end of the axis.
    FlexStart,
    /// The items are packed towards the end of the axis, unless the flex direction is reversed;
    /// then the items are packed towards the start of the axis.
    FlexEnd,
    /// The items are packed along the center of the axis.
    Center,
    /// The items are stretched to fill the container along the axis.
    Stretch,
    /// The items are distributed such that the gap between any two items is equal.
    SpaceBetween,
    /// The items are distributed such that the gap between and around any two items is equal.
    SpaceEvenly,
    /// The items are distributed such that the gap between and around any two items is equal, with half-size gaps on either end.
    SpaceAround,
}

impl JustifyContent {
    pub const DEFAULT: Self = Self::Default;
}

impl Default for JustifyContent {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines the layout model used by this node.
///
/// Part of the [`Node`] component.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Display {
    /// Use Flexbox layout model to determine the position of this [`Node`]'s children.
    Flex,
    /// Use CSS Block layout model to determine the position of this [`Node`]'s children.
    Block,
    /// Use no layout, don't render this node and its children.
    ///
    /// If you want to hide a node and its children,
    /// but keep its layout in place, set its [`Visibility`] component instead.
    None,
}

impl Display {
    pub const DEFAULT: Self = Self::Flex;
}

impl Default for Display {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Which part of a Node's box length styles like width and height control
///
/// See: <https://developer.mozilla.org/en-US/docs/Web/CSS/box-sizing>
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BoxSizing {
    /// Length styles like width and height refer to the "border box" size (size including padding and border)
    BorderBox,
    /// Length styles like width and height refer to the "content box" size (size excluding padding and border)
    ContentBox,
}

impl BoxSizing {
    pub const DEFAULT: Self = Self::BorderBox;
}

impl Default for BoxSizing {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines how flexbox items are ordered within a flexbox
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FlexDirection {
    /// Same way as text direction along the main axis.
    Row,
    /// Flex from top to bottom.
    Column,
    /// Opposite way as text direction along the main axis.
    RowReverse,
    /// Flex from bottom to top.
    ColumnReverse,
}

impl FlexDirection {
    pub const DEFAULT: Self = Self::Row;
}

impl Default for FlexDirection {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Whether to show or hide overflowing items
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Overflow {
    /// Whether to show or clip overflowing items on the x axis
    pub x: OverflowAxis,
    /// Whether to show or clip overflowing items on the y axis
    pub y: OverflowAxis,
}

impl Overflow {
    pub const DEFAULT: Self = Self {
        x: OverflowAxis::DEFAULT,
        y: OverflowAxis::DEFAULT,
    };

    /// Show overflowing items on both axes
    pub const fn visible() -> Self {
        Self {
            x: OverflowAxis::Visible,
            y: OverflowAxis::Visible,
        }
    }

    /// Clip overflowing items on both axes
    pub const fn clip() -> Self {
        Self {
            x: OverflowAxis::Clip,
            y: OverflowAxis::Clip,
        }
    }

    /// Hide overflowing items on both axes by influencing layout and then clipping
    pub const fn hidden() -> Self {
        Self {
            x: OverflowAxis::Hidden,
            y: OverflowAxis::Hidden,
        }
    }

    /// Overflow is visible on both axes
    pub const fn is_visible(&self) -> bool {
        self.x.is_visible() && self.y.is_visible()
    }

    pub const fn scroll() -> Self {
        Self {
            x: OverflowAxis::Scroll,
            y: OverflowAxis::Scroll,
        }
    }

    /// Scroll overflowing items on the x axis
    pub const fn scroll_x() -> Self {
        Self {
            x: OverflowAxis::Scroll,
            y: OverflowAxis::Visible,
        }
    }

    /// Scroll overflowing items on the y axis
    pub const fn scroll_y() -> Self {
        Self {
            x: OverflowAxis::Visible,
            y: OverflowAxis::Scroll,
        }
    }
}

impl Default for Overflow {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Whether to show or hide overflowing items
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OverflowAxis {
    /// Show overflowing items.
    Visible,
    /// Hide overflowing items by clipping.
    Clip,
    /// Hide overflowing items by influencing layout and then clipping.
    Hidden,
    /// Scroll overflowing items.
    Scroll,
}

impl OverflowAxis {
    pub const DEFAULT: Self = Self::Visible;

    /// Overflow is visible on this axis
    pub const fn is_visible(&self) -> bool {
        matches!(self, Self::Visible)
    }
}

impl Default for OverflowAxis {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The bounds of the visible area when a UI node is clipped.
#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct OverflowClipMargin {
    /// Visible unclipped area
    pub visual_box: OverflowClipBox,
    /// Width of the margin on each edge of the visual box in logical pixels.
    /// The width of the margin will be zero if a negative value is set.
    pub margin: f32,
}

impl OverflowClipMargin {
    pub const DEFAULT: Self = Self {
        visual_box: OverflowClipBox::PaddingBox,
        margin: 0.,
    };
}

/// Used to determine the bounds of the visible area when a UI node is clipped.
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
pub enum OverflowClipBox {
    /// Clip any content that overflows outside the content box
    ContentBox,
    /// Clip any content that overflows outside the padding box
    #[default]
    PaddingBox,
    /// Clip any content that overflows outside the border box
    BorderBox,
}

/// The strategy used to position this node
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PositionType {
    /// Relative to all other nodes with the [`PositionType::Relative`] value.
    Relative,
    /// Independent of all other nodes, but relative to its parent node.
    Absolute,
}

impl PositionType {
    pub const DEFAULT: Self = Self::Relative;
}

impl Default for PositionType {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines if flexbox items appear on a single line or on multiple lines
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FlexWrap {
    /// Single line, will overflow if needed.
    NoWrap,
    /// Multiple lines, if needed.
    Wrap,
    /// Same as [`FlexWrap::Wrap`] but new lines will appear before the previous one.
    WrapReverse,
}

impl FlexWrap {
    pub const DEFAULT: Self = Self::NoWrap;
}

impl Default for FlexWrap {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The background color of the node
///
/// This serves as the "fill" color.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct BackgroundColor(pub Color);

impl BackgroundColor {
    /// Background color is transparent by default.
    pub const DEFAULT: Self = Self(Color::NONE);
}

impl Default for BackgroundColor {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<T: Into<Color>> From<T> for BackgroundColor {
    fn from(color: T) -> Self {
        Self(color.into())
    }
}

/// The border color of the UI node.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct BorderColor {
    pub top: Color,
    pub right: Color,
    pub bottom: Color,
    pub left: Color,
}

impl<T: Into<Color>> From<T> for BorderColor {
    fn from(color: T) -> Self {
        Self::all(color.into())
    }
}

impl BorderColor {
    /// Border color is transparent by default.
    pub const DEFAULT: Self = BorderColor {
        top: Color::NONE,
        right: Color::NONE,
        bottom: Color::NONE,
        left: Color::NONE,
    };

    /// Helper to create a `BorderColor` struct with all borders set to the given color
    #[inline]
    pub fn all(color: impl Into<Color>) -> Self {
        let color = color.into();
        Self {
            top: color,
            bottom: color,
            left: color,
            right: color,
        }
    }

    /// Helper to set all border colors to a given color.
    pub fn set_all(&mut self, color: impl Into<Color>) -> &mut Self {
        let color: Color = color.into();
        self.top = color;
        self.bottom = color;
        self.left = color;
        self.right = color;
        self
    }

    /// Check if all contained border colors are transparent
    pub fn is_fully_transparent(&self) -> bool {
        self.top.is_fully_transparent()
            && self.bottom.is_fully_transparent()
            && self.left.is_fully_transparent()
            && self.right.is_fully_transparent()
    }
}

impl Default for BorderColor {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Component, Copy, Clone, Default, Debug, PartialEq)]
/// The [`Outline`] component adds an outline outside the edge of a UI node.
/// Outlines do not take up space in the layout.
///
/// To add an [`Outline`] to a ui node you can spawn a `(Node, Outline)` tuple bundle:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ui::prelude::*;
/// # use bevy_color::palettes::basic::{RED, BLUE};
/// fn setup_ui(mut commands: Commands) {
///     commands.spawn((
///         Node {
///             width: Val::Px(100.),
///             height: Val::Px(100.),
///             ..Default::default()
///         },
///         BackgroundColor(BLUE.into()),
///         Outline::new(Val::Px(10.), Val::ZERO, RED.into())
///     ));
/// }
/// ```
///
/// [`Outline`] components can also be added later to existing UI nodes:
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ui::prelude::*;
/// # use bevy_color::Color;
/// fn outline_hovered_button_system(
///     mut commands: Commands,
///     mut node_query: Query<(Entity, &Interaction, Option<&mut Outline>), Changed<Interaction>>,
/// ) {
///     for (entity, interaction, mut maybe_outline) in node_query.iter_mut() {
///         let outline_color =
///             if matches!(*interaction, Interaction::Hovered) {
///                 Color::WHITE
///             } else {
///                 Color::NONE
///             };
///         if let Some(mut outline) = maybe_outline {
///             outline.color = outline_color;
///         } else {
///             commands.entity(entity).insert(Outline::new(Val::Px(10.), Val::ZERO, outline_color));
///         }
///     }
/// }
/// ```
/// Inserting and removing an [`Outline`] component repeatedly will result in table moves, so it is generally preferable to
/// set `Outline::color` to [`Color::NONE`] to hide an outline.
pub struct Outline {
    /// The width of the outline.
    ///
    /// Percentage `Val` values are resolved based on the width of the outlined [`Node`].
    pub width: Val,
    /// The amount of space between a node's outline the edge of the node.
    ///
    /// Percentage `Val` values are resolved based on the width of the outlined [`Node`].
    pub offset: Val,
    /// The color of the outline.
    ///
    /// If you are frequently toggling outlines for a UI node on and off it is recommended to set [`Color::NONE`] to hide the outline.
    /// This avoids the table moves that would occur from the repeated insertion and removal of the `Outline` component.
    pub color: Color,
}

impl Outline {
    /// Create a new outline
    pub const fn new(width: Val, offset: Val, color: Color) -> Self {
        Self {
            width,
            offset,
            color,
        }
    }
}

/// The calculated clip of the node
#[derive(Component, Default, Copy, Clone, Debug)]
pub struct CalculatedClip {
    /// The rect of the clip
    pub clip: Rect,
}

/// UI node entities with this component will ignore any clipping rect they inherit,
/// the node will not be clipped regardless of its ancestors' `Overflow` setting.
#[derive(Component)]
pub struct OverrideClip;

#[expect(
    rustdoc::redundant_explicit_links,
    reason = "To go around the `<code>` limitations, we put the link twice so we're \
sure it's recognized as a markdown link."
)]
/// Indicates that this [`Node`] entity's front-to-back ordering is not controlled solely
/// by its location in the UI hierarchy. A node with a higher z-index will appear on top
/// of sibling nodes with a lower z-index.
///
/// UI nodes that have the same z-index will appear according to the order in which they
/// appear in the UI hierarchy. In such a case, the last node to be added to its parent
/// will appear in front of its siblings.
///
/// Nodes without this component will be treated as if they had a value of
/// <code>[ZIndex][ZIndex]\(0\)</code>.
///
/// Use [`GlobalZIndex`] if you need to order separate UI hierarchies or nodes that are
/// not siblings in a given UI hierarchy.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ZIndex(pub i32);

/// `GlobalZIndex` allows a [`Node`] entity anywhere in the UI hierarchy to escape the implicit draw ordering of the UI's layout tree and
/// be rendered above or below other UI nodes.
/// Nodes with a `GlobalZIndex` of greater than 0 will be drawn on top of nodes without a `GlobalZIndex` or nodes with a lower `GlobalZIndex`.
/// Nodes with a `GlobalZIndex` of less than 0 will be drawn below nodes without a `GlobalZIndex` or nodes with a greater `GlobalZIndex`.
///
/// If two Nodes have the same `GlobalZIndex`, the node with the greater [`ZIndex`] will be drawn on top.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalZIndex(pub i32);

/// Used to add rounded corners to a UI node. You can set a UI node to have uniformly
/// rounded corners or specify different radii for each corner. If a given radius exceeds half
/// the length of the smallest dimension between the node's height or width, the radius will
/// calculated as half the smallest dimension.
///
/// Elliptical nodes are not supported yet. Percentage values are based on the node's smallest
/// dimension, either width or height.
///
/// # Example
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # use bevy_ui::prelude::*;
/// # use bevy_color::palettes::basic::{BLUE};
/// fn setup_ui(mut commands: Commands) {
///     commands.spawn((
///         Node {
///             width: Val::Px(100.),
///             height: Val::Px(100.),
///             border: UiRect::all(Val::Px(2.)),
///             border_radius: BorderRadius::new(
///                 // top left
///                 Val::Px(10.),
///                 // top right
///                 Val::Px(20.),
///                 // bottom right
///                 Val::Px(30.),
///                 // bottom left
///                 Val::Px(40.),
///             ),
///             ..Default::default()
///         },
///         BackgroundColor(BLUE.into()),
///     ));
/// }
/// ```
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius>
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BorderRadius {
    pub top_left: Val,
    pub top_right: Val,
    pub bottom_right: Val,
    pub bottom_left: Val,
}

impl Default for BorderRadius {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl BorderRadius {
    pub const DEFAULT: Self = Self::ZERO;

    /// Zero curvature. All the corners will be right-angled.
    pub const ZERO: Self = Self::all(Val::Px(0.));

    /// Maximum curvature. The UI Node will take a capsule shape or circular if width and height are equal.
    pub const MAX: Self = Self::all(Val::Px(f32::MAX));

    #[inline]
    /// Set all four corners to the same curvature.
    pub const fn all(radius: Val) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_left: radius,
            bottom_right: radius,
        }
    }

    #[inline]
    pub const fn new(top_left: Val, top_right: Val, bottom_right: Val, bottom_left: Val) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    #[inline]
    /// Sets the radii to logical pixel values.
    pub const fn px(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left: Val::Px(top_left),
            top_right: Val::Px(top_right),
            bottom_right: Val::Px(bottom_right),
            bottom_left: Val::Px(bottom_left),
        }
    }

    #[inline]
    /// Sets the radii to percentage values.
    pub const fn percent(
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) -> Self {
        Self {
            top_left: Val::Percent(top_left),
            top_right: Val::Percent(top_right),
            bottom_right: Val::Percent(bottom_right),
            bottom_left: Val::Percent(bottom_left),
        }
    }

    /// Resolve the border radius for a single corner from the given context values.
    /// Returns the radius of the corner in physical pixels.
    pub const fn resolve_single_corner(
        radius: Val,
        scale_factor: f32,
        min_length: f32,
        viewport_size: Vec2,
    ) -> f32 {
        if let Ok(radius) = radius.resolve(scale_factor, min_length, viewport_size) {
            radius.clamp(0., 0.5 * min_length)
        } else {
            0.
        }
    }

    /// Resolve the border radii for the corners from the given context values.
    /// Returns the radii of the each corner in physical pixels.
    pub const fn resolve(
        &self,
        scale_factor: f32,
        node_size: Vec2,
        viewport_size: Vec2,
    ) -> ResolvedBorderRadius {
        let length = node_size.x.min(node_size.y);
        ResolvedBorderRadius {
            top_left: Self::resolve_single_corner(
                self.top_left,
                scale_factor,
                length,
                viewport_size,
            ),
            top_right: Self::resolve_single_corner(
                self.top_right,
                scale_factor,
                length,
                viewport_size,
            ),
            bottom_left: Self::resolve_single_corner(
                self.bottom_left,
                scale_factor,
                length,
                viewport_size,
            ),
            bottom_right: Self::resolve_single_corner(
                self.bottom_right,
                scale_factor,
                length,
                viewport_size,
            ),
        }
    }
}

/// Represents the resolved border radius values for a UI node.
///
/// The values are in physical pixels.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ResolvedBorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl ResolvedBorderRadius {
    pub const ZERO: Self = Self {
        top_left: 0.,
        top_right: 0.,
        bottom_right: 0.,
        bottom_left: 0.,
    };
}

impl From<ResolvedBorderRadius> for [f32; 4] {
    fn from(radius: ResolvedBorderRadius) -> Self {
        [
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
        ]
    }
}

#[derive(Component, Copy, Clone, Debug, PartialEq)]
/// This component can be added to any UI node to modify its layout behavior.
pub struct LayoutConfig {
    /// If set to true the coordinates for this node and its descendents will be rounded to the nearest physical pixel.
    /// This can help prevent visual artifacts like blurry images or semi-transparent edges that can occur with sub-pixel positioning.
    ///
    /// Defaults to true.
    pub use_rounding: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self { use_rounding: true }
    }
}

/// Indicates that this root [`Node`] entity should be rendered to a specific camera.
///
/// UI then will be laid out respecting the camera's viewport and scale factor, and
/// rendered to this camera's [`bevy_camera::RenderTarget`].
///
/// Setting this component on a non-root node will have no effect. It will be overridden
/// by the root node's component.
///
/// Root node's without an explicit [`UiTargetCamera`] will be rendered to the default UI camera,
/// which is either a single camera with the [`IsDefaultUiCamera`] marker component or the highest
/// order camera targeting the primary window.
#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct UiTargetCamera(pub Entity);

impl UiTargetCamera {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

/// Marker used to identify default cameras, they will have priority over the [`PrimaryWindow`] camera.
///
/// This is useful if the [`PrimaryWindow`] has two cameras, one of them used
/// just for debug purposes and the user wants a way to choose the default [`Camera`]
/// without having to add a [`UiTargetCamera`] to the root node.
///
/// Another use is when the user wants the Ui to be in another window by default,
/// all that is needed is to place this component on the camera
///
/// ```
/// # use bevy_ui::prelude::*;
/// # use bevy_ecs::prelude::Commands;
/// # use bevy_camera::{Camera, Camera2d, RenderTarget};
/// # use bevy_window::{Window, WindowRef};
///
/// fn spawn_camera(mut commands: Commands) {
///     let another_window = commands.spawn(Window {
///         title: String::from("Another window"),
///         ..Default::default()
///     }).id();
///     commands.spawn((
///         Camera2d,
///         Camera {
///             ..Default::default()
///         },
///         RenderTarget::Window(WindowRef::Entity(another_window)),
///         // We add the Marker here so all Ui will spawn in
///         // another window if no UiTargetCamera is specified
///         IsDefaultUiCamera
///     ));
/// }
/// ```
#[derive(Component, Default)]
pub struct IsDefaultUiCamera;

#[derive(SystemParam)]
pub struct DefaultUiCamera<'w, 's> {
    cameras: Query<'w, 's, (Entity, &'static Camera, &'static RenderTarget)>,
    default_cameras: Query<'w, 's, Entity, (With<Camera>, With<IsDefaultUiCamera>)>,
    primary_window: Query<'w, 's, Entity, With<PrimaryWindow>>,
}

impl<'w, 's> DefaultUiCamera<'w, 's> {
    pub fn get(&self) -> Option<Entity> {
        self.default_cameras.single().ok().or_else(|| {
            // If there isn't a single camera and the query isn't empty, there is two or more cameras queried.
            if !self.default_cameras.is_empty() {
                once!(warn!("Two or more Entities with IsDefaultUiCamera found when only one Camera with this marker is allowed."));
            }
            self.cameras
                .iter()
                .filter(|(_, _, render_target)| match render_target {
                    RenderTarget::Window(WindowRef::Primary) => true,
                    RenderTarget::Window(WindowRef::Entity(w)) => {
                        self.primary_window.get(*w).is_ok()
                    }
                    _ => false,
                })
                .max_by_key(|(e, c, _)| (c.order, *e))
                .map(|(e, _, _)| e)
        })
    }
}

/// Derived information about the camera target for this UI node.
///
/// Updated in [`UiSystems::Prepare`](crate::UiSystems::Prepare) by [`propagate_ui_target_cameras`](crate::update::propagate_ui_target_cameras)
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ComputedUiTargetCamera {
    pub(crate) camera: Entity,
}

impl Default for ComputedUiTargetCamera {
    fn default() -> Self {
        Self {
            camera: Entity::PLACEHOLDER,
        }
    }
}

impl ComputedUiTargetCamera {
    /// Returns the id of the target camera for this UI node.
    pub fn get(&self) -> Option<Entity> {
        Some(self.camera).filter(|&entity| entity != Entity::PLACEHOLDER)
    }
}

/// Derived information about the render target for this UI node.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ComputedUiRenderTargetInfo {
    /// The scale factor of the target camera's render target.
    pub(crate) scale_factor: f32,
    /// The size of the target camera's viewport in physical pixels.
    pub(crate) physical_size: UVec2,
}

impl Default for ComputedUiRenderTargetInfo {
    fn default() -> Self {
        Self {
            scale_factor: 1.,
            physical_size: UVec2::ZERO,
        }
    }
}

impl ComputedUiRenderTargetInfo {
    pub const fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Returns the size of the target camera's viewport in physical pixels.
    pub const fn physical_size(&self) -> UVec2 {
        self.physical_size
    }

    /// Returns the size of the target camera's viewport in logical pixels.
    pub fn logical_size(&self) -> Vec2 {
        self.physical_size.as_vec2() / self.scale_factor
    }
}
