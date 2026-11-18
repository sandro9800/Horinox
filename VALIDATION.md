# Validation status

## Completed checks (2026-11-19)

- `cargo fmt --all --check` — passed.
- `cargo test --workspace --locked` — passed, 5 tests.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `python3 tools/horizon.py package` — generated `target/horixon-0.4.0.hnmod` (6,125 bytes).
- `python3 tools/validate_package.py` — passed; 8 archive entries, valid JSON/TOML, matching mod identity and compatibility pins.
- Package SHA-256: `30a93b324a6c8e3d3bf8ab6e8ada663fde671361d14a2ee7162bcf2de5f209ff`.

## Not yet verified

The full `python3 tools/horizon.py check --horizon ../horizon` against the pinned Horizon and `horizon-mixins` checkouts has not yet completed. In-game behaviour also remains unverified; it requires the Horizon launcher, a trusted native build, and prepared Minecraft 26.3 resources. A valid `.hnmod` archive and passing runtime unit tests do not prove an in-game build.
