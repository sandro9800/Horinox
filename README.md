# HORIXON

**HORIXON** is a native Rust client project for Horizon, targeting Minecraft Java Edition 26.3. Its goal is a feature-by-feature reimplementation of Meteor Client functionality using Horizon's native Rust mixin system, with a completely original interface and visual design.

This is an experimental native client. It is not a Fabric mod, Java client, or ordinary `.hmod` content mod. Native code runs with the game's user privileges; review every change before building or trusting it.

## Current implementation

- Native HORIXON module registered in Horizon's Mods settings.
- Original top-left HUD with live FPS and player coordinates/dimension.
- Keyboard-driven GUI opened/closed with Right Shift.
- Six categories: Combat, Movement, Render, Player, World, Misc.
- First working Render category: ArrayList, Coordinates, and FPS.
- Up/Down selects modules; Space/Enter toggles them; Escape closes the GUI.
- Module states persist in Horizon's native mod-data storage.
- ArrayList renders enabled modules at the top-right; Coordinates and FPS can be hidden independently.
- Empty categories are placeholders until their modules are implemented.

This is an initial module registry, not yet a complete Meteor port. Mouse interaction, a general typed-settings framework, search, and the remaining Meteor feature ports are still unimplemented. Minecraft 26.3 in-game validation has not yet been completed.

## Build requirements

- Linux (the staging tools use Linux file locks).
- Rust 1.95.0.
- Python 3.12+.
- Git and Cargo.

On Windows, use WSL2 with a Linux distribution. Windows and macOS staging are not validated.

## Build

Clone this repository and the pinned upstream sources beside it:

```sh
git clone https://github.com/sandro9800/Horinox.git
git clone https://github.com/horizon-rust/horizon.git ../horizon
git clone https://github.com/horizon-rust/horizon-mixins.git ../horizon-mixins
```

Use the exact revisions in `compatibility.json`, then run:

```sh
cargo fetch --locked
cargo test --workspace --locked --offline
python3 tools/horizon.py stage --horizon ../horizon
python3 tools/horizon.py check --horizon ../horizon
python3 tools/horizon.py test --horizon ../horizon
python3 tools/horizon.py build --horizon ../horizon
```

The generated output is a custom Horizon executable. Native mod code is not sandboxed.

## Licensing

The initial scaffold comes from the MIT-licensed Horizon native mod template and retains its upstream notices. No Meteor source has been copied into this repository. Any future adaptation of Meteor source must preserve its GPL-3.0 license and notices.
