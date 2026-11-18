# HORIXON

**HORIXON** is a native Rust client project for Horizon, targeting Minecraft Java Edition 26.3. Its goal is a feature-by-feature reimplementation of Meteor Client functionality using Horizon's native Rust mixin system, with a completely original interface and visual design.

This is an early, experimental native mod. It is not a Fabric mod, Java client, or ordinary .hmod content mod. Native code runs with the game's user privileges; only install and build code you have reviewed and trust.

## Current implementation

- Native HORIXON module registered in Horizon's Mods settings.
- Original top-left HUD with live FPS and player coordinates/dimension.
- Keyboard-driven GUI shell: Right Shift opens/closes; Left/Right changes the selected category; Escape closes.
- Six navigation categories: Combat, Movement, Render, Player, World, Misc.
- HUD respects the native mod enable/disable setting and avoids Horizon's occupied debug-screen corner.
- Unit tests cover HUD formatting and category identity.

The category screens do not yet contain Meteor modules. Mouse interaction, module registry/settings framework, and Meteor feature ports remain unimplemented. Minecraft 26.3 is the declared target; in-game validation has not yet been performed.

## Build requirements

- Linux (the staging tools use Linux file locks)
- Rust 1.95.0
- Python 3.12+
- Git and Cargo

On Windows, use WSL2 with a Linux distribution. Windows and macOS staging are not validated.

## Build

Clone this repository and the pinned upstream sources beside it:

```sh
git clone https://github.com/sandro9800/Horinox.git
git clone https://github.com/horizon-rust/horizon ../horizon
git clone https://github.com/horizon-rust/horizon-mixins ../horizon-mixins
```

Use the exact revisions in compatibility.json, then run:

```sh
cargo fetch --locked
cargo test --workspace --locked --offline
python3 tools/test_template.py
python3 tools/test_stage.py --horizon ../horizon
python3 tools/horizon.py check --horizon ../horizon
python3 tools/horizon.py test --horizon ../horizon
python3 tools/horizon.py build --horizon ../horizon
```

The generated output is a custom Horizon executable. Native mod code is not sandboxed; review every change before trusting/building it.

## Roadmap

1. Native module and HUD foundation.
2. Finish the original GUI with module lists, search, mouse input, and settings controls.
3. Event bus, module registry, typed settings, and persistent configuration.
4. Feature-by-feature Meteor behavior reimplementation, with explicit compatibility notes.
5. Automated and in-game validation for each supported feature.

## Licensing

The initial scaffold comes from the MIT-licensed Horizon native mod template and retains its upstream notices. No Meteor source has been copied into this repository. Any future adaptation of Meteor source must preserve its GPL-3.0 license and notices.
