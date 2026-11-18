# HORIXON contributor notes

- This repository is a native Horizon `.hmod` source project. Mod identity and SDK pins live only in `horizon-mod.toml`.
- Use `horizon-mod-api` from `.horizon/sdk/api`; do not add a local clone/path to the game repository.
- Do not add `rust-toolchain.toml`, `horizon-native.json`, a hand-written `horizon-mixin.toml`, or a `native/` source directory. The Horizon launcher owns the native build environment.
- Run `cargo horizon-mod check`, `cargo test`, `cargo horizon-mod build`, and `cargo horizon-mod check target/horizon-mod/horixon.hmod --kind mod` before claiming a build passes.
- Keep GUI input routed through `Widgets` and the public screen hooks. Keep HUD drawing inside `gui_render_post` / public DrawContext APIs.
- Store player-facing module toggles as registered `SettingSpec`s; persist custom-screen changes with `ClientHost::set_setting`.
- Native mods execute with the user's permissions. Do not add telemetry, credential access, stealth, persistence outside Horizon's mod settings, or anti-cheat bypasses.
- If adapting Meteor source, preserve its GPL-3.0 license and all applicable copyright notices. Do not mix copied GPL source into this MIT-licensed code without resolving the license change first.
