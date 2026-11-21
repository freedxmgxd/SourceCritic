#!/usr/bin/env python3
"""Load Metacritic-scraped example reviews into a running SourceCritic instance."""

from __future__ import annotations

import argparse
import json
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "data" / "seed_reviews.json"
SYNOPSES = ROOT / "data" / "game_synopses.json"


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
    for game_slug, game in data.items():
        cover = game.get("cover") or f"/covers/{game_slug}.jpg"
        release_year = game.get("release_year")
        platforms = game.get("platforms") or []
        steam_appid = game.get("steam_appid")
        steam_lists = game.get("steam_lists") or []
        synopsis = (game.get("synopsis") or synopses.get(game_slug) or "").strip()
        for review in game["reviews"]:
            item = {
                "outlet_slug": review["slug"] or review["publication"].lower(),
                "outlet_name": review["publication"],
                "game_slug": game_slug,
                "game_title": game["title"],
                "cover_path": cover,
                "raw_score": review["score"],
                "scale_max": 100.0,
                "raw_score_norm": review["score"],
            }
            if release_year is not None:
                item["release_year"] = int(release_year)
            if platforms:
                item["platforms"] = list(platforms)
            if steam_appid:
                item["steam_appid"] = int(steam_appid)
            if steam_lists:
                item["steam_lists"] = list(steam_lists)
            if synopsis:
                item["synopsis"] = synopsis
            if review.get("excerpt"):
                item["excerpt"] = review["excerpt"]
            url = (review.get("url") or "").strip()
            if url.startswith("http://") or url.startswith("https://"):
                item["review_url"] = url
            reviews.append(item)

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
