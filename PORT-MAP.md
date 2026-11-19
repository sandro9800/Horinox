# HORIXON port map


HORIXON is a Meteor-derived Rust client-mod port for Horizon/Minecraft 26.3. Its package is a flat native `.hmod`, using Horizon's current `horizon-mod.toml` manifest and `mod_api` SDK crate. This project does not use the old `.hnmod` source-bundle workflow, hand-written `horizon-mixin.toml`, or a nested engine package layout.


## Ported systems

- Meteor-style module registry and typed settings
- Module categories and search
- Config, commands and friends systems
- HUD elements, rendering and HUD editor
- Native screen input routing for the HORIXON Command Center
- Original dark-navy/cyan Command Center GUI


## Module inventory

167 expected registrations are organized across combat, player, movement, render, world, misc and one example. This is a registration count, not a claim of 167 complete feature ports. Source `PORT GAP` markers identify missing or incomplete game behavior; runtime parity is unverified.


## GUI controls

Right Shift opens/closes the Command Center; Escape closes. Mouse navigation, module toggle, search and settings panels are implemented. Tab focuses settings, Up/Down moves, Left/Right adjusts values, Enter/Space toggles or activates. H opens the HUD editor.


## Known upstream limitation

Discord presence still uses the upstream Meteor Discord application ID. Upstream license and attribution are preserved; see `UPSTREAM.md`.
