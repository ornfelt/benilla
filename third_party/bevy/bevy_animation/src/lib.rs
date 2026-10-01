#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Animation for the game engine Bevy

extern crate alloc;

pub mod animatable;
pub mod animation_curves;
pub mod graph;
pub mod transition;

mod util;

use core::{
    any::TypeId,
    cell::RefCell,
    fmt::Debug,
    hash::{Hash, Hasher},
    iter,
};
use graph::AnimationNodeType;
use prelude::AnimationCurveEvaluator;

use crate::{
    graph::{AnimationGraphHandle, ThreadedAnimationGraphs},
    prelude::EvaluatorId,
};

use bevy_app::{AnimationSystems, App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, AssetEventSystems, Assets};
use bevy_ecs::{prelude::*, world::EntityMutExcept};
use bevy_platform::{collections::HashMap, hash::NoOpHash};
use bevy_reflect::TypePath;
use bevy_time::Time;
use bevy_transform::TransformSystems;
use bevy_utils::{PreHashMap, PreHashMapExt, TypeIdMap};
use thread_local::ThreadLocal;
use tracing::{trace, warn};
use uuid::Uuid;

/// The animation prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        animatable::*, animation_curves::*, graph::*, transition::*, AnimationClip,
        AnimationPlayer, AnimationPlugin, VariableCurve,
    };
}

use crate::{
    animation_curves::AnimationCurve,
    graph::{AnimationGraph, AnimationNodeIndex},
    transition::{advance_transitions, expire_completed_transitions},
};

/// The [UUID namespace] of animation targets (e.g. bones).
///
/// [UUID namespace]: https://en.wikipedia.org/wiki/Universally_unique_identifier#Versions_3_and_5_(namespace_name-based)
pub static ANIMATION_TARGET_NAMESPACE: Uuid = Uuid::from_u128(0x3179f519d9274ff2b5966fd077023911);

/// Contains an [animation curve] which is used to animate a property of an entity.
///
/// [animation curve]: AnimationCurve
#[derive(Debug, TypePath)]
pub struct VariableCurve(pub Box<dyn AnimationCurve>);

impl Clone for VariableCurve {
    fn clone(&self) -> Self {
        Self(AnimationCurve::clone_value(&*self.0))
    }
}

impl VariableCurve {
    /// Create a new [`VariableCurve`] from an [animation curve].
    ///
    /// [animation curve]: AnimationCurve
    pub fn new(animation_curve: impl AnimationCurve) -> Self {
        Self(Box::new(animation_curve))
    }
}

/// A list of [`VariableCurve`]s and the [`AnimationTargetId`]s to which they
/// apply.
///
/// Because animation clips refer to targets by UUID, they can target any
/// entity with that ID.
#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct AnimationClip {
    // This field is ignored by reflection because AnimationCurves can contain things that are not reflect-able
    curves: AnimationCurves,
    duration: f32,
}

/// A mapping from [`AnimationTargetId`] (e.g. bone in a skinned mesh) to the
/// animation curves.
pub type AnimationCurves = HashMap<AnimationTargetId, Vec<VariableCurve>, NoOpHash>;

/// A component that identifies which parts of an [`AnimationClip`] asset can
/// be applied to an entity. Typically used alongside the
/// [`AnimatedBy`] component.
///
/// `AnimationTargetId` is implemented as a [UUID]. When importing an armature
/// or an animation clip, asset loaders typically use the full path name from
/// the armature to the bone to generate these UUIDs. The ID is unique to the
/// full path name and based only on the names. So, for example, any imported
/// armature with a bone at the root named `Hips` will assign the same
/// [`AnimationTargetId`] to its root bone. Likewise, any imported animation
/// clip that animates a root bone named `Hips` will reference the same
/// [`AnimationTargetId`]. Any animation is playable on any armature as long as
/// the bone names match, which allows for easy animation retargeting.
///
/// Note that asset loaders generally use the *full* path name to generate the
/// [`AnimationTargetId`]. Thus a bone named `Chest` directly connected to a
/// bone named `Hips` will have a different ID from a bone named `Chest` that's
/// connected to a bone named `Stomach`.
///
/// [UUID]: https://en.wikipedia.org/wiki/Universally_unique_identifier
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Component)]
pub struct AnimationTargetId(pub Uuid);

