//! A stripped down version of petgraph's `UnGraph`.
//!
//! - Index types always use `u32`.
//! - Edge iteration order after serialization/deserialization is preserved.
//! - Fewer iterators and helpers, and a few new ones.

use core::cmp::max;
use core::ops::{Index, IndexMut};

use derive_more::derive::From;

/// A node identifier for a graph structure.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, From)]
pub struct NodeIndex(pub u32);

impl NodeIndex {
    /// A special index used to denote the final node,
    /// for example at the end of an adjacency list.
    ///
    /// Equivalent to `NodeIndex(u32::MAX)`.
    pub const END: NodeIndex = NodeIndex(u32::MAX);

    /// Returns the inner `u32` value as a `usize`.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// An edge identifier for a graph structure.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, From)]
pub struct EdgeIndex(pub u32);

impl EdgeIndex {
    /// A special index used to denote the abscence of an edge,
    /// for example at the end of an adjacency list.
    ///
    /// Equivalent to `EdgeIndex(u32::MAX)`.
    pub const END: EdgeIndex = EdgeIndex(u32::MAX);

    /// Returns the inner `u32` value as a `usize`.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The direction of a graph edge.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[repr(usize)]
pub enum EdgeDirection {
    /// An `Outgoing` edge is an outward edge *from* the current node.
    Outgoing = 0,
    /// An `Incoming` edge is an inbound edge *to* the current node.
    Incoming = 1,
}

impl EdgeDirection {
    /// The two possible directions for an edge.
    pub const ALL: [EdgeDirection; 2] = [EdgeDirection::Outgoing, EdgeDirection::Incoming];
}

/// The node type for a graph structure.
#[derive(Clone, Copy, Debug)]
pub struct Node<N> {
    /// Associated node data.
    pub weight: N,
    /// Next edge in outgoing and incoming edge lists.
    pub(super) next: [EdgeIndex; 2],
}

/// The edge type for a graph structure.
#[derive(Clone, Copy, Debug)]
pub struct Edge<E> {
    /// Associated edge data.
    pub weight: E,
    /// Next edge in outgoing and incoming edge lists.
    pub(super) next: [EdgeIndex; 2],
    /// Start and End node index
    pub(super) node: [NodeIndex; 2],
}

/// A graph with undirected edges.
///
/// The graph can invalidate node or edge indices when items are removed.
/// If you need stable indices, use [`StableUnGraph`](super::stable_graph::StableUnGraph).
#[derive(Clone, Debug)]
pub struct UnGraph<N, E> {
    pub(super) nodes: Vec<Node<N>>,
    pub(super) edges: Vec<Edge<E>>,
}

impl<N, E> Default for UnGraph<N, E> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

pub(super) enum Pair<T> {
    Both(T, T),
    One(T),
    None,
}

/// Get mutable references at index `a` and `b`.
pub(super) fn index_twice<T>(arr: &mut [T], a: usize, b: usize) -> Pair<&mut T> {
    if max(a, b) >= arr.len() {
        Pair::None
    } else if a == b {
        Pair::One(&mut arr[max(a, b)])
    } else {
        // safe because `a` and `b` are in bounds and distinct.
        unsafe {
            let ar = &mut *(arr.get_unchecked_mut(a) as *mut _);
            let br = &mut *(arr.get_unchecked_mut(b) as *mut _);
            Pair::Both(ar, br)
        }
    }
}

impl<N, E> UnGraph<N, E> {
    /// Adds a node (also called vertex) with associated data `weight` to the graph.
    ///
    /// Computes in **O(1)** time.
    ///
    /// Returns the index of the new node.
    ///
    /// # Panics
    ///
    /// Panics if the graph is at the maximum number of nodes.
    pub fn add_node(&mut self, weight: N) -> NodeIndex {
        let node = Node {
            weight,
            next: [EdgeIndex::END, EdgeIndex::END],
        };
        assert!(self.nodes.len() as u32 != u32::MAX);
        let node_idx = NodeIndex(self.nodes.len() as u32);
        self.nodes.push(node);
        node_idx
    }

    /// For edge `e` with endpoints `edge_node`, replaces links to it
    /// with links to `edge_next`.
    pub(super) fn change_edge_links(
        &mut self,
        edge_node: [NodeIndex; 2],
        e: EdgeIndex,
        edge_next: [EdgeIndex; 2],
    ) {
        for d in EdgeDirection::ALL {
            let k = d as usize;
            let node = match self.nodes.get_mut(edge_node[k].index()) {
                Some(r) => r,
                None => {
                    debug_assert!(
                        false,
                        "Edge's endpoint dir={:?} index={:?} not found",
                        d, edge_node[k]
                    );
                    return;
                }
            };
            let fst = node.next[k];
            if fst == e {
                node.next[k] = edge_next[k];
            } else {
                let mut edges = EdgesWalkerMut {
                    edges: &mut self.edges,
                    next: fst,
                    dir: d,
                };
                while let Some(curedge) = edges.next_edge() {
                    if curedge.next[k] == e {
                        curedge.next[k] = edge_next[k];
                        // The edge can only be present once in the list.
                        break;
                    }
                }
            }
        }
    }
}

struct EdgesWalkerMut<'a, E: 'a> {
    edges: &'a mut [Edge<E>],
    next: EdgeIndex,
    dir: EdgeDirection,
}

impl<E> EdgesWalkerMut<'_, E> {
    fn next_edge(&mut self) -> Option<&mut Edge<E>> {
        self.next().map(|t| t.1)
    }

    fn next(&mut self) -> Option<(EdgeIndex, &mut Edge<E>)> {
        let this_index = self.next;
        let k = self.dir as usize;
        match self.edges.get_mut(self.next.index()) {
            None => None,
            Some(edge) => {
                self.next = edge.next[k];
                Some((this_index, edge))
            }
        }
    }
}

/// Indexes the `UnGraph` by `NodeIndex` to access node weights.
///
/// # Panics
///
/// Panics if the node doesn't exist.
impl<N, E> Index<NodeIndex> for UnGraph<N, E> {
    type Output = N;
    fn index(&self, index: NodeIndex) -> &N {
        &self.nodes[index.index()].weight
    }
}

/// Indexes the `UnGraph` by `NodeIndex` to access node weights.
///
/// # Panics
///
/// Panics if the node doesn't exist.
impl<N, E> IndexMut<NodeIndex> for UnGraph<N, E> {
    fn index_mut(&mut self, index: NodeIndex) -> &mut N {
        &mut self.nodes[index.index()].weight
    }
}

/// Indexes the `UnGraph` by `EdgeIndex` to access edge weights.
///
/// # Panics
///
/// Panics if the edge doesn't exist.
impl<N, E> Index<EdgeIndex> for UnGraph<N, E> {
    type Output = E;
    fn index(&self, index: EdgeIndex) -> &E {
        &self.edges[index.index()].weight
    }
}

/// Indexes the `UnGraph` by `EdgeIndex` to access edge weights.
///
/// # Panics
///
/// Panics if the edge doesn't exist.
impl<N, E> IndexMut<EdgeIndex> for UnGraph<N, E> {
    fn index_mut(&mut self, index: EdgeIndex) -> &mut E {
        &mut self.edges[index.index()].weight
    }
}
