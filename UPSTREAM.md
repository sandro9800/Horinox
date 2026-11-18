# Upstream and attribution

HORIXON's module engine is derived from the public native Horizon port:

- Repository: https://github.com/ShiraiFyuya/meteor-horizon
- Imported source commit: `3c0e720e502465755cae5fb01ee492b2cf76c0cf`
- Imported source tree: `851249ab9346efc5d794055b973ff9b5380948f9`
- License: GNU GPL-3.0-or-later (the upstream `LICENSE` text is retained)
- Original project: https://github.com/MeteorDevelopment/meteor-client

The imported tree supplied the Meteor-compatible module registry, module implementations, settings, command/config/friends/HUD systems and Horizon native API adapter. HORIXON-specific changes include the mod identity (`horixon`), version and packaging, branding/theme changes, a new full-screen Command Center GUI, and native screen input routing for GUI mouse/keyboard/text/scroll events.

The upstream README described a 169-module count and an older Horizon revision. HORIXON follows the source manifest's pinned Horizon revision (`9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`) and the registry's `expected_count()` value (167). The source also contains explicit `PORT GAP` comments; see `PORT-MAP.md`.

All original copyright headers and upstream license notices in source files are preserved. HORIXON modifications are distributed under the same GPL terms. No Meteor source is relicensed as MIT or proprietary.
