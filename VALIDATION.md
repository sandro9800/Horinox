# Validation status

## Completed checks

- `rustfmt --edition 2021 --check`: **PASS** on all 246 Rust source files using Rust 1.99.1.
- `python tools/pack_bundle.py`: **PASS**; deterministic archive contains 252 files and is 668,758 bytes.
- `python tools/validate_bundle.py`: **PASS**; native manifest, package identity, GPL license, source layout and required GUI hooks validated.
- `.hnmod` SHA-256: `1c00c8c6c74cf99484413f87481ee472b23f289d6eee2587869b9c771aa43ec7`.

## Native integration status

The source targets Horizon revision `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` and uses the native trait-mod API. The HORIXON GUI is a native screen and uses screen-specific input hooks so key presses, mouse clicks, drag, scrolling and text input can be consumed without leaking into gameplay. The engine depends on `horizon-client` with default features disabled because it consumes only the generated `mod_api`; the optional CI `native-check` job stages the engine into the pinned Horizon workspace and runs `cargo check -p horixon-engine`.

**Not yet verified:** the optional `native-check` workflow has not yet been run, and a successful Cargo check would still not replace a build through Horizon Launcher or an in-game test on a prepared Minecraft 26.3 instance. A valid `.hnmod` archive and Rust syntax/format checks do not prove that every module behaves exactly like Java Meteor. The launcher build and runtime test remain the final gates.