impl Hash for AnimationTargetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (hi, lo) = self.0.as_u64_pair();
        state.write_u64(hi ^ lo);
    }
}

/// A component that links an animated entity to an entity containing an
/// [`AnimationPlayer`]. Typically used alongside the [`AnimationTargetId`]
/// component - the linked `AnimationPlayer` plays [`AnimationClip`] assets, and
/// the `AnimationTargetId` identifies which curves in the `AnimationClip` will
/// affect the target entity.
///
/// By convention, asset loaders add [`AnimationTargetId`] components to the
/// descendants of an [`AnimationPlayer`], as well as to the [`AnimationPlayer`]
/// entity itself, but Bevy doesn't require this in any way. So, for example,
/// it's entirely possible for an [`AnimationPlayer`] to animate a target that
/// it isn't an ancestor of. If you add a new bone to or delete a bone from an
/// armature at runtime, you may want to update the [`AnimationTargetId`]
/// component as appropriate, as Bevy won't do this automatically.
///
/// Note that each entity can only be animated by one animation player at a
/// time. However, you can change [`AnimatedBy`] components at runtime and
/// link them to a different player.
#[derive(Clone, Copy, Component, Debug)]
pub struct AnimatedBy(#[entities] pub Entity);

impl AnimationClip {
    #[inline]
    /// [`VariableCurve`]s for each animation target. Indexed by the [`AnimationTargetId`].
    pub fn curves(&self) -> &AnimationCurves {
        &self.curves
    }

    /// Gets the curves for a single animation target.
    ///
    /// Returns `None` if this clip doesn't animate the target.
    #[inline]
    pub fn curves_for_target(
        &self,
        target_id: AnimationTargetId,
    ) -> Option<&'_ Vec<VariableCurve>> {
        self.curves.get(&target_id)
    }

    /// Duration of the clip, represented in seconds.
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Set the duration of the clip in seconds.
    #[inline]
    pub fn set_duration(&mut self, duration_sec: f32) {
        self.duration = duration_sec;
    }

    /// Adds an [`AnimationCurve`] that can target an entity with the given
    /// [`AnimationTargetId`] component.
    ///
    /// If the curve extends beyond the current duration of this clip, this
    /// method lengthens this clip to include the entire time span that the
    /// curve covers.
    ///
    /// More specifically:
    /// - This clip will be sampled on the interval `[0, duration]`.
    /// - Each curve in the clip is sampled by first clamping the sample time to its [domain].
    /// - Curves that extend forever never contribute to the duration.
    ///
    /// For example, a curve with domain `[2, 5]` will extend the clip to cover `[0, 5]`
    /// when added and will produce the same output on the entire interval `[0, 2]` because
    /// these time values all get clamped to `2`.
    ///
    /// By contrast, a curve with domain `[-10, ∞]` will never extend the clip duration when
    /// added and will be sampled only on `[0, duration]`, ignoring all negative time values.
    ///
    /// [domain]: AnimationCurve::domain
    pub fn add_curve_to_target(
        &mut self,
        target_id: AnimationTargetId,
        curve: impl AnimationCurve,
    ) {
        // Update the duration of the animation by this curve duration if it's longer
        let end = curve.domain().end();
        if end.is_finite() {
            self.duration = self.duration.max(end);
        }
        self.curves
            .entry(target_id)
            .or_default()
            .push(VariableCurve::new(curve));
    }
}

/// Repetition behavior of an animation.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
pub enum RepeatAnimation {
    /// The animation will finish after running once.
    #[default]
    Never,
    /// The animation will finish after running "n" times.
    Count(u32),
    /// The animation will never finish.
    Forever,
}

