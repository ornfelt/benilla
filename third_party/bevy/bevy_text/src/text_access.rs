use bevy_color::Color;
use bevy_ecs::{
    component::Mutable,
    prelude::*,
    system::{Query, SystemParam},
};

use crate::{LineHeight, TextColor, TextFont, TextSpan};

/// Helper trait for using the [`TextReader`] system param.
pub trait TextSpanAccess: Component<Mutability = Mutable> {
    /// Gets the text span's string.
    fn read_span(&self) -> &str;
}

/// Helper trait for the root text component in a text block.
pub trait TextRoot: TextSpanAccess + From<String> {}

/// Scratch buffer used to store intermediate state when iterating over text spans.
#[derive(Resource, Default)]
pub struct TextIterScratch {
    stack: Vec<(&'static Children, usize)>,
}

impl TextIterScratch {
    fn take<'a>(&mut self) -> Vec<(&'a Children, usize)> {
        core::mem::take(&mut self.stack)
            .into_iter()
            .map(|_| -> (&Children, usize) { unreachable!() })
            .collect()
    }

    fn recover(&mut self, mut stack: Vec<(&Children, usize)>) {
        stack.clear();
        self.stack = stack
            .into_iter()
            .map(|_| -> (&'static Children, usize) { unreachable!() })
            .collect();
    }
}

/// System parameter for reading text spans in a text block.
///
/// `R` is the root text component.
#[derive(SystemParam)]
pub struct TextReader<'w, 's, R: TextRoot> {
    // This is a local to avoid system ambiguities when TextReaders run in parallel.
    scratch: Local<'s, TextIterScratch>,
    roots: Query<
        'w,
        's,
        (
            &'static R,
            &'static TextFont,
            &'static TextColor,
            &'static LineHeight,
            Option<&'static Children>,
        ),
    >,
    spans: Query<
        'w,
        's,
        (
            &'static TextSpan,
            &'static TextFont,
            &'static TextColor,
            &'static LineHeight,
            Option<&'static Children>,
        ),
    >,
}

impl<'w, 's, R: TextRoot> TextReader<'w, 's, R> {
    /// Returns an iterator over text spans in a text block, starting with the root entity.
    pub fn iter(&mut self, root_entity: Entity) -> TextSpanIter<'_, R> {
        let stack = self.scratch.take();

        TextSpanIter {
            scratch: &mut self.scratch,
            root_entity: Some(root_entity),
            stack,
            roots: &self.roots,
            spans: &self.spans,
        }
    }
}

/// Iterator returned by [`TextReader::iter`].
///
/// Iterates all spans in a text block according to hierarchy traversal order.
/// Does *not* flatten interspersed ghost nodes. Only contiguous spans are traversed.
// TODO: Use this iterator design in UiChildrenIter to reduce allocations.
pub struct TextSpanIter<'a, R: TextRoot> {
    scratch: &'a mut TextIterScratch,
    root_entity: Option<Entity>,
    /// Stack of (children, next index into children).
    stack: Vec<(&'a Children, usize)>,
    roots: &'a Query<
        'a,
        'a,
        (
            &'static R,
            &'static TextFont,
            &'static TextColor,
            &'static LineHeight,
            Option<&'static Children>,
        ),
    >,
    spans: &'a Query<
        'a,
        'a,
        (
            &'static TextSpan,
            &'static TextFont,
            &'static TextColor,
            &'static LineHeight,
            Option<&'static Children>,
        ),
    >,
}

impl<'a, R: TextRoot> Iterator for TextSpanIter<'a, R> {
    /// Item = (entity in text block, hierarchy depth in the block, span text, span style).
    type Item = (Entity, usize, &'a str, &'a TextFont, Color, LineHeight);
    fn next(&mut self) -> Option<Self::Item> {
        // Root
        if let Some(root_entity) = self.root_entity.take() {
            if let Ok((text, text_font, color, line_height, maybe_children)) =
                self.roots.get(root_entity)
            {
                if let Some(children) = maybe_children {
                    self.stack.push((children, 0));
                }
                return Some((
                    root_entity,
                    0,
                    text.read_span(),
                    text_font,
                    color.0,
                    *line_height,
                ));
            }
            return None;
        }

        // Span
        loop {
            let (children, idx) = self.stack.last_mut()?;

            loop {
                let Some(child) = children.get(*idx) else {
                    break;
                };

                // Increment to prep the next entity in this stack level.
                *idx += 1;

                let entity = *child;
                let Ok((span, text_font, color, line_height, maybe_children)) =
                    self.spans.get(entity)
                else {
                    continue;
                };

                let depth = self.stack.len();
                if let Some(children) = maybe_children {
                    self.stack.push((children, 0));
                }
                return Some((
                    entity,
                    depth,
                    span.read_span(),
                    text_font,
                    color.0,
                    *line_height,
                ));
            }

            // All children at this stack entry have been iterated.
            self.stack.pop();
        }
    }
}

impl<'a, R: TextRoot> Drop for TextSpanIter<'a, R> {
    fn drop(&mut self) {
        // Return the internal stack.
        let stack = core::mem::take(&mut self.stack);
        self.scratch.recover(stack);
    }
}
