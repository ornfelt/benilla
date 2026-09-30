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

impl<E> Edge<E> {
    /// Return the source node index.
    pub fn source(&self) -> NodeIndex {
        self.node[0]
    }

    /// Return the target node index.
    pub fn target(&self) -> NodeIndex {
        self.node[1]
    }
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

    /// Accessor for data structure internals: returns the next edge for the given direction.
    pub fn next_edge(&self, e: EdgeIndex, dir: EdgeDirection) -> Option<EdgeIndex> {
        match self.edges.get(e.index()) {
            None => None,
            Some(node) => {
                let edix = node.next[dir as usize];
                if edix == EdgeIndex::END {
                    None
                } else {
                    Some(edix)
                }
            }
        }
    }
}

/// An iterator over the neighbors of a node.
///
/// The iterator element type is `NodeIndex`.
#[derive(Debug)]
pub struct Neighbors<'a, E: 'a> {
    /// The starting node to skip over.
    skip_start: NodeIndex,

    /// The edges to iterate over.
    edges: &'a [Edge<E>],

    /// The next edge to visit.
    next: [EdgeIndex; 2],
}

impl<E> Iterator for Neighbors<'_, E> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<NodeIndex> {
        // First any outgoing edges.
        match self.edges.get(self.next[0].index()) {
            None => {}
            Some(edge) => {
                self.next[0] = edge.next[0];
                return Some(edge.node[1]);
            }
        }

        // Then incoming edges.
        // For an "undirected" iterator, make sure we don't double
        // count self-loops by skipping them in the incoming list.
        while let Some(edge) = self.edges.get(self.next[1].index()) {
            self.next[1] = edge.next[1];
            if edge.node[0] != self.skip_start {
                return Some(edge.node[0]);
            }
        }
        None
    }
}

impl<E> Clone for Neighbors<'_, E> {
    fn clone(&self) -> Self {
        Neighbors {
            skip_start: self.skip_start,
            edges: self.edges,
            next: self.next,
        }
    }
}

/// An iterator over edges from or to a node.
pub struct Edges<'a, E: 'a> {
    /// The starting node to skip over.
    skip_start: NodeIndex,

    /// The edges to iterate over.
    edges: &'a [Edge<E>],

    /// The next edge to visit.
    next: [EdgeIndex; 2],

    /// The direction of edges.
    direction: EdgeDirection,
}

impl<'a, E> Iterator for Edges<'a, E> {
    type Item = EdgeReference<'a, E>;

    fn next(&mut self) -> Option<Self::Item> {
        // Outgoing
        let i = self.next[0].index();
        if let Some(Edge {
            node, weight, next, ..
        }) = self.edges.get(i)
        {
            self.next[0] = next[0];
            return Some(EdgeReference {
                index: EdgeIndex(i as u32),
                node: if self.direction == EdgeDirection::Incoming {
                    swap_pair(*node)
                } else {
                    *node
                },
                weight,
            });
        }

        // Incoming
        while let Some(Edge { node, weight, next }) = self.edges.get(self.next[1].index()) {
            let edge_index = self.next[1];
            self.next[1] = next[1];

            // In any of the "both" situations, self-loops would be iterated over twice.
            // Skip them here.
            if node[0] == self.skip_start {
                continue;
            }

            return Some(EdgeReference {
                index: edge_index,
                node: if self.direction == EdgeDirection::Outgoing {
                    swap_pair(*node)
                } else {
                    *node
                },
                weight,
            });
        }

        None
    }
}

impl<E> Clone for Edges<'_, E> {
    fn clone(&self) -> Self {
        Edges {
            skip_start: self.skip_start,
            edges: self.edges,
            next: self.next,
            direction: self.direction,
        }
    }
}

/// An iterator over mutable references to all edges from or to a node.
pub struct EdgesMut<'a, N, E> {
    graph: &'a mut UnGraph<N, E>,
    incoming_edge: Option<EdgeIndex>,
    outgoing_edge: Option<EdgeIndex>,
}

impl<'a, N: Copy, E> Iterator for EdgesMut<'a, N, E> {
    type Item = EdgeMut<'a, E>;

    #[inline]
    fn next(&mut self) -> Option<EdgeMut<'a, E>> {
        if let Some(edge) = self.incoming_edge {
            self.incoming_edge = self.graph.next_edge(edge, EdgeDirection::Incoming);
            let weights = &mut self.graph[edge];
            return Some(EdgeMut {
                index: edge,
                weight: unsafe { core::mem::transmute::<&mut E, &'a mut E>(weights) },
            });
        }

        let edge = self.outgoing_edge?;
        self.outgoing_edge = self.graph.next_edge(edge, EdgeDirection::Outgoing);
        let weights = &mut self.graph[edge];
        Some(EdgeMut {
            index: edge,
            weight: unsafe { core::mem::transmute::<&mut E, &'a mut E>(weights) },
        })
    }
}

/// Iterator over the edges between a source node and a target node.
#[derive(Clone)]
pub struct EdgesBetween<'a, E: 'a> {
    target_node: NodeIndex,
    edges: Edges<'a, E>,
}

impl<'a, E> Iterator for EdgesBetween<'a, E> {
    type Item = EdgeReference<'a, E>;

