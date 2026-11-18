#!/usr/bin/env python3
"""Validate the HORIXON native bundle manifest and .hnmod archive."""
from __future__ import annotations
import argparse
import tomllib
import zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
PIN = "9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0"
REQUIRED = {"horizon-mixin.toml", "LICENSE", "README.md", "UPSTREAM.md", "PORT-MAP.md", "engine/Cargo.toml", "engine/src/lib.rs", "engine/src/horizon_api_shim.rs", "engine/src/meteor_core/mod.rs", "engine/src/port/hooks.rs", "engine/src/port/gui/mod.rs"}
def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("archive", type=Path, nargs="?", default=None); args = parser.parse_args()
    manifest = tomllib.loads((ROOT / "horizon-mixin.toml").read_text()); cargo = tomllib.loads((ROOT / "engine/Cargo.toml").read_text()); mod = manifest["mod"]
    assert mod["id"] == "horixon" and mod["version"] == "0.5.0", mod
    assert manifest["compatibility"]["horizon_revision"] == PIN and manifest["api"]["path"] == "engine"
    assert cargo["package"]["name"] == "horixon-engine" and cargo["package"]["version"] == mod["version"]
    assert cargo["package"]["license"] == "GPL-3.0-or-later"
    archive_path = args.archive or ROOT / "target" / f"{mod['id']}-{mod['version']}.hnmod"
    if not archive_path.is_absolute(): archive_path = ROOT / archive_path
    assert archive_path.is_file(), f"Missing archive: {archive_path}"
    with zipfile.ZipFile(archive_path) as bundle:
        names = bundle.namelist(); assert len(names) == len(set(names)), "Duplicate paths in .hnmod"
        missing = REQUIRED.difference(names); assert not missing, f"Missing required bundle files: {sorted(missing)}"
        assert not any(name.startswith((".git/", "target/", "mixins/")) for name in names)
        assert sum(name.endswith(".rs") for name in names) >= 200
        assert b"GNU GENERAL PUBLIC LICENSE" in bundle.read("LICENSE")
        assert b'pub const MOD_ID: &str = "horixon"' in bundle.read("engine/src/meteor_core/mod.rs")
        hooks = bundle.read("engine/src/port/hooks.rs").decode()
        for hook in ["screen_render_post", "screen_key_pressed_pre", "screen_mouse_button_pressed_pre", "screen_mouse_scrolled_pre"]: assert hook in hooks, f"Missing native GUI hook: {hook}"
        gui = bundle.read("engine/src/port/gui/mod.rs").decode(); assert "HORIXON" in gui and "MODULE SETTINGS" in gui
    print(f"Validated {archive_path.relative_to(ROOT)}: {len(names)} files, {archive_path.stat().st_size} bytes")
    return 0
if __name__ == "__main__": raise SystemExit(main())
