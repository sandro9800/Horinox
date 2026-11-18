# HORIXON

**HORIXON** is an experimental native Rust client mod for Horizon / Minecraft Java 26.3. It is distributed as a source-only `.hnmod` bundle. The launcher compiles native code for the selected Horizon instance; this is not a Fabric `.jar` or a content-only `.hmod`.

## Current features

- Original dark/cyan HORIXON HUD and module-manager overlay.
- Right Shift opens/closes the menu; Escape closes it.
- Left/Right switches between Combat, Movement, Render, Player, World and Misc.
- Up/Down selects a module; Space/Enter toggles it.
- Render modules currently implemented: **ArrayList**, **Coordinates** and **FPS**.
- Module states persist in Horizon's native mod-data storage.
- ArrayList lists enabled modules at the top-right; Coordinates and FPS can be toggled independently.
- Horizon Options → Mods exposes clickable settings for the Render modules.

The custom overlay is currently keyboard-driven; it does not yet have mouse hit-testing. Combat, Movement, Player, World and Misc are category scaffolding, not finished feature sets. This is a feature-by-feature port, not a claim that all Meteor modules already exist.

## Build the `.hnmod`

Python 3.11+ is enough to package the source bundle:

```sh
python3 tools/horizon.py package
```

Output: `target/horixon-0.4.0.hnmod`. The packager writes a deterministic archive containing `horizon-native.json` and the `.rs` / `.toml` files in `mixins/horixon/`.

## Build for a Horizon instance

A native mod is source code and runs with your user permissions. Review the source before trusting it. The `.hnmod` is a source bundle, not a precompiled executable. For a local build with the launcher, extract the archive to a folder and run:

```sh
horizon-launcher native-build path/to/unpacked-horixon --trust-native-code --instance "Your Instance"
```

Then prepare and launch that instance. The launcher builds a separate game for the instance; it does not patch your ordinary game installation. Native build support and accepted bundle versions depend on the installed launcher release.

## Compatibility pins

The bundle was authored against the exact Horizon and `horizon-mixins` commits recorded in `compatibility.json` and `mixins/horixon/horizon-mixin.toml`. Those pins must match. Compatibility with other Nightlies is not claimed until the native check is run against them.

## Development checks

```sh
cargo test --workspace --locked
python3 tools/horizon.py package
python3 tools/validate_package.py
```

For the full source-compatibility check, clone Horizon and the pinned `horizon-mixins` repository beside this repository, then run:

```sh
python3 tools/horizon.py check --horizon ../horizon
```

See `VALIDATION.md` for the checks actually completed. No Minecraft assets or player data are included in this repository.
