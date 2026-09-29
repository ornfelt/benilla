//! The per-instance body tint: a unit's whole model multiplied by one colour, the render channel
//! for the aura kits' CharProc 1 (`benilla-app`'s `aura_visual`).
//!
//! The reference's CM2 instance carries a modulate tint at `model+0x184..0x18c` (setter
//! `0x710cf0`, default white) beside the additive highlight (`0x710d40`), which is
//! [`crate::mesh_tag::HIGHLIGHT_BIT`] here. The animate kernel `0x714260` multiplies it by
//! `inputColorA`, and the batcher clamps it to [0,1] and sets it as the GL ambient and diffuse
//! material colour (`0x70c3a9`–`0x70c4d3`, `0x70c8ad`): it scales the light sum inside the clamp,
//! not emission, where `wow_model.wgsl` applies a WMO batch's MOCV. An unlit batch adds the tinted
//! M2Color and the highlight inside one clamp.
//!
//! The table is indexed by the `MeshTag` rig slot ([`crate::rig_palette`]), so it takes no tag
//! bits; an unskinned mesh reads its slot safely, as skinning keys on the vertex layout, not the
//! tag. Slot 0, the no-rig sentinel, is never written. A `0` word is identity: the reference packs
//! an authored colour as `round(param) | 0xff000000` (`0x60d840`), so black (spell 27200, Defile)
//! is `0xff000000`. The tint applies on change with no easing (`unit+0xd04` into `0x710cf0`),
//! unlike the alpha's 1000 ms ramp (`0x614f80`).

use std::sync::Arc;

use bevy::prelude::*;

use crate::mesh_tag::MAX_RIG_SLOTS;

/// Bytes per slot in the tint region: one packed `0xFFRRGGBB` word.
const SLOT_BYTES: u64 = 4;

/// The no-tint word: zero, so a zeroed buffer is inert.
pub const IDENTITY: u32 = 0;

/// Byte offset of the tint region in a `wow_light`-layout buffer, after the rig slot table and
/// before the runtime-sized palette rows; it must match `wow_model.wgsl`'s struct order.
pub(crate) fn region_offset() -> u64 {
    crate::rig_palette::rig_table_region_offset() + MAX_RIG_SLOTS as u64 * SLOT_BYTES
}

/// Bytes this region adds to every `wow_light`-layout buffer (8 KB at 2048 slots).
pub(crate) fn region_bytes() -> u64 {
    MAX_RIG_SLOTS as u64 * SLOT_BYTES
}

/// Packs RGB as the reference does (`0x60d840`: `param | 0xff000000`), so even black is never
/// [`IDENTITY`].
pub fn pack(rgb: [u8; 3]) -> u32 {
    0xff00_0000 | (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2])
}

/// Off-world `wow_light`-layout buffers that also carry the tint region, by key; deliberately not
/// [`crate::rig_palette::RigPaletteMirrors`]' list. Portraits stay off it: the reference bakes one
/// from a fresh CM2 with colour (1,1,1) and alpha 1 (`0x524f60`, ctor `0x70ea60`, set again at
/// `0x525261` via `0x47a230`), so a ghost's portrait is untinted, and ours carry the world unit's
/// rig slot. The glue scene is on it: its character is the instance the reference tints
/// (`0x472939` into `0x710cf0`, the char-select ghost) and has a slot of its own.
#[derive(Resource, Clone, Default)]
pub struct InstanceTintMirrors(
    pub std::collections::HashMap<&'static str, bevy::render::render_resource::BufferId>,
);

/// The per-slot tint table, indexed by the `MeshTag` rig slot; `Arc`-shared for a cheap clone
/// and generation-stamped so an untinted world uploads nothing.
#[derive(Resource, Clone)]
pub struct InstanceTints {
    slots: Arc<Vec<u32>>,
    generation: u64,
}

impl Default for InstanceTints {
    fn default() -> Self {
        Self {
            slots: Arc::new(vec![IDENTITY; MAX_RIG_SLOTS]),
            generation: 0,
        }
    }
}

impl InstanceTints {
    /// Sets a slot's packed word ([`pack`], or [`IDENTITY`] to clear). Slot 0, shared by everything
    /// unskinned, is never written.
    pub fn set(&mut self, slot: u16, word: u32) {
        let i = slot as usize;
        if i == 0 || i >= MAX_RIG_SLOTS || self.slots[i] == word {
            return;
        }
        Arc::make_mut(&mut self.slots)[i] = word;
        self.generation += 1;
    }

    /// Clears a slot; `RigSkin`'s free hook calls it too, so a reused slot never inherits a dead
    /// unit's tint.
    pub(crate) fn clear(&mut self, slot: u16) {
        self.set(slot, IDENTITY);
    }

    /// This slot's word, for tests (the render path reads the uploaded region); not
    /// `#[cfg(test)]` because `benilla-app`'s aura tests use it.
    pub fn get(&self, slot: u16) -> u32 {
        self.slots.get(slot as usize).copied().unwrap_or(IDENTITY)
    }
}

impl InstanceTints {
    /// For gfx: every slot's word and the generation, which gates the upload.
    pub(crate) fn gfx_slots(&self) -> (&[u32], u64) {
        (self.slots.as_slice(), self.generation)
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<InstanceTints>()
        .init_resource::<InstanceTintMirrors>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_matches_the_reference_and_never_collides_with_identity() {
        // The ghost aura's node value (`round(9222653) | 0xff000000`).
        assert_eq!(pack([0x8c, 0xb9, 0xfd]), 0xff8c_b9fd);
        // Spell 27200 (Defile) authors param0 = 0: a real black tint.
        assert_eq!(pack([0, 0, 0]), 0xff00_0000);
        assert_ne!(pack([0, 0, 0]), IDENTITY);
    }

    #[test]
    fn slot_zero_is_never_written() {
        let mut t = InstanceTints::default();
        t.set(0, pack([255, 0, 0]));
        assert_eq!(t.get(0), IDENTITY);
        assert_eq!(t.generation, 0, "and it costs no upload");
    }

    #[test]
    fn the_generation_tracks_real_changes_only() {
        let mut t = InstanceTints::default();
        assert_eq!(t.generation, 0);
        t.set(7, pack([0x8c, 0xb9, 0xfd]));
        assert_eq!(t.generation, 1);
        t.set(7, pack([0x8c, 0xb9, 0xfd]));
        assert_eq!(t.generation, 1, "same colour, no upload");
        t.clear(7);
        assert_eq!(t.generation, 2);
        assert_eq!(t.get(7), IDENTITY);
        t.clear(7);
        assert_eq!(t.generation, 2, "already clear");
    }

    #[test]
    fn an_out_of_range_slot_is_dropped() {
        let mut t = InstanceTints::default();
        t.set(u16::MAX, pack([1, 2, 3]));
        assert_eq!(t.generation, 0);
        assert_eq!(t.get(u16::MAX), IDENTITY);
    }
}
