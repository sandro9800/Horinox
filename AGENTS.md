# HORIXON contributor notes

- This repository builds a Horizon native source bundle (`.hnmod`) from `mixins/horixon/`.
- Keep `compatibility.json` and `mixins/horixon/horizon-mixin.toml` pinned to the same Horizon commit.
- Use `python3 tools/horizon.py package` to create the deterministic `.hnmod`.
- Before claiming game compatibility, run `python3 tools/horizon.py check --horizon ../horizon` with the exact pinned Horizon checkout and pinned `horizon-mixins` checkout.
- Run `cargo test --workspace --locked` for the pure Rust runtime tests.
- Native code runs with the user's operating-system permissions. Never add telemetry, credential access, stealth, persistence outside Horizon's mod settings, or anti-cheat bypasses.
- If adapting Meteor source, preserve GPL-3.0 and applicable copyright notices; do not mix copied GPL code into this MIT project without resolving licensing first.
