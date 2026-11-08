#!/usr/bin/env python3
"""Schreibt assets/lizenztexte.txt: Lizenz- und Copyright-Texte aller Fremdbausteine der Linux-Fassung
(MIT, Apache und BSD verlangen den Text bei der Weitergabe). Aufruf im Projektordner:
python3 packaging/lizenztexte.py"""
import json, subprocess, sys, os, re

# Crates aus Workspaces (iced, typst …) haben keine eigene Lizenzdatei: Standardtext der Lizenz einsetzen
WURZEL = subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip()
TEXTE = os.path.join(WURZEL, "share", "doc", "rust", "licenses")


def standardtext(p):
    ausdruck = p.get("license") or ""
    for name in ("MIT", "Apache-2.0"):
        if name in ausdruck:
            with open(os.path.join(TEXTE, name + ".txt"), encoding="utf-8") as f:
                t = f.read().strip()
            autoren = ", ".join(a.split("<")[0].strip() for a in p.get("authors") or []) or "die Autoren des Crates"
            t = t.replace("Copyright (c) <year> <copyright holders>", f"Copyright (c) {autoren}")
            return f"--- {name} (Standardtext; das Crate bringt keine eigene Lizenzdatei mit) ---\n{t}\n"
    return None

meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", "x86_64-unknown-linux-gnu"]))
muster = re.compile(r"^(LICEN[CS]E|COPYING|COPYRIGHT|NOTICE|UNLICENSE)", re.I)
ohne = []
teile = []
for p in sorted(meta["packages"], key=lambda p: (p["name"].lower(), p["version"])):
    if p["source"] is None:
        continue
    ordner = os.path.dirname(p["manifest_path"])
    dateien = sorted(f for f in os.listdir(ordner) if muster.match(f) and os.path.isfile(os.path.join(ordner, f)))
    ersatz = None
    if not dateien:
        ersatz = standardtext(p)
        if not ersatz:
            ohne.append(f'{p["name"]} {p["version"]} ({p.get("license")})')
    teile.append(f'{"=" * 78}\n{p["name"]} {p["version"]} – {p.get("license") or "?"}\n{p.get("repository") or ""}\n')
    if ersatz:
        teile.append(ersatz)
    for d in dateien:
        with open(os.path.join(ordner, d), encoding="utf-8", errors="replace") as f:
            teile.append(f'--- {d} ---\n{f.read().strip()}\n')
with open("assets/lizenztexte.txt", "w", encoding="utf-8") as f:
    f.write("Lizenztexte der Fremdbausteine der Werkbank (Linux-Fassung)\n\n" + "\n".join(teile))
print(f"{len(teile)} Abschnitte; Crates ohne Lizenzdatei ({len(ohne)}):", *ohne, sep="\n  ", file=sys.stderr)
