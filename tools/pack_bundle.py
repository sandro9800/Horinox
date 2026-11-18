#!/usr/bin/env python3
"""Create a deterministic Horizon native .hnmod source bundle."""
from __future__ import annotations
import argparse
import hashlib
import tomllib
import zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
INCLUDE_FILES = ["horizon-mixin.toml", "LICENSE", "README.md", "UPSTREAM.md", "PORT-MAP.md"]
INCLUDE_DIRS = ["engine"]
EXCLUDED_PARTS = {".git", ".horizon", "target", "__pycache__"}
def bundle_files() -> list[Path]:
    files = [ROOT / name for name in INCLUDE_FILES]
    for directory in INCLUDE_DIRS:
        base = ROOT / directory
        files.extend(path for path in base.rglob("*") if path.is_file() and not any(part in EXCLUDED_PARTS for part in path.parts) and path.suffix not in {".pyc", ".hnmod"})
    missing = [path.relative_to(ROOT).as_posix() for path in files if not path.is_file()]
    if missing: raise SystemExit(f"Missing bundle inputs: {', '.join(missing)}")
    return sorted(set(files), key=lambda path: path.relative_to(ROOT).as_posix())
def main() -> int:
    parser = argparse.ArgumentParser(); parser.add_argument("--output", type=Path, default=None); args = parser.parse_args()
    manifest = tomllib.loads((ROOT / "horizon-mixin.toml").read_text()); mod = manifest["mod"]
    output = args.output or ROOT / "target" / f"{mod['id']}-{mod['version']}.hnmod"
    output = output if output.is_absolute() else ROOT / output; output.parent.mkdir(parents=True, exist_ok=True)
    files = bundle_files()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in files:
            relative = path.relative_to(ROOT).as_posix(); info = zipfile.ZipInfo(relative, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED; info.create_system = 3; info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes())
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    print(f"Built {output.relative_to(ROOT)}\nFiles: {len(files)} | Bytes: {output.stat().st_size} | SHA-256: {digest}")
    return 0
if __name__ == "__main__": raise SystemExit(main())
