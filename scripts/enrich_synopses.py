#!/usr/bin/env python3
"""Fetch game synopses from Metacritic into data/game_synopses.json (+ optional seed/DB)."""

from __future__ import annotations

import argparse
import json
import re
import sqlite3
import time
import urllib.request
from pathlib import Path

TRAILING_TAG = re.compile(r"\s*\[[^\[\]\n]+\]\s*$")


def clean_synopsis(text: str) -> str:
    cleaned = text.strip()
    while True:
        next_text = TRAILING_TAG.sub("", cleaned).rstrip()
        if next_text == cleaned:
            return cleaned
        cleaned = next_text

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "data" / "seed_reviews.json"
OUT = ROOT / "data" / "game_synopses.json"
DB = ROOT / "sourcecritic.db"
MC_API_KEY = "1MOZgmNFxvmljaQR1X9KAij9Mo4xAY3u"
MC_UA = {
    "User-Agent": "Mozilla/5.0",
    "Accept": "application/json",
    "Referer": "https://www.metacritic.com/",
}


def get_json(url: str) -> dict:
    req = urllib.request.Request(url, headers=MC_UA)
    with urllib.request.urlopen(req, timeout=40) as response:
        return json.loads(response.read().decode())


def mc_synopsis(slug: str) -> str | None:
    url = (
        "https://backend.metacritic.com/composer/metacritic/pages/games/"
        f"{slug}/web?apiKey={MC_API_KEY}"
    )
    data = get_json(url)
    item = ((data.get("components") or [{}])[0].get("data") or {}).get("item") or {}
    text = clean_synopsis(item.get("description") or "")
    return text or None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--merge-seed", action="store_true")
    parser.add_argument("--update-db", action="store_true")
    args = parser.parse_args()

    seed = json.loads(SEED.read_text()) if SEED.exists() else {}
    synopses: dict[str, str] = {}
    if OUT.exists():
        synopses = {
            k: v
            for k, v in json.loads(OUT.read_text()).items()
            if isinstance(v, str) and v.strip()
        }

    targets = []
    for slug, game in seed.items():
        if synopses.get(slug) or (game.get("synopsis") or "").strip():
            if (game.get("synopsis") or "").strip() and slug not in synopses:
                synopses[slug] = game["synopsis"].strip()
            continue
        mc_slug = game.get("mc_slug") or slug
        targets.append((slug, mc_slug))

    print(f"have {len(synopses)}; fetch {len(targets)}")
    for i, (slug, mc_slug) in enumerate(targets, 1):
        try:
            text = mc_synopsis(mc_slug)
            time.sleep(0.08)
            if text:
                synopses[slug] = text
                print(f"  [{i}/{len(targets)}] {slug}: ok ({len(text)} chars)")
            else:
                print(f"  [{i}/{len(targets)}] {slug}: empty")
        except Exception as error:
            print(f"  [{i}/{len(targets)}] {slug}: fail {error}")

    OUT.write_text(json.dumps(synopses, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {len(synopses)} synopses -> {OUT}")

    if args.merge_seed and seed:
        changed = 0
        for slug, text in synopses.items():
            if slug not in seed:
                continue
            if seed[slug].get("synopsis") != text:
                seed[slug]["synopsis"] = text
                changed += 1
        SEED.write_text(json.dumps(seed, indent=2, ensure_ascii=False) + "\n")
        print(f"merged into seed: {changed} games")

    if args.update_db and DB.exists():
        conn = sqlite3.connect(DB)
        cols = {row[1] for row in conn.execute("PRAGMA table_info(games)")}
        if "synopsis" not in cols:
            conn.execute("ALTER TABLE games ADD COLUMN synopsis TEXT")
        updated = 0
        for slug, text in synopses.items():
            cur = conn.execute(
                "UPDATE games SET synopsis = ? WHERE slug = ?",
                (text, slug),
            )
            updated += cur.rowcount
        conn.commit()
        have = conn.execute(
            'SELECT COUNT(*) FROM games WHERE synopsis IS NOT NULL AND synopsis != ""'
        ).fetchone()[0]
        total = conn.execute("SELECT COUNT(*) FROM games").fetchone()[0]
        conn.close()
        print(f"db updated rows={updated}; with synopsis={have}/{total}")


if __name__ == "__main__":
    main()