/// Why Bevy failed to evaluate an animation.
#[derive(Clone, Debug)]
pub enum AnimationEvaluationError {
    /// The component to be animated isn't present on the animation target.
    ///
    /// To fix this error, make sure the entity to be animated contains all
    /// components that have animation curves.
    ComponentNotPresent(TypeId),

    /// The component to be animated was present, but the property on the
    /// component wasn't present.
    PropertyNotPresent(TypeId),

    /// An internal error occurred in the implementation of
    /// [`AnimationCurveEvaluator`].
    ///
    /// You shouldn't ordinarily see this error unless you implemented
    /// [`AnimationCurveEvaluator`] yourself. The contained [`TypeId`] is the ID
    /// of the curve evaluator.
    InconsistentEvaluatorImplementation(TypeId),
}

/// An animation that an [`AnimationPlayer`] is currently either playing or was
/// playing, but is presently paused.
///
/// A stopped animation is considered no longer active.
#[derive(Debug, Clone, Copy)]
pub struct ActiveAnimation {
    /// The factor by which the weight from the [`AnimationGraph`] is multiplied.
    weight: f32,
    repeat: RepeatAnimation,
    speed: f32,
    /// Total time the animation has been played.
    ///
    /// Note: Time does not increase when the animation is paused or after it has completed.
    elapsed: f32,
    /// The timestamp inside of the animation clip.
    ///
    /// Note: This will always be in the range [0.0, animation clip duration]
    seek_time: f32,
    /// Number of times the animation has completed.
    /// If the animation is playing in reverse, this increments when the animation passes the start.
    completions: u32,
    paused: bool,
}

impl Default for ActiveAnimation {
    fn default() -> Self {
        Self {
            weight: 1.0,
            repeat: RepeatAnimation::default(),
            speed: 1.0,
            elapsed: 0.0,
            seek_time: 0.0,
            completions: 0,
            paused: false,
        }
    }
}

impl ActiveAnimation {
    /// Check if the animation has finished, based on its repetition behavior and the number of times it has repeated.
    ///
    /// Note: An animation with `RepeatAnimation::Forever` will never finish.
    #[inline]
    pub fn is_finished(&self) -> bool {
        match self.repeat {
            RepeatAnimation::Forever => false,
            RepeatAnimation::Never => self.completions >= 1,
            RepeatAnimation::Count(n) => self.completions >= n,
        }
    }

    /// Update the animation given the delta time and the duration of the clip being played.
    #[inline]
    fn update(&mut self, delta: f32, clip_duration: f32) {
        if self.is_finished() {
            return;
        }

        self.elapsed += delta;
        self.seek_time += delta * self.speed;

        let over_time = self.speed > 0.0 && self.seek_time >= clip_duration;
        let under_time = self.speed < 0.0 && self.seek_time < 0.0;

        if over_time || under_time {
            self.completions += 1;

            if self.is_finished() {
                return;
            }
        }
        if self.seek_time >= clip_duration {
            self.seek_time %= clip_duration;
        }
        // Note: assumes delta is never lower than -clip_duration
        if self.seek_time < 0.0 {
            self.seek_time += clip_duration;
        }
    }

    /// Reset back to the initial state as if no time has elapsed.
    pub fn replay(&mut self) {
        self.completions = 0;
        self.elapsed = 0.0;
        self.seek_time = 0.0;
    }

    /// Returns the current weight of this animation.
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Sets the weight of this animation.
    pub fn set_weight(&mut self, weight: f32) -> &mut Self {
        self.weight = weight;
        self
    }

    /// Pause the animation.
    pub fn pause(&mut self) -> &mut Self {
        self.paused = true;
        self
    }

    /// Unpause the animation.
    pub fn resume(&mut self) -> &mut Self {
        self.paused = false;
        self
    }

