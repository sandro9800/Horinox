# HORIXON

**HORIXON** is a native Rust client mod for Horizon (Minecraft Java 26.3). It ships as an installable `.hmod` package and uses Horizon's public `horizon-mod-api`; it does not require a custom Horizon executable or a Fabric loader.

## Current features

- Original dark/cyan module manager UI — open with **Right Shift**; close with Escape or Right Shift.
- Clickable category tabs and module toggles, plus keyboard navigation with Tab, Enter and Space.
- Persistent module settings, also available under Horizon **Options → Mods**.
- **ArrayList** — enabled modules in the top-right HUD.
- **Coordinates** — live XYZ position.
- **FPS** — client frame-rate counter.
- **Compass** — eight-way facing direction.
- **Zoom** — hold **Z** to zoom; scroll while holding to adjust magnification.
- English and Georgian settings/key translations.

The Combat, Movement, Player and World categories are currently scaffolding for subsequent feature ports; they are not falsely advertised as implemented modules.

## Build the `.hmod`

Install the `cargo-horizon-mod` tool from the exact SDK build pinned in `horizon-mod.toml`:

```sh
cargo install --locked --git https://github.com/horizon-rust/horizon --rev 0b14f8acfd97fa2c750e42795249dcc981178ca4 cargo-horizon-mod
cargo horizon-mod check
cargo test
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/horixon.hmod --kind mod
```

The package is written to `target/horizon-mod/horixon.hmod`. GitHub Actions runs the same checks and uploads the `.hmod` as a build artifact; version tags also attach it to a GitHub release.

## Install in Horizon

1. Open Horizon Launcher → your instance → **Mods**.
2. Choose **Add from GitHub** and enter `https://github.com/sandro9800/Horinox`.
3. Review the resolved source revision and the native-code warning before trusting it.
4. Build the installed native mod from the Mods page.
5. Join a world and press **Right Shift**.

Native mods run with your operating-system user permissions. Read the source before trusting or installing any native mod. HORIXON does not include telemetry, credential collection, stealth, or anti-cheat bypass code.

## SDK compatibility

- Game: `26.3`
- Horizon SDK commit: `0b14f8acfd97fa2c750e42795249dcc981178ca4`
- Horizon Mixins tool commit: `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`
- Mod API contract: `0.2`

The launcher compiles the source package against its pinned SDK when building the instance. The `.hmod` is a source package, not a precompiled executable.
