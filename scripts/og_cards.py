#!/usr/bin/env python3
"""Write a 1200x630 JPG share card per game into the static export."""

from __future__ import annotations

import sqlite3
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "dist/og")
    db = Path(sys.argv[2] if len(sys.argv) > 2 else ROOT / "sourcecritic.db")
    out.mkdir(parents=True, exist_ok=True)
    con = sqlite3.connect(db)
    rows = con.execute(
        "SELECT slug, title, cover_path, agg_score FROM games"
    ).fetchall()
    font = ImageFont.load_default()
    for slug, title, cover, score in rows:
        card = Image.new("RGB", (1200, 630), (250, 244, 245))
        draw = ImageDraw.Draw(card)
        cover_path = None
        if cover:
            candidate = ROOT / cover.lstrip("/")
            if not candidate.is_file():
                candidate = ROOT / "static" / cover.lstrip("/")
            if candidate.is_file():
                cover_path = candidate
        if cover_path:
            try:
                art = Image.open(cover_path).convert("RGB")
                art.thumbnail((420, 560))
                card.paste(art, (48, (630 - art.height) // 2))
            except OSError:
                cover_path = None
        score_text = "—" if score is None else str(int(round(score)))
        draw.text((520, 220), title[:80], fill=(26, 18, 24), font=font)
        draw.text((520, 280), f"SourceCritic  {score_text}", fill=(139, 30, 74), font=font)
        card.save(out / f"{slug}.jpg", "JPEG", quality=80, optimize=True)
    print(f"og cards {len(rows)} -> {out}")


if __name__ == "__main__":
    main()