    /// Returns true if this animation is currently paused.
    ///
    /// Note that paused animations are still [`ActiveAnimation`]s.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Sets the repeat mode for this playing animation.
    pub fn set_repeat(&mut self, repeat: RepeatAnimation) -> &mut Self {
        self.repeat = repeat;
        self
    }

    /// Marks this animation as repeating forever.
    pub fn repeat(&mut self) -> &mut Self {
        self.set_repeat(RepeatAnimation::Forever)
    }

    /// Returns the repeat mode assigned to this active animation.
    pub fn repeat_mode(&self) -> RepeatAnimation {
        self.repeat
    }

    /// Returns the number of times this animation has completed.
    pub fn completions(&self) -> u32 {
        self.completions
    }

    /// Returns the speed of the animation playback.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Sets the speed of the animation playback.
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    /// Returns the amount of time the animation has been playing.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Returns the seek time of the animation.
    ///
    /// This is nonnegative and no more than the clip duration.
    pub fn seek_time(&self) -> f32 {
        self.seek_time
    }

    /// Seeks to a specific time in the animation.
    pub fn seek_to(&mut self, seek_time: f32) -> &mut Self {
        self.seek_time = seek_time;
        self
    }
}

/// Animation controls.
///
/// Automatically added to any root animations of a scene when it is
/// spawned.
#[derive(Component, Default)]
pub struct AnimationPlayer {
    active_animations: HashMap<AnimationNodeIndex, ActiveAnimation>,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for AnimationPlayer {
    fn clone(&self) -> Self {
        Self {
            active_animations: self.active_animations.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.active_animations.clone_from(&source.active_animations);
    }
}

/// Temporary data that the [`animate_targets`] system maintains.
#[derive(Default)]
pub struct AnimationEvaluationState {
    /// Stores all [`AnimationCurveEvaluator`]s corresponding to properties that
    /// we've seen so far.
    ///
    /// This is a mapping from the id of an animation curve evaluator to
    /// the animation curve evaluator itself.
    ///
    /// For efficiency's sake, the [`AnimationCurveEvaluator`]s are cached from
    /// frame to frame and animation target to animation target. Therefore,
    /// there may be entries in this list corresponding to properties that the
    /// current [`AnimationPlayer`] doesn't animate. To iterate only over the
    /// properties that are currently being animated, consult the
    /// [`Self::current_evaluators`] set.
    evaluators: AnimationCurveEvaluators,

    /// The set of [`AnimationCurveEvaluator`] types that the current
    /// [`AnimationPlayer`] is animating.
    ///
    /// This is built up as new curve evaluators are encountered during graph
    /// traversal.
    current_evaluators: CurrentEvaluators,
}

#[derive(Default)]
struct AnimationCurveEvaluators {
    component_property_curve_evaluators:
        PreHashMap<(TypeId, usize), Box<dyn AnimationCurveEvaluator>>,
    type_id_curve_evaluators: TypeIdMap<Box<dyn AnimationCurveEvaluator>>,
}

impl AnimationCurveEvaluators {
    #[inline]
    pub(crate) fn get_mut(&mut self, id: EvaluatorId) -> Option<&mut dyn AnimationCurveEvaluator> {
        match id {
            EvaluatorId::ComponentField(component_property) => self
                .component_property_curve_evaluators
                .get_mut(component_property),
            EvaluatorId::Type(type_id) => self.type_id_curve_evaluators.get_mut(&type_id),
        }
        .map(|e| &mut **e)
    }

    #[inline]
    pub(crate) fn get_or_insert_with(
        &mut self,
        id: EvaluatorId,
        func: impl FnOnce() -> Box<dyn AnimationCurveEvaluator>,
    ) -> &mut dyn AnimationCurveEvaluator {
        match id {
            EvaluatorId::ComponentField(component_property) => &mut **self
                .component_property_curve_evaluators
                .get_or_insert_with(component_property, func),
            EvaluatorId::Type(type_id) => match self.type_id_curve_evaluators.entry(type_id) {
                bevy_platform::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    &mut **occupied_entry.into_mut()
                }
                bevy_platform::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    &mut **vacant_entry.insert(func())
                }
            },
        }
    }
}

