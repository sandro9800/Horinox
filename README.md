# HORIXON

**HORIXON** is a native Rust client mod for Horizon / Minecraft Java 26.3. It ports Meteor Client's module engine and systems while replacing its ClickGUI with an original HORIXON Command Center: a cyan/navy full-screen interface with category navigation, search, module toggles, settings and a HUD editor.

This repository is a **native `.hnmod` source bundle**, not a Fabric `.jar` and not a precompiled executable. Horizon Launcher builds the native Rust package for the selected instance. Native mods run with your operating-system permissions; review the source and trust only code you intend to run.

## Install

1. Open Horizon Launcher and select your Minecraft 26.3 instance.
2. Open **Mods → Add from GitHub**.
3. Enter `https://github.com/sandro9800/Horinox` (or pin a specific commit).
4. Review the native-code warning, trust the source, and build the mod for that instance.
5. Join a world and press **Right Shift** to open HORIXON.

The launcher builds the native package for that instance. The `.hnmod` release artifact is a source bundle containing the same manifest and engine folder; it is not a precompiled game binary.

## HORIXON Command Center

- **Right Shift** — open the full-screen GUI; **Escape** or Right Shift closes it.
- **Category sidebar** — Combat, Player, Movement, Render, World and Misc.
- **Search** — filters modules across categories.
- **Module list** — select a module; click its ON/OFF badge to toggle it.
- **Settings panel** — configure booleans, sliders and enum settings for the selected module.
- **Mouse wheel** — scroll the module list or settings panel under the cursor.
- **Arrow keys** — navigate modules/categories. With settings focus active, Up/Down selects settings and Left/Right adjusts sliders or enum choices.
- **Enter/Space** — toggle the selected module, or activate the focused setting.
- **Tab** — enter/exit settings focus; **H** — toggle the HUD editor and drag HUD elements.

The GUI is HORIXON-specific; the imported module engine and its original license notices remain attributed to their upstream authors.

## Ported systems

- **167 expected module registrations** across Combat (29), Player (31), Movement (31), Render (38), World (21), Misc (16) and one example module. The registry's `expected_count()` is the source of this count; actual feature availability depends on the Horizon API.
- Meteor-style module manager, keybinds, typed settings, commands, SNBT configuration, friends, HUD elements and HUD editor.
- Native Horizon screen hooks for keyboard, mouse clicks, dragging, scrolling and character input.
- A HORIXON cyan/navy theme and HORIXON-branded HUD defaults.

**Port status:** the source currently contains 284 `PORT GAP` markers documenting Java behaviours that Horizon's public native API does not expose or that still need a faithful implementation. Module definitions are not a guarantee that every Java-side behaviour is fully equivalent. See [PORT-MAP.md](PORT-MAP.md) and [VALIDATION.md](VALIDATION.md).

## Build the `.hnmod` source bundle

Python 3.11+ is sufficient to package the source bundle:

```sh
python tools/pack_bundle.py
python tools/validate_bundle.py
```

Output: `target/horixon-0.5.0.hnmod`. GitHub Actions builds and validates the same artifact on every push and attaches it to tagged releases. A manual workflow run or version tag also runs a Cargo compile check against the exact pinned Horizon source.

## Compatibility

- Minecraft: Java Edition **26.3**
- Horizon revision: `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0`
- Native API bundle entry: `horizon-mixin.toml` → `[api] path = "engine"`

The Horizon revision in `horizon-mixin.toml` is authoritative. Other Nightlies are not claimed compatible until the manifest pin and native build are checked against them.

## License

HORIXON is distributed under **GNU GPL-3.0-or-later**. It incorporates a GPL-licensed Meteor Client port for Horizon; the original license and attribution are retained. See [LICENSE](LICENSE) and [UPSTREAM.md](UPSTREAM.md).
