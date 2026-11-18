# Validation status

## Completed checks

- `rustfmt --edition 2021 --check`: **PASS** on all 246 Rust source files using Rust 1.99.1.
- `python tools/pack_bundle.py`: **PASS**; deterministic archive contains 252 files and is 668,792 bytes.
- `python tools/validate_bundle.py`: **PASS**; native manifest, package identity, GPL license, source layout and required GUI hooks validated.
- `.hnmod` SHA-256: `a995837afa075e46027f30f7f79e9c9c798d82406ad92b77edaaa2b97a67abd1`.

## Native integration status

The source targets Horizon revision `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` and uses the native trait-mod API. The HORIXON GUI is a native screen and uses screen-specific input hooks so key presses, mouse clicks, drag, scrolling and text input can be consumed without leaking into gameplay. The engine depends on `horizon-client` with default features disabled because it consumes only the generated `mod_api`; the optional CI `native-check` job stages the engine into the pinned Horizon workspace and runs `cargo check -p horixon-engine`.

The first native-check attempt exposed eight compile errors: six screen-hook `Host` mutability mismatches and two `Value::Double` precision-field mismatches. Those have been corrected in the follow-up source commit; the native-check workflow must be rerun to confirm the fix.

**Still not verified:** a successful native engine compile after those fixes, a build through Horizon Launcher, and an in-game test on a prepared Minecraft 26.3 instance. A valid `.hnmod` archive and Rust syntax/format checks do not prove that every module behaves exactly like Java Meteor. The launcher build and runtime test remain the final gates.