#[derive(Default)]
struct CurrentEvaluators {
    component_properties: PreHashMap<(TypeId, usize), ()>,
    type_ids: TypeIdMap<()>,
}

impl CurrentEvaluators {
    pub(crate) fn keys(&self) -> impl Iterator<Item = EvaluatorId<'_>> {
        self.component_properties
            .keys()
            .map(EvaluatorId::ComponentField)
            .chain(self.type_ids.keys().copied().map(EvaluatorId::Type))
    }

    pub(crate) fn clear(
        &mut self,
        mut visit: impl FnMut(EvaluatorId) -> Result<(), AnimationEvaluationError>,
    ) -> Result<(), AnimationEvaluationError> {
        for (key, _) in self.component_properties.drain() {
            (visit)(EvaluatorId::ComponentField(&key))?;
        }

        for (key, _) in self.type_ids.drain() {
            (visit)(EvaluatorId::Type(key))?;
        }

        Ok(())
    }

    #[inline]
    pub(crate) fn insert(&mut self, id: EvaluatorId) {
        match id {
            EvaluatorId::ComponentField(component_property) => {
                self.component_properties.insert(*component_property, ());
            }
            EvaluatorId::Type(type_id) => {
                self.type_ids.insert(type_id, ());
            }
        }
    }
}

impl AnimationPlayer {
    /// Start playing an animation, restarting it if necessary.
    pub fn start(&mut self, animation: AnimationNodeIndex) -> &mut ActiveAnimation {
        let playing_animation = self.active_animations.entry(animation).or_default();
        playing_animation.replay();
        playing_animation
    }

    /// Start playing an animation, unless the requested animation is already playing.
    pub fn play(&mut self, animation: AnimationNodeIndex) -> &mut ActiveAnimation {
        self.active_animations.entry(animation).or_default()
    }

    /// Stops playing the given animation, removing it from the list of playing
    /// animations.
    pub fn stop(&mut self, animation: AnimationNodeIndex) -> &mut Self {
        self.active_animations.remove(&animation);
        self
    }

    /// Stops all currently-playing animations.
    pub fn stop_all(&mut self) -> &mut Self {
        self.active_animations.clear();
        self
    }

    /// Iterates through all animations that this [`AnimationPlayer`] is
    /// currently playing.
    pub fn playing_animations(
        &self,
    ) -> impl Iterator<Item = (&AnimationNodeIndex, &ActiveAnimation)> {
        self.active_animations.iter()
    }

    /// Iterates through all animations that this [`AnimationPlayer`] is
    /// currently playing, mutably.
    pub fn playing_animations_mut(
        &mut self,
    ) -> impl Iterator<Item = (&AnimationNodeIndex, &mut ActiveAnimation)> {
        self.active_animations.iter_mut()
    }

    /// Returns the [`ActiveAnimation`] associated with the given animation
    /// node if it's currently playing.
    ///
    /// If the animation isn't currently active, returns `None`.
    pub fn animation(&self, animation: AnimationNodeIndex) -> Option<&ActiveAnimation> {
        self.active_animations.get(&animation)
    }

    /// Returns a mutable reference to the [`ActiveAnimation`] associated with
    /// the given animation node if it's currently active.
    ///
    /// If the animation isn't currently active, returns `None`.
    pub fn animation_mut(&mut self, animation: AnimationNodeIndex) -> Option<&mut ActiveAnimation> {
        self.active_animations.get_mut(&animation)
    }
}

/// Stand-in for the system that triggered untargeted animation events (no clip has events); its
/// `Commands` keeps the sync point before `expire_completed_transitions`.
fn trigger_untargeted_animation_events(_commands: Commands) {}

