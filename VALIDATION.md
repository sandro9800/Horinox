# Validation status

## Package

The project targets Minecraft Java 26.3. Its native bundle is pinned by `compatibility.json` and `mixins/horixon/horizon-mixin.toml`.

The package command is `python3 tools/horizon.py package`; it creates a deterministic `.hnmod` source archive. The package validator checks the archive entries, JSON manifest, TOML bundle manifest, identity, version and compatibility pins.

## Runtime

The runtime has unit tests in `mixins/horixon/runtime.rs`. CI runs the workspace tests and validates the `.hnmod` artifact.

## Not yet verified

A package check does not prove that the mod compiles against a live Horizon checkout or runs in-game. The full `tools/horizon.py check --horizon ../horizon` and an in-game test still need to be run against the exact pinned Horizon revision and a prepared Minecraft 26.3 resource set. Do not treat package validation as an in-game compatibility result.
