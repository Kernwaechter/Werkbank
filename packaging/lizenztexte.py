#!/usr/bin/env python3
"""Erzeugt assets/lizenztexte.txt mit den vollständigen Lizenz- und Copyright-Texten."""

import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIZENZ_PREFIXE = ("LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE")
PLATTFORM_NAMEN = {
    "x86_64-unknown-linux-gnu": "Linux-Fassung",
    "x86_64-pc-windows-msvc": "Windows-Fassung",
    "aarch64-apple-darwin": "macOS-Fassung",
}


def finde_lizenzdateien(crate_dir: Path, explizit: str | None) -> list[Path]:
    dateien: list[Path] = []
    seen: set[Path] = set()

    if explizit:
        p = (crate_dir / explizit).resolve()
        if p.is_file():
            dateien.append(p)
            seen.add(p)

    for eintrag in sorted(crate_dir.iterdir(), key=lambda x: x.name.lower()):
        if not eintrag.is_file():
            continue
        upper = eintrag.name.upper()
        if upper.startswith(LIZENZ_PREFIXE) and eintrag.resolve() not in seen:
            dateien.append(eintrag)
            seen.add(eintrag.resolve())

    return dateien


def main() -> int:
    parser = argparse.ArgumentParser(description="Erzeugt assets/lizenztexte.txt")
    parser.add_argument(
        "--filter-platform",
        default="x86_64-unknown-linux-gnu",
        help="Zielplattform für cargo metadata (Standard: x86_64-unknown-linux-gnu)",
    )
    parser.add_argument(
        "--output",
        default=str(ROOT / "assets" / "lizenztexte.txt"),
        help="Ausgabedatei (Standard: assets/lizenztexte.txt)",
    )
    args = parser.parse_args()

    cmd = [
        "cargo",
        "metadata",
        "--format-version",
        "1",
        "--locked",
        "--filter-platform",
        args.filter_platform,
    ]
    res = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True)
    meta = json.loads(res.stdout)

    fassung = PLATTFORM_NAMEN.get(args.filter_platform, args.filter_platform)
    bloecke: list[str] = [f"Lizenztexte der Fremdbausteine der Werkbank ({fassung})\n"]

    externe = [pkg for pkg in meta.get("packages", []) if pkg.get("source") is not None]
    externe.sort(key=lambda p: (p["name"].lower(), p["version"]))

    for pkg in externe:
        name = pkg["name"]
        version = pkg["version"]
        lizenz = pkg.get("license") or pkg.get("license_file") or "unbekannt"
        quelle = pkg.get("repository") or pkg.get("homepage") or ""
        crate_dir = Path(pkg["manifest_path"]).parent
        dateien = finde_lizenzdateien(crate_dir, pkg.get("license_file"))

        kopf = [
            "=" * 78,
            f"{name} {version} – {lizenz}",
        ]
        if quelle:
            kopf.append(quelle)
        kopf.append("")

        if not dateien:
            kopf.append("(Keine separate Lizenzdatei im Crate gefunden.)\n")
        else:
            for datei in dateien:
                kopf.append(f"--- {datei.name} ---")
                inhalt = datei.read_text(encoding="utf-8", errors="replace").rstrip()
                kopf.append(inhalt)
                kopf.append("")

        bloecke.append("\n".join(kopf))

    ziel = Path(args.output)
    ziel.parent.mkdir(parents=True, exist_ok=True)
    ziel.write_text("\n".join(bloecke) + "\n", encoding="utf-8")
    print(f"{len(externe)} Lizenzblöcke ({fassung}) nach {ziel} geschrieben.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