/// A system that advances the time for all playing animations.
pub fn advance_animations(
    time: Res<Time>,
    animation_clips: Res<Assets<AnimationClip>>,
    animation_graphs: Res<Assets<AnimationGraph>>,
    mut players: Query<(&mut AnimationPlayer, &AnimationGraphHandle)>,
) {
    let delta_seconds = time.delta_secs();
    players
        .par_iter_mut()
        .for_each(|(mut player, graph_handle)| {
            let Some(animation_graph) = animation_graphs.get(graph_handle) else {
                return;
            };

            // Tick animations, and schedule them.

            let AnimationPlayer {
                ref mut active_animations,
                ..
            } = *player;

            for node_index in animation_graph.graph.node_indices() {
                let node = &animation_graph[node_index];

                if let Some(active_animation) = active_animations.get_mut(&node_index) {
                    // Tick the animation if necessary.
                    if !active_animation.paused
                        && let AnimationNodeType::Clip(ref clip_handle) = node.node_type
                        && let Some(clip) = animation_clips.get(clip_handle)
                    {
                        active_animation.update(delta_seconds, clip.duration);
                    }
                }
            }
        });
}

/// A type alias for [`EntityMutExcept`] as used in animation.
pub type AnimationEntityMut<'w, 's> = EntityMutExcept<
    'w,
    's,
    (
        AnimationTargetId,
        AnimatedBy,
        AnimationPlayer,
        AnimationGraphHandle,
    ),
>;

