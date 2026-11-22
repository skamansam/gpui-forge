#!/usr/bin/env python3
"""Regenerate crates/gpui-forge-runtime/src/lucide-categories.json.

Iconify's lucide.json ships no `categories`, but the lucide-icons/lucide
repo carries them as per-icon metadata (`icons/<name>.json` ->
`categories: [slugs]`) plus display titles in `categories/<slug>.json`.
Deprecated/renamed icons reference `$extends:<target>` in place of a
real category. We follow those refs so aliases inherit their target's
categories.

Usage: python3 scripts/gen-lucide-categories.py
Fetches the repo tarball once (~3.5MB), writes the vendored JSON in the
iconify `categories` shape: {"categories": {"<Title>": [<names>]}}.
"""
import glob
import json
import os
import re
import subprocess
import sys
import tempfile

REPO = "https://codeload.github.com/lucide-icons/lucide/tar.gz/refs/heads/main"
OUT = os.path.join(
    os.path.dirname(__file__),
    "..", "crates", "gpui-forge-runtime", "src", "lucide-categories.json",
)


def natkey(s):
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r"(\d+)", s)]


def main():
    with tempfile.TemporaryDirectory() as td:
        tar = os.path.join(td, "lucide.tar.gz")
        subprocess.run(["curl", "-sL", "--max-time", "180", "-o", tar, REPO], check=True)
        subprocess.run(["tar", "xzf", tar, "-C", td], check=True)
        root = glob.glob(os.path.join(td, "lucide-*"))[0]

        titles = {}
        for f in glob.glob(os.path.join(root, "categories", "*.json")):
            slug = os.path.basename(f)[:-5]
            titles[slug] = json.load(open(f))["title"]

        raw = {}
        for f in glob.glob(os.path.join(root, "icons", "*.json")):
            name = os.path.basename(f)[:-5]
            raw[name] = json.load(open(f)).get("categories", [])

        def resolve(name, depth=0):
            if depth > 3:
                return []
            out = []
            for c in raw.get(name, []):
                if c.startswith("$extends:"):
                    out += resolve(c[len("$extends:"):], depth + 1)
                elif c in titles:
                    out.append(c)
            return out

        cats = {}
        for name in raw:
            for slug in set(resolve(name)):
                cats.setdefault(titles[slug], []).append(name)

        out = {k: sorted(v, key=natkey) for k, v in sorted(cats.items())}
        with open(OUT, "w") as fh:
            json.dump({"categories": out}, fh, separators=(",", ":"))
        print(f"{len(out)} categories, "
              f"{len(set().union(*map(set, out.values())))} icons -> {OUT}")


if __name__ == "__main__":
    sys.exit(main())
