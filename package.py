#!/usr/bin/env python3
"""Deterministic packager for Horizon native mod bundles (.hnmod).

Produces a byte-for-byte reproducible .hnmod bundle according to the official
Horizon Native Mod specification.
"""

import argparse
import hashlib
import json
import sys
import tomllib
import zipfile
from pathlib import Path


def package(mod_dir: Path, output_dir: Path) -> Path:
    mod_dir = mod_dir.resolve()
    compat_path = mod_dir / "compatibility.json"
    if not compat_path.is_file():
        sys.exit(f"Error: missing compatibility.json in {mod_dir}")

    compat = json.loads(compat_path.read_text(encoding="utf-8"))

    mixins_dir = mod_dir / "mixins"
    if not mixins_dir.is_dir():
        sys.exit(f"Error: missing mixins/ directory in {mod_dir}")

    bundle_dir = None
    for sub in sorted(mixins_dir.iterdir()):
        if sub.is_dir() and (sub / "horizon-mixin.toml").is_file():
            bundle_dir = sub
            break

    if not bundle_dir:
        sys.exit(f"Error: could not find mixin bundle with horizon-mixin.toml in {mixins_dir}")

    manifest_path = bundle_dir / "horizon-mixin.toml"
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    mod_id = manifest["mod"]["id"]
    version = manifest["mod"]["version"]

    pinned = manifest.get("compatibility", {}).get("horizon_revision")
    compat_rev = compat.get("horizon_revision")
    if pinned and compat_rev and pinned != compat_rev:
        sys.exit(f"Error: horizon-mixin.toml pins {pinned}, but compatibility.json specifies {compat_rev}")

    horizon_native_manifest = {
        "game": compat.get("game", "26.3"),
        "hooks": compat.get("hooks", 1),
        "horizon": compat_rev,
        "id": mod_id,
        "mixins": compat.get("horizon_mixins_revision", "0d6b103c70633d8fd9443d8d0acd3aadaacedb06"),
        "schema": 1,
        "version": version,
    }

    files = {
        "horizon-native.json": (json.dumps(horizon_native_manifest, indent=2, sort_keys=True) + "\n").encode("utf-8")
    }

    # Mixin bundle source files
    for path in sorted(bundle_dir.rglob("*")):
        if path.is_file() and path.suffix in (".rs", ".toml"):
            arc_name = f"mixins/{mod_id}/{path.relative_to(bundle_dir).as_posix()}"
            files[arc_name] = path.read_bytes()

    # License file inclusion
    license_file = mod_dir / "LICENSE"
    if license_file.is_file():
        files.setdefault(f"mixins/{mod_id}/LICENSE", license_file.read_bytes())

    output_dir.mkdir(parents=True, exist_ok=True)
    out_file = output_dir / f"{mod_id}-{version}.hnmod"

    with zipfile.ZipFile(out_file, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.create_system = 3  # Unix POSIX attribute compatibility
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, data)

    return out_file


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mod", type=Path, default=Path(__file__).parent, help="Root directory of the native mod")
    parser.add_argument("--out", type=Path, default=Path(__file__).parent.parent.parent / "builds", help="Output directory")
    args = parser.parse_args()

    out_file = package(args.mod, args.out)
    data = out_file.read_bytes()
    sha256 = hashlib.sha256(data).hexdigest()
    result = {
        "file": str(out_file),
        "size": len(data),
        "sha256": sha256
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()

