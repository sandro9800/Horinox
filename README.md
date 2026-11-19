# HORIXON


**HORIXON** is a native Rust mod for Horizon / Minecraft **26.3**, built as a flat, official **`.hmod`** package. It ports a Meteor-derived module engine while using an original HORIXON Command Center GUI.


## Package and build

HORIXON follows Horizon's native mod project layout:


- `horizon-mod.toml` — mod identity, API version and pinned game/Horizon/mixins revisions.
- `Cargo.toml` / `Cargo.lock` — Rust package and dependency lock.
- `src/` — native mod source.
- `assets/` and `data/` — optional resources.


With Rust and the official `cargo-horizon-mod` installed from the Horizon repository:


```sh
cargo horizon-mod check --uses
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/horixon.hmod
```

The package is written to `target/horizon-mod/horixon.hmod`. CI builds and validates this archive and uploads it as the `horixon-hmod` artifact.

## Install

Use the Horizon Launcher Mods page to install HORIXON from GitHub, or run:

```sh
horizon-launcher mods add-github https://github.com/sandro9800/Horinox --trust-native-code
```

Native mods execute with your computer’s permissions, outside a sandbox. Review the source before trusting it. After installation, build the native mods for your instance through Horizon Launcher.


## In-game controls

- **Right Shift** — open/close the HORIXON Command Center.
- **Escape** — close it.
- **H** — HUD editor.
- **Tab** — settings focus; arrows adjust/select and Enter/Space activate.


## Current scope

The source contains 167 expected module registrations across combat, player, movement, render, world and misc. Port completeness varies: documented `PORT GAP` markers identify behavior that is not yet fully implemented or verified against Horizon. A native package/API build does not prove in-game functionality; test on a trusted Horizon instance before use.


## License

GPL-3.0-or-later. Upstream Meteor-derived code and attribution are documented in `UPSTREAM.md`.
