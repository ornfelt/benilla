# Quake 3 movement

The `q3_movement` cargo feature (off by default) builds in Quake 3 strafe jumping and bunny
hopping, with a ups speedometer. It is a client-side hack: the client moves faster than WoW lets
it and reports those positions to the server. So it is only for a local vmangos with its movement
anticheat off. The code is `crates/benilla-app/src/q3move.rs`, and every hook into the mover,
controller and bindings is behind `#[cfg(feature = "q3_movement")]`. Without the feature, the
client is unchanged.

## The server

In vmangos's `mangosd.conf`, turn the movement anticheat off and restart `mangosd`:

```ini
Anticheat.Enable = 0
```

With the anticheat on, the server rejects the faster positions, and you get pulled back or kicked.

## Build and run

The usual run, with the feature added (`WOW_DATA` and the rest are as in the README):

```powershell
$env:WOW_DATA="C:\path\to\WoW\Data"; cargo run --release -p benilla --features q3_movement

# Using an existing environment variable
$env:WOW_DATA="$env:wow_classic_dir\Data"; cargo run --release -p benilla --features q3_movement
```

```sh
WOW_DATA=/path/to/WoW/Data cargo run --release -p benilla --features q3_movement

# Using an existing environment variable
WOW_DATA="$wow_classic_dir/Data" cargo run --release -p benilla --features q3_movement
```

Leave out `--release` for the debug build. For the player build, add `--no-default-features`
(`cargo run --release -p benilla --no-default-features --features q3_movement`). That build has no
dev chords, so Ctrl+Shift+Q does nothing there, and only the console command switches modes.

## In game

- **It starts on.** Ctrl+Shift+Q switches between Quake 3 and WoW movement. So does
  `/console q3move` in chat; `/console q3move on` and `/console q3move off` set it. The command's
  reply goes to the log, but the speedometer shows the mode either way.
- **The speedometer.** A pill under the character shows the horizontal speed in ups (Quake 3 units
  per second, 36 per yard, so the 7 yd/s run reads 252 ups), then the speed in yd/s and the mode,
  `q3` or `wow`: `412 ups  11.4 yd/s q3`.
- **On the ground.** VQ3 friction and acceleration (`pm_friction` 6, `pm_accelerate` 10,
  `pm_stopspeed` 100 ups). The top speed is still WoW's live speed (run, walk, backward, mounts and
  buffs), so you can't run faster than normal; you only get there and stop a little less sharply.
- **Strafe jumping.** In the air, `pm_airaccelerate` 1 and no friction. Hold forward and a strafe
  key, and turn the view with the mouse toward the strafe side while in the air. Hold the right
  mouse button so the mouse turns the character, which also makes A/D strafe instead of turning.
  Swap the strafe key and turn direction from one jump to the next. Each jump adds speed: about
  410 ups after the first good one, about 1000 ups after ten.
- **Bunny hopping.** Hold Space: you jump again on the frame you land, so there is no ground frame
  with friction between hops. Let go of Space to stop, and ground friction slows you down.
- **Walls.** A wall hit in the air takes the speed going into it, as in Quake 3, and never adds
  any.
- **What stays WoW's.** Gravity, the jump height, swimming, roots, the hold after a teleport, and
  the frame a knockback launches you.

## Tests

The physics tests are in `q3move.rs`. The existing mover tests check WoW movement, so they turn
Quake 3 movement off for their own thread. Both builds should pass:

```powershell
cargo test -p benilla-app --features q3_movement -- player:: q3move:: bindings:: console::
cargo test -p benilla-app -- player:: bindings:: console::
```

Each one builds the whole crate's tests, which takes a lot of memory. Run them one at a time.

For a quick test rebuild (after an upstream merge, say), add `--profile dev-fast`. Our crates build
at opt-level 0 with no debuginfo, so they compile faster and with less memory, but run slower:

```powershell
cargo test -p benilla-app --profile dev-fast --features q3_movement -- player:: q3move:: bindings:: console::
cargo test -p benilla-app --profile dev-fast -- player:: bindings:: console::
```