/// A system that modifies animation targets (e.g. bones in a skinned mesh)
/// according to the currently-playing animations.
pub fn animate_targets(
    // Unused (it carried the targeted animation events; no clip has events): it keeps the sync
    // point after this system.
    _par_commands: ParallelCommands,
    clips: Res<Assets<AnimationClip>>,
    graphs: Res<Assets<AnimationGraph>>,
    threaded_animation_graphs: Res<ThreadedAnimationGraphs>,
    players: Query<(&AnimationPlayer, &AnimationGraphHandle)>,
    mut targets: Query<(&AnimationTargetId, &AnimatedBy, AnimationEntityMut)>,
    animation_evaluation_state: Local<ThreadLocal<RefCell<AnimationEvaluationState>>>,
) {
    // Evaluate all animation targets in parallel.
    targets
        .par_iter_mut()
        .for_each(|(&target_id, &AnimatedBy(player_id), entity_mut)| {
            let (animation_player, animation_graph_id) =
                if let Ok((player, graph_handle)) = players.get(player_id) {
                    (player, graph_handle.id())
                } else {
                    trace!(
                        "Either an animation player {} or a graph was missing for the target \
                         entity {} ({:?}); no animations will play this frame",
                        player_id,
                        entity_mut.id(),
                        entity_mut.get::<Name>(),
                    );
                    return;
                };

            // The graph might not have loaded yet. Safely bail.
            let Some(animation_graph) = graphs.get(animation_graph_id) else {
                return;
            };

            let Some(threaded_animation_graph) =
                threaded_animation_graphs.0.get(&animation_graph_id)
            else {
                return;
            };

            // Determine which mask groups this animation target belongs to.
            let target_mask = animation_graph
                .mask_groups
                .get(&target_id)
                .cloned()
                .unwrap_or_default();

            let mut evaluation_state = animation_evaluation_state.get_or_default().borrow_mut();
            let evaluation_state = &mut *evaluation_state;

            // Evaluate the graph.
            for &animation_graph_node_index in threaded_animation_graph.threaded_graph.iter() {
                let Some(animation_graph_node) = animation_graph.get(animation_graph_node_index)
                else {
                    continue;
                };

                match animation_graph_node.node_type {
                    AnimationNodeType::Blend => {
                        // This is a blend node.
                        for edge_index in threaded_animation_graph.sorted_edge_ranges
                            [animation_graph_node_index.index()]
                        .clone()
                        {
                            if let Err(err) = evaluation_state.blend_all(
                                threaded_animation_graph.sorted_edges[edge_index as usize],
                            ) {
                                warn!("Failed to blend animation: {:?}", err);
                            }
                        }

                        if let Err(err) = evaluation_state.push_blend_register_all(
                            animation_graph_node.weight,
                            animation_graph_node_index,
                        ) {
                            warn!("Animation blending failed: {:?}", err);
                        }
                    }

                    AnimationNodeType::Add => {
                        // This is an additive blend node.
                        for edge_index in threaded_animation_graph.sorted_edge_ranges
                            [animation_graph_node_index.index()]
                        .clone()
                        {
                            if let Err(err) = evaluation_state
                                .add_all(threaded_animation_graph.sorted_edges[edge_index as usize])
                            {
                                warn!("Failed to blend animation: {:?}", err);
                            }
                        }

                        if let Err(err) = evaluation_state.push_blend_register_all(
                            animation_graph_node.weight,
                            animation_graph_node_index,
                        ) {
                            warn!("Animation blending failed: {:?}", err);
                        }
                    }

                    AnimationNodeType::Clip(ref animation_clip_handle) => {
                        // This is a clip node.
                        let Some(active_animation) = animation_player
                            .active_animations
                            .get(&animation_graph_node_index)
                        else {
                            continue;
                        };

                        // If the weight is zero or the current animation target is
                        // masked out, stop here.
                        if active_animation.weight == 0.0
                            || (target_mask
                                & threaded_animation_graph.computed_masks
                                    [animation_graph_node_index.index()])
                                != 0
                        {
                            continue;
                        }

                        let Some(clip) = clips.get(animation_clip_handle) else {
                            continue;
                        };

                        let Some(curves) = clip.curves_for_target(target_id) else {
                            continue;
                        };

                        let weight = active_animation.weight * animation_graph_node.weight;
                        let seek_time = active_animation.seek_time;

                        for curve in curves {
                            // Fetch the curve evaluator. Curve evaluator types
                            // are unique to each property, but shared among all
                            // curve types. For example, given two curve types A
                            // and B, `RotationCurve<A>` and `RotationCurve<B>`
                            // will both yield a `RotationCurveEvaluator` and
                            // therefore will share the same evaluator in this
                            // table.
                            let curve_evaluator_id = (*curve.0).evaluator_id();
                            let curve_evaluator = evaluation_state
                                .evaluators
                                .get_or_insert_with(curve_evaluator_id.clone(), || {
                                    curve.0.create_evaluator()
                                });

                            evaluation_state
                                .current_evaluators
                                .insert(curve_evaluator_id);

                            if let Err(err) = AnimationCurve::apply(
                                &*curve.0,
                                curve_evaluator,
                                seek_time,
                                weight,
                                animation_graph_node_index,
                            ) {
                                warn!("Animation application failed: {:?}", err);
                            }
                        }
                    }
                }
            }

            if let Err(err) = evaluation_state.commit_all(entity_mut) {
                warn!("Animation application failed: {:?}", err);
            }
        });
}

/// Adds animation support to an app
#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AnimationClip>()
            .init_asset::<AnimationGraph>()
            .init_resource::<ThreadedAnimationGraphs>()
            .add_systems(
                PostUpdate,
                (
                    graph::thread_animation_graphs.before(AssetEventSystems),
                    advance_transitions,
                    advance_animations,
                    // TODO: `animate_targets` can animate anything, so
                    // ambiguity testing currently considers it ambiguous with
                    // every other system in `PostUpdate`. We may want to move
                    // it to its own system set after `Update` but before
                    // `PostUpdate`. For now, we just disable ambiguity testing
                    // for this system.
                    #[cfg(feature = "bevy_mesh")]
                    animate_targets
                        .before(bevy_mesh::InheritWeightSystems)
                        .ambiguous_with_all(),
                    #[cfg(not(feature = "bevy_mesh"))]
                    animate_targets.ambiguous_with_all(),
                    trigger_untargeted_animation_events,
                    expire_completed_transitions,
                )
                    .chain()
                    .in_set(AnimationSystems)
                    .before(TransformSystems::Propagate),
            );
    }
}

