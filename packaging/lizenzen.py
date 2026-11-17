#!/usr/bin/env python3
"""Erzeugt assets/lizenzen.txt aus `cargo metadata` (externe Fremdbausteine)."""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    cmd = ["cargo", "metadata", "--format-version", "1", "--locked"]
    if len(sys.argv) > 1:
        cmd.extend(["--filter-platform", sys.argv[1]])

    res = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True)
    meta = json.loads(res.stdout)

    zeilen = []
    for pkg in meta.get("packages", []):
        if pkg.get("source") is None:
            continue
        name = pkg["name"]
        version = pkg["version"]
        lizenz = pkg.get("license") or pkg.get("license_file") or "unbekannt"
        zeilen.append((name.lower(), version, f"{name} {version}\t{lizenz}"))

    zeilen.sort(key=lambda item: (item[0], item[1]))
    ziel = ROOT / "assets" / "lizenzen.txt"
    ziel.write_text("\n".join(z[2] for z in zeilen) + "\n", encoding="utf-8")
    print(f"{len(zeilen)} Fremdbausteine nach {ziel.relative_to(ROOT)} geschrieben.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
