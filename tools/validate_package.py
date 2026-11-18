#!/usr/bin/env python3
import json
import tomllib
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parents[1]
compat = json.loads((root / "compatibility.json").read_text())
manifest = tomllib.loads((root / "mixins/horixon/horizon-mixin.toml").read_text())
mod = manifest["mod"]
path = root / "target" / f"{mod['id']}-{mod['version']}.hnmod"
assert path.is_file(), f"missing package: {path}"
with zipfile.ZipFile(path) as archive:
    names = archive.namelist()
    assert len(names) == len(set(names)), "duplicate archive paths"
    assert "horizon-native.json" in names
    assert f"mixins/{mod['id']}/horizon-mixin.toml" in names
    native = json.loads(archive.read("horizon-native.json"))
    assert native["schema"] == 1
    assert native["id"] == mod["id"]
    assert native["version"] == mod["version"]
    assert native["game"] == compat["game"]
    assert native["horizon"] == compat["horizon_revision"]
    assert native["mixins"] == compat["horizon_mixins_revision"]
    for name in names:
        if name == "horizon-native.json":
            continue
        assert name.startswith(f"mixins/{mod['id']}/")
        if name.endswith(".toml"):
            tomllib.loads(archive.read(name).decode())
        elif name.endswith(".rs"):
            assert archive.read(name), f"empty source: {name}"
        else:
            raise AssertionError(f"unexpected file in native bundle: {name}")
print(f"validated {path} ({path.stat().st_size} bytes, {len(names)} entries)")
