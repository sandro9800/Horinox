# HORIXON

**HORIXON** is a native Rust client project for Horizon, targeting Minecraft Java Edition 26.3. The goal is a faithful, feature-by-feature port of Meteor Client functionality into Horizon's native Rust mixin system, with a completely original HORIXON interface and interaction design.

This repository is an early foundation, not a completed client. The current code is the renamed native Horizon template example: it demonstrates one HEAD hook and a testable Rust runtime. No Meteor source or gameplay modules have been ported yet, and Minecraft 26.3 compatibility has not yet been verified in-game.

## Project direction

- **Native implementation:** Rust code injected through Horizon Mixins. This is not a Fabric mod, Java client, or ordinary .hmod content mod.
- **Feature parity goal:** reimplement Meteor's module categories and behavior systematically, documenting differences where Horizon's APIs or Minecraft version require them.
- **Original interface:** HORIXON will have its own GUI, visual language, settings screens, module list, and HUD. It will not copy Meteor's GUI.
- **Compatibility first:** every feature must be checked against the exact pinned Horizon and mixin revisions before being marked supported.
- **Transparent status:** planned features are not presented as implemented; compilation alone does not prove in-game behavior.

## Current status

| Area | Status |
|---|---|
| Native Horizon build scaffold | Present |
| Pinned Horizon / Horizon Mixins revisions | Present |
| HORIXON module identity and runtime | Initial rename complete |
| Original GUI and HUD | Not implemented |
| Module registry and settings system | Not implemented |
| Meteor feature port | Not started |
| In-game validation | Not performed for HORIXON |

## Build requirements

The upstream template currently documents and validates this toolchain: Linux, Rust 1.95.0, Python 3.12+, Git, and Cargo. The staging tools use Linux file locks. Windows and macOS are not validated; on Windows, use WSL2 with a Linux distribution.

## Build

Clone this repository and the exact upstream source repositories beside it:

```sh
git clone https://github.com/sandro9800/Horinox.git
git clone https://github.com/horizon-rust/horizon ../horizon
git clone https://github.com/horizon-rust/horizon-mixins ../horizon-mixins
```

The compatible revisions are recorded in compatibility.json. Check out those exact revisions before staging. Then run:

```sh
cargo fetch --locked
cargo test --workspace --locked --offline
python3 tools/test_template.py
python3 tools/test_stage.py --horizon ../horizon
python3 tools/horizon.py check --horizon ../horizon
python3 tools/horizon.py test --horizon ../horizon
python3 tools/horizon.py build --horizon ../horizon
```

See VALIDATION.md for the distinction between automated checks and manual in-game validation.

## Architecture

- mixins/horixon/horizon-mixin.toml declares the native module and its hooks.
- mixins/horixon/runtime.rs contains the shared, independently testable runtime logic.
- mixins/horixon/break.rs adapts the runtime to the pinned Horizon game hook.
- tools/horizon.py stages the pinned upstream source and invokes the Horizon Mixins plan/apply/build pipeline.
- tools/audit/ verifies that generated AST changes match the declared mixin plan.

All native AST changes must go through Horizon Mixins' plan/apply flow. Do not rewrite Horizon source with string substitutions or edit the upstream checkout.

## Roadmap

1. Establish a clean native HORIXON module and reproducible validation.
2. Inspect the pinned Horizon APIs and design the original GUI/HUD integration.
3. Build the event, module registry, settings, and configuration foundations.
4. Port Meteor functionality in small, testable groups, preserving feature behavior where the target APIs permit it.
5. Validate each feature in-game on the supported Minecraft/Horizon build and publish an explicit compatibility matrix.

## Licensing and attribution

The current scaffold is based on the MIT-licensed Horizon native mod template and retains its upstream notices. No Meteor source has been copied into this repository at this stage. Any future adaptation of Meteor source must preserve its GPL-3.0 license and notices; the repository's licensing and third-party notices will be updated before such code is introduced. See LICENSE.