impl AnimationTargetId {
    /// Creates a new [`AnimationTargetId`] by hashing a list of names.
    ///
    /// Typically, this will be the path from the animation root to the
    /// animation target (e.g. bone) that is to be animated.
    pub fn from_names<'a>(names: impl Iterator<Item = &'a Name>) -> Self {
        let mut blake3 = blake3::Hasher::new();
        blake3.update(ANIMATION_TARGET_NAMESPACE.as_bytes());
        for name in names {
            blake3.update(name.as_bytes());
        }
        let hash = blake3.finalize().as_bytes()[0..16].try_into().unwrap();
        Self(*uuid::Builder::from_sha1_bytes(hash).as_uuid())
    }

    /// Creates a new [`AnimationTargetId`] by hashing a single name.
    pub fn from_name(name: &Name) -> Self {
        Self::from_names(iter::once(name))
    }
}

impl<T: AsRef<str>> FromIterator<T> for AnimationTargetId {
    /// Creates a new [`AnimationTargetId`] by hashing a list of strings.
    ///
    /// Typically, this will be the path from the animation root to the
    /// animation target (e.g. bone) that is to be animated.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut blake3 = blake3::Hasher::new();
        blake3.update(ANIMATION_TARGET_NAMESPACE.as_bytes());
        for str in iter {
            blake3.update(str.as_ref().as_bytes());
        }
        let hash = blake3.finalize().as_bytes()[0..16].try_into().unwrap();
        Self(*uuid::Builder::from_sha1_bytes(hash).as_uuid())
    }
}

impl From<&Name> for AnimationTargetId {
    fn from(name: &Name) -> Self {
        AnimationTargetId::from_name(name)
    }
}

impl AnimationEvaluationState {
    /// Calls [`AnimationCurveEvaluator::blend`] on all curve evaluator types
    /// that we've been building up for a single target.
    ///
    /// The given `node_index` is the node that we're evaluating.
    fn blend_all(
        &mut self,
        node_index: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        for curve_evaluator_type in self.current_evaluators.keys() {
            self.evaluators
                .get_mut(curve_evaluator_type)
                .unwrap()
                .blend(node_index)?;
        }
        Ok(())
    }

    /// Calls [`AnimationCurveEvaluator::add`] on all curve evaluator types
    /// that we've been building up for a single target.
    ///
    /// The given `node_index` is the node that we're evaluating.
    fn add_all(&mut self, node_index: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        for curve_evaluator_type in self.current_evaluators.keys() {
            self.evaluators
                .get_mut(curve_evaluator_type)
                .unwrap()
                .add(node_index)?;
        }
        Ok(())
    }

    /// Calls [`AnimationCurveEvaluator::push_blend_register`] on all curve
    /// evaluator types that we've been building up for a single target.
    ///
    /// The `weight` parameter is the weight that should be pushed onto the
    /// stack, while the `node_index` parameter is the node that we're
    /// evaluating.
    fn push_blend_register_all(
        &mut self,
        weight: f32,
        node_index: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        for curve_evaluator_type in self.current_evaluators.keys() {
            self.evaluators
                .get_mut(curve_evaluator_type)
                .unwrap()
                .push_blend_register(weight, node_index)?;
        }
        Ok(())
    }

    /// Calls [`AnimationCurveEvaluator::commit`] on all curve evaluator types
    /// that we've been building up for a single target.
    ///
    /// This is the call that actually writes the computed values into the
    /// components being animated.
    fn commit_all(
        &mut self,
        mut entity_mut: AnimationEntityMut,
    ) -> Result<(), AnimationEvaluationError> {
        self.current_evaluators.clear(|id| {
            self.evaluators
                .get_mut(id)
                .unwrap()
                .commit(entity_mut.reborrow())
        })
    }
}
