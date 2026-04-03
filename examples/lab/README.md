# Character State Machine Lab

Crate-local standalone lab app for inspecting the shared `character_state_machine` crate in a real Bevy application.

## Purpose

- verify the shared crate in a real app, not only as a standalone example
- expose current state, previous state, state stack, pending request, timing, and last rejection reason through an on-screen overlay
- keep a direct Bevy `AnimationPlayer` integration path available for runtime inspection

## Status

Working

## Run

```bash
cargo run -p character_state_machine_lab
```

## E2E

```bash
cargo run -p saddle-character-state-machine-lab --features e2e -- state_machine_smoke
cargo run -p saddle-character-state-machine-lab --features e2e -- state_machine_airborne
cargo run -p saddle-character-state-machine-lab --features e2e -- state_machine_actions
```

The lab names the main actor `State Machine Hero` and the overlay `Character State Machine Overlay`, so BRP and E2E helpers can locate them without relying on private marker types.

## Notes

- Keyboard controls:
  - `W` move / locomotion
  - `Space` jump
  - `J` attack push
  - `R` reload push
  - `E` emote push
- The animation clips are programmatic. No external assets are required for the lab.
