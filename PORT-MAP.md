# Port map and current limitations

## Source layout

| Path | Responsibility |
|---|---|
| `engine/src/meteor_core/` | Engine-agnostic module, settings, command, config, friend, HUD and theme systems ported from Meteor Client |
| `engine/src/port/modules/` | Category module registrations and Horizon-facing module implementations |
| `engine/src/port/hooks.rs` | Horizon native hook adapter; opens the HORIXON native screen and routes keyboard/mouse/scroll/text events |
| `engine/src/port/gui/mod.rs` | Original HORIXON Command Center GUI and HUD editor |
| `engine/src/port/env_impl.rs` | Horizon host/environment adapter and side-effect queue |
| `horizon-mixin.toml` | Native bundle identity, Horizon compatibility pin and engine entry point |

## Expected module registrations

The source registry's `expected_count()` currently sums to 167:

| Category | Expected |
|---|---:|
| Combat | 29 |
| Player | 31 |
| Movement | 31 |
| Render | 38 |
| World | 21 |
| Misc | 16 |
| Example | 1 |
| **Total** | **167** |

This is a registry expectation, not a claim that every module has full Java-equivalent behaviour. The port's test currently checks that at least 150 modules register, because some Java behaviours have no equivalent in Horizon's current API.

## Known API gaps

The imported source contains 284 explicit `PORT GAP` markers. These mark behaviours that still require an API hook, a packet/game primitive not exposed by Horizon, or a more faithful native implementation. Examples include game-specific rendering/input paths and Java client behaviours that cannot be reproduced from the currently exposed host surface. Search the source for `PORT GAP` to inspect each location.

The Discord Presence module still uses the upstream Meteor Discord application identity; its local presence strings are HORIXON-branded, but Discord may show the upstream app identity until a dedicated HORIXON application ID is configured.

## Validation boundary

The `.hnmod` packager validates the native manifest, engine source layout, license and archive contents. It does **not** compile the engine against a Horizon installation or prove in-game behaviour. The final native compile and runtime test must be performed by Horizon Launcher for the pinned Horizon revision. See `VALIDATION.md`.