    fn next(&mut self) -> Option<EdgeReference<'a, E>> {
        let target_node = self.target_node;
        self.edges
            .by_ref()
            .find(|&edge| edge.target() == target_node)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.edges.size_hint();
        (0, upper)
    }
}

fn swap_pair<T>(mut x: [T; 2]) -> [T; 2] {
    x.swap(0, 1);
    x
}

// TODO: Reduce duplication between `Edges` variants.
/// An iterator over edge weights for edges from or to a node.
pub struct EdgeWeights<'a, E: 'a> {
    /// The starting node to skip over.
    skip_start: NodeIndex,

    /// The edges to iterate over.
    edges: &'a [Edge<E>],

    /// The next edge to visit.
    next: [EdgeIndex; 2],

    /// The direction of edges.
    direction: EdgeDirection,
}

impl<'a, E> Iterator for EdgeWeights<'a, E> {
    type Item = &'a E;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.next[0].index();
        if let Some(Edge { weight, next, .. }) = self.edges.get(i) {
            self.next[0] = next[0];
            return Some(weight);
        }

        while let Some(Edge { node, weight, next }) = self.edges.get(self.next[1].index()) {
            self.next[1] = next[1];

            // In any of the "both" situations, self-loops would be iterated over twice.
            // Skip them here.
            if node[0] == self.skip_start {
                continue;
            }

            return Some(weight);
        }

        None
    }
}

impl<E> Clone for EdgeWeights<'_, E> {
    fn clone(&self) -> Self {
        EdgeWeights {
            skip_start: self.skip_start,
            edges: self.edges,
            next: self.next,
            direction: self.direction,
        }
    }
}

/// An iterator over mutable references to all edge weights from or to a node.
pub struct EdgeWeightsMut<'a, N, E> {
    /// A mutable reference to the graph.
    pub graph: &'a mut UnGraph<N, E>,

    /// The next incoming edge to visit.
    pub incoming_edge: Option<EdgeIndex>,

    /// The next outgoing edge to visit.
    pub outgoing_edge: Option<EdgeIndex>,
}

impl<'a, N: Copy, E> Iterator for EdgeWeightsMut<'a, N, E> {
    type Item = &'a mut E;

    #[inline]
    fn next(&mut self) -> Option<&'a mut E> {
        if let Some(edge) = self.incoming_edge {
            self.incoming_edge = self.graph.next_edge(edge, EdgeDirection::Incoming);
            let weight = &mut self.graph[edge];
            return Some(unsafe { core::mem::transmute::<&mut E, &'a mut E>(weight) });
        }

        let edge = self.outgoing_edge?;
        self.outgoing_edge = self.graph.next_edge(edge, EdgeDirection::Outgoing);
        let weight = &mut self.graph[edge];
        Some(unsafe { core::mem::transmute::<&mut E, &'a mut E>(weight) })
    }
}

/// An iterator yielding immutable access to all edge weights.
pub struct AllEdgeWeights<'a, E: 'a> {
    edges: core::slice::Iter<'a, Edge<E>>,
}

impl<'a, E> Iterator for AllEdgeWeights<'a, E> {
    type Item = &'a E;

    fn next(&mut self) -> Option<&'a E> {
        self.edges.next().map(|edge| &edge.weight)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.edges.size_hint()
    }
}

/// An iterator yielding mutable access to all edge weights.
#[derive(Debug)]
pub struct AllEdgeWeightsMut<'a, E: 'a> {
    edges: core::slice::IterMut<'a, Edge<E>>,
}

impl<'a, E> Iterator for AllEdgeWeightsMut<'a, E> {
    type Item = &'a mut E;

    fn next(&mut self) -> Option<&'a mut E> {
        self.edges.next().map(|edge| &mut edge.weight)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.edges.size_hint()
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

/// A reference to a graph edge.
#[derive(Debug)]
pub struct EdgeReference<'a, E: 'a> {
    index: EdgeIndex,
    node: [NodeIndex; 2],
    weight: &'a E,
}

impl<'a, E: 'a> EdgeReference<'a, E> {
    /// Returns the target node index.
    #[inline]
    pub fn target(&self) -> NodeIndex {
        self.node[1]
    }
}

impl<E> Clone for EdgeReference<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E> Copy for EdgeReference<'_, E> {}

impl<E> PartialEq for EdgeReference<'_, E>
where
    E: PartialEq,
{
    fn eq(&self, rhs: &Self) -> bool {
        self.index == rhs.index && self.weight == rhs.weight
    }
}

/// A mutable reference to a graph edge.
#[derive(Debug)]
pub struct EdgeMut<'a, E: 'a> {
    index: EdgeIndex,
    weight: &'a mut E,
}

impl<E> PartialEq for EdgeMut<'_, E>
where
    E: PartialEq,
{
    fn eq(&self, rhs: &Self) -> bool {
        self.index == rhs.index && self.weight == rhs.weight
    }
}
