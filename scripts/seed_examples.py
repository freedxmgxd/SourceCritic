#!/usr/bin/env python3
"""Load Metacritic-scraped example reviews into a running SourceCritic instance."""

from __future__ import annotations

import argparse
import json
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
SEED = ROOT / "data" / "seed_reviews.json"
SYNOPSES = ROOT / "data" / "game_synopses.json"

from adapters import (  # noqa: E402
    PSEUDO_OUTLET_SLUGS,
    UNSCORED_SLUGS,
    outlet_merge_key,
    publication_merge_name,
    sanitize_review_url,
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default="http://127.0.0.1:3000/reviews")
    parser.add_argument("--seed", type=Path, default=SEED)
    parser.add_argument("--batch-size", type=int, default=200)
    args = parser.parse_args()

    data = json.loads(args.seed.read_text())
    synopses: dict[str, str] = {}
    if SYNOPSES.exists():
        synopses = {
            key: value.strip()
            for key, value in json.loads(SYNOPSES.read_text()).items()
            if isinstance(value, str) and value.strip()
        }
    reviews = []
    skipped = 0
    for game_slug, game in data.items():
        cover = game.get("cover") or f"/covers/{game_slug}.jpg"
        release_year = game.get("release_year")
        platforms = game.get("platforms") or []
        steam_appid = game.get("steam_appid")
        steam_lists = game.get("steam_lists") or []
        synopsis = (game.get("synopsis") or synopses.get(game_slug) or "").strip()
        for review in game.get("reviews") or []:
            outlet_slug = outlet_merge_key(review)
            if outlet_slug in PSEUDO_OUTLET_SLUGS:
                skipped += 1
                continue
            unscored = outlet_slug in UNSCORED_SLUGS
            score = review.get("score")
            score_f = None
            if not unscored:
                if score is None:
                    skipped += 1
                    continue
                try:
                    score_f = float(score)
                except (TypeError, ValueError):
                    skipped += 1
                    continue
            outlet_name = publication_merge_name(
                (review.get("publication") or review.get("outlet") or outlet_slug).strip()
            )
            item = {
                "outlet_slug": outlet_slug,
                "outlet_name": outlet_name,
                "game_slug": game_slug,
                "game_title": game["title"],
                "cover_path": cover,
                "scale_max": 100.0,
                "raw_score_norm": 0.0 if score_f is None else score_f,
            }
            if score_f is not None:
                item["raw_score"] = score_f
            if release_year is not None:
                item["release_year"] = int(release_year)
            if platforms:
                item["platforms"] = list(platforms)
            review_plats = [
                p for p in (review.get("platforms") or []) if isinstance(p, str) and p.strip()
            ]
            if review_plats:
                item["review_platforms"] = review_plats
            if steam_appid:
                item["steam_appid"] = int(steam_appid)
            if steam_lists:
                item["steam_lists"] = list(steam_lists)
            if synopsis:
                item["synopsis"] = synopsis
            if review.get("excerpt"):
                item["excerpt"] = review["excerpt"]
            url = sanitize_review_url(review.get("url"))
            if url:
                item["review_url"] = url
            reviews.append(item)
    if skipped:
        print(f"skipped {skipped} unscored/pseudo reviews")

    total_upserted = 0
    last_games = []
    batch_size = max(1, args.batch_size)
    for start in range(0, len(reviews), batch_size):
        batch = reviews[start : start + batch_size]
        body = json.dumps({"reviews": batch}).encode()
        req = urllib.request.Request(
            args.url,
            data=body,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=180) as response:
            result = json.loads(response.read().decode())
        total_upserted += int(result.get("upserted") or 0)
        last_games = result.get("games") or last_games
        print(
            f"batch {start // batch_size + 1}: "
            f"upserted {result.get('upserted')} "
            f"({start + len(batch)}/{len(reviews)})"
        )

    print(f"upserted {total_upserted} reviews across {len(last_games)} games")
    for game in last_games:
        print(
            f"  {game['title']}: score={game.get('agg_score')} "
            f"mean={game.get('simple_mean')} n={game.get('review_count')} "
            f"cover={game.get('cover_path')}"
        )


if __name__ == "__main__":
    main()
