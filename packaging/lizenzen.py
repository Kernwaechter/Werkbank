#!/usr/bin/env python3
"""Schreibt assets/lizenzen.txt: alle Fremdbausteine der Werkbank mit Lizenz (aus `cargo metadata`).
Aufruf im Projektordner: python3 packaging/lizenzen.py"""
import json, subprocess, sys, collections

meta = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--locked"]))
eigene = {m["name"] for m in meta["packages"] if m["source"] is None}
zeilen, zaehler = [], collections.Counter()
for p in sorted(meta["packages"], key=lambda p: p["name"].lower()):
    if p["name"] in eigene:
        continue
    lizenz = p.get("license") or ("siehe " + p.get("license_file", "Quelltext") if p.get("license_file") else "UNBEKANNT")
    zeilen.append(f'{p["name"]} {p["version"]}\t{lizenz}')
    zaehler[lizenz] += 1
with open("assets/lizenzen.txt", "w", encoding="utf-8") as f:
    f.write("\n".join(zeilen) + "\n")
print(f"{len(zeilen)} Fremdbausteine", file=sys.stderr)
kritisch = [l for l in zaehler if any(k in l.upper() for k in ("GPL", "AGPL", "SSPL", "UNBEKANNT")) and "LGPL" not in l.upper().replace("AGPL", "")]
print("Lizenzen nach Häufigkeit:", file=sys.stderr)
for l, n in zaehler.most_common():
    print(f"  {n:4d}  {l}", file=sys.stderr)
if kritisch:
    print("ACHTUNG: genauer prüfen:", *kritisch, sep="\n  ", file=sys.stderr)
