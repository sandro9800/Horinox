# HORIXON validation


## Package checks

The CI workflow uses the official `cargo-horizon-mod` CLI from the pinned Horizon SDK build. It:


1. checks native and wasm API compatibility and the mod's declared API usage;
2. builds the flat native `horixon.hmod` package;
3. runs the repository's archive/manifest consistency validator;
4. runs Horizon's official package check against the generated `.hmod`;
5. uploads the `.hmod` artifact.


## Target

- Minecraft: `26.3`
- Horizon SDK build: `dd67b76ff23e28c6d8e21859df600d31572f81a0`
- Horizon mixins revision: `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`
- Native package: `horizon-mod.toml` + `Cargo.toml` + `src/` in a flat `.hmod`
- License: GPL-3.0-or-later


## Still requiring runtime validation

CI package/API checks are not a substitute for the Horizon Launcher compiling the mod into an installed game build and an in-game test. The 167 module registrations are not all guaranteed to be feature-complete Meteor equivalents; the documented `PORT GAP` markers remain. The project must not be described as game-ready until those runtime checks are performed.
