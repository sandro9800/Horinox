# Validation status

## Package target

HORIXON is now a native Horizon `.hmod` source project for game `26.3`, using mod API contract `0.2` and Horizon SDK commit `0b14f8acfd97fa2c750e42795249dcc981178ca4`.

## Required checks

```sh
cargo horizon-mod check
cargo test
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/horixon.hmod --kind mod
```

The build workflow runs these checks and uploads `target/horizon-mod/horixon.hmod`. The launcher performs the final native compilation against the selected Horizon instance's SDK when the player chooses Build.

## Important distinction

A successful `.hmod` package check validates the manifest, source layout and package. It does not prove that the native library has been compiled by the Horizon launcher or tested in a running game. Those are separate integration checks and must be reported separately.
