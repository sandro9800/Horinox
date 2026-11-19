#!/usr/bin/env python3
"""Validate HORIXON's flat, native .hmod archive and its manifest pins."""
from __future__ import annotations
import argparse
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HORIZON = "dd67b76ff23e28c6d8e21859df600d31572f81a0"
MIXINS = "e59da9094f0c2fdda6c627fe9eff22dec6a906f0"
REQUIRED = {"horizon-mod.toml", "Cargo.toml", "src/lib.rs", "LICENSE", "README.md", "src/horizon_api_shim.rs", "src/meteor_core/mod.rs", "src/port/gui/mod.rs"}

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=Path, nargs="?", default=ROOT / "target/horizon-mod/horixon.hmod")
    args = parser.parse_args()
    archive = args.archive if args.archive.is_absolute() else ROOT / args.archive
    manifest = tomllib.loads((ROOT / "horizon-mod.toml").read_text())
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
    assert manifest["id"] == "horixon" and manifest["version"] == "0.5.0"
    assert manifest["api"] == "0.2" and manifest["side"] == "client"
    assert manifest["native"]["game"] == "26.3"
    assert manifest["native"]["horizon"] == HORIZON
    assert manifest["native"]["mixins"] == MIXINS and manifest["capabilities"] == []
    assert cargo["package"]["name"] == "mod-horixon" and cargo["package"]["version"] == manifest["version"]
    assert cargo["package"]["license"] == "GPL-3.0-or-later"
    assert archive.is_file(), f"Missing archive: {archive}"
    with zipfile.ZipFile(archive) as z:
        names = z.namelist()
        assert len(names) == len(set(names)), "Duplicate archive paths"
        missing = REQUIRED.difference(names)
        assert not missing, f"Missing required files: {sorted(missing)}"
        assert all(not name.startswith((".git/", "target/", ".horizon/", "engine/")) for name in names)
        assert not any(name.endswith((".hnmod", ".jar", ".exe", ".dll", ".so")) for name in names)
        assert sum(name.endswith(".rs") for name in names) >= 200
        hmod_manifest = tomllib.loads(z.read("horizon-mod.toml").decode())
        assert hmod_manifest == manifest, "Archive manifest differs from project manifest"
        assert b"GNU GENERAL PUBLIC LICENSE" in z.read("LICENSE")
        source = z.read("src/lib.rs").decode()
        assert "#[mod_api::horizon_mod]" in source and "impl Hooks for HorixonHooks" in source
        hooks = source
        for hook in ["screen_render_post", "screen_key_pressed_pre", "screen_mouse_button_pressed_pre", "screen_mouse_scrolled_pre"]:
            assert hook in hooks, f"Missing native GUI hook: {hook}"
        gui = z.read("src/port/gui/mod.rs").decode()
        assert "HORIXON" in gui and "MODULE SETTINGS" in gui
    print(f"Validated flat native .hmod: {len(names)} files, {archive.stat().st_size} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
