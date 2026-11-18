# HORIXON contributor notes

- This repository is a Horizon native `.hnmod` bundle. The root manifest is `horizon-mixin.toml`; the native API crate is `engine/`.
- Keep `[mod].id = "horixon"`, the engine package version, and the manifest version in sync.
- The exact compatibility pin is `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`. Do not silently change it; verify the new revision against the native API before updating.
- Preserve GPL-3.0-or-later and all upstream Meteor / meteor-horizon copyright headers. Do not add MIT-only or proprietary code to the derivative.
- The GUI is HORIXON-specific. Keep the native screen hooks wired: `screen_render_post`, `screen_key_pressed_pre`, `screen_char_typed_pre`, `screen_mouse_button_pressed_pre`, `screen_mouse_button_released_pre`, `screen_mouse_dragged_pre`, and `screen_mouse_scrolled_pre`.
- Native code runs with the user's OS permissions. Do not add telemetry, credential collection, stealth, persistence outside Horizon's normal mod data, or anti-cheat bypasses.
- Run `python tools/pack_bundle.py` and `python tools/validate_bundle.py` before committing a bundle change. The Horizon Launcher native build remains the final compatibility test.
