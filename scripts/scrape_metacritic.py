#!/usr/bin/env python3
"""Scrape Metacritic must-play / high-score games into data/seed_reviews.json."""

from __future__ import annotations

import argparse
import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "data" / "seed_reviews.json"
COVERS = ROOT / "static" / "covers"

API_KEY = "1MOZgmNFxvmljaQR1X9KAij9Mo4xAY3u"
UA = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
    "Accept": "application/json",
    "Referer": "https://www.metacritic.com/",
}

SKIP_SLUGS = {
    "all",
    "ps5",
    "ps4",
    "xbox-series-x",
    "xbox-one",
    "pc",
    "switch",
    "nintendo-switch",
    "ios",
    "android",
}


def get_json(url: str) -> dict:
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=45) as response:
        return json.loads(response.read().decode())


def get_bytes(url: str) -> bytes:
    req = urllib.request.Request(
        url,
        headers={
            "User-Agent": UA["User-Agent"],
            "Referer": "https://www.metacritic.com/",
        },
    )
    with urllib.request.urlopen(req, timeout=45) as response:
        return response.read()


def slugify(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "outlet"


def browse_slugs(pages: int, release_type: str | None) -> list[str]:
    ordered: list[str] = []
    seen: set[str] = set()
    for page in range(1, pages + 1):
        params = {"releaseYearMin": "1958", "page": str(page)}
        if release_type:
            params["releaseType"] = release_type
        url = "https://www.metacritic.com/browse/game/?" + urllib.parse.urlencode(params)
        html = get_bytes(url).decode("utf-8", "ignore")
        for slug in re.findall(r"/game/([a-z0-9-]+)/", html):
            if slug in SKIP_SLUGS or slug in seen:
                continue
            seen.add(slug)
            ordered.append(slug)
        print(f"  browse page {page}: {len(ordered)} unique so far")
        time.sleep(0.15)
    return ordered


def game_details(mc_slug: str) -> dict | None:
    url = (
        "https://backend.metacritic.com/composer/metacritic/pages/games/"
        f"{mc_slug}/web?apiKey={API_KEY}"
    )
    try:
        data = get_json(url)
    except urllib.error.HTTPError as error:
        print(f"  details fail {mc_slug}: {error.code}")
        return None
    components = data.get("components") or []
    if not components:
        return None
    item = (components[0].get("data") or {}).get("item") or {}
    if not item.get("slug"):
        return None
    summary = item.get("criticScoreSummary") or {}
    images = item.get("images") or []
    card = next((img for img in images if img.get("typeName") == "cardImage"), None)
    if card is None and images:
        card = images[0]
    cover_url = None
    if card and card.get("bucketPath"):
        cover_url = f"https://www.metacritic.com/a/img/catalog{card['bucketPath']}"
    platforms = item.get("platforms") or []
    platform_slug = None
    if platforms:
        platform_slug = platforms[0].get("slug") or None
    return {
        "title": item.get("title") or mc_slug,
        "slug": item.get("slug") or mc_slug,
        "metascore": summary.get("score"),
        "review_count": summary.get("reviewCount") or 0,
        "platform": item.get("platform"),
        "platform_slug": platform_slug,
        "cover_url": cover_url,
        "must_play": bool(item.get("mustPlay")),
    }


def fetch_reviews(mc_slug: str, platform: str | None = None) -> list[dict]:
    items: list[dict] = []
    offset = 0
    while True:
        params = {
            "apiKey": API_KEY,
            "offset": str(offset),
            "filter": "all",
            "sortOptions": "date",
            "componentName": "critic-reviews",
        }
        if platform:
            params["platform"] = platform
        url = (
            "https://backend.metacritic.com/reviews/metacritic/critic/games/"
            f"{mc_slug}/web?" + urllib.parse.urlencode(params)
        )
        data = get_json(url)
        batch = ((data.get("data") or {}).get("items")) or []
        total = (data.get("data") or {}).get("totalResults") or 0
        items.extend(batch)
        offset += len(batch)
        if not batch or offset >= total:
            break
        time.sleep(0.08)
    by_pub: dict[str, dict] = {}
    for it in items:
        name = (it.get("publicationName") or "").strip()
        score = it.get("score")
        if not name or score is None:
            continue
        quote = (it.get("quote") or "").strip()
        link = (it.get("url") or "").strip()
        by_pub[name] = {
            "publication": name,
            "slug": it.get("publicationSlug") or slugify(name),
            "score": float(score),
        }
        if quote:
            by_pub[name]["excerpt"] = quote
        if link.startswith("http://") or link.startswith("https://"):
            by_pub[name]["url"] = link
    return list(by_pub.values())


def download_cover(game_slug: str, cover_url: str | None) -> str | None:
    if not cover_url:
        return None
    COVERS.mkdir(parents=True, exist_ok=True)
    dest = COVERS / f"{game_slug}.jpg"
    try:
        dest.write_bytes(get_bytes(cover_url))
    except Exception as error:
        print(f"  cover fail {game_slug}: {error}")
        return None
    return f"/covers/{game_slug}.jpg"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pages", type=int, default=6, help="browse pages per list")
    parser.add_argument("--limit", type=int, default=80, help="max new games to scrape")
    parser.add_argument("--min-reviews", type=int, default=8)
    parser.add_argument("--seed", type=Path, default=SEED)
    parser.add_argument("--keep-existing", action="store_true", default=True)
    args = parser.parse_args()

    existing = {}
    if args.seed.exists():
        existing = json.loads(args.seed.read_text())
    print(f"existing seed games: {len(existing)}")

    print("collecting browse slugs…")
    slugs: list[str] = []
    seen: set[str] = set(existing)
    for release_type in ("must-play", None):
        for slug in browse_slugs(args.pages, release_type):
            if slug in seen:
                continue
            seen.add(slug)
            slugs.append(slug)
    print(f"candidate slugs: {len(slugs)}")

    seed = dict(existing) if args.keep_existing else {}
    added = 0
    for mc_slug in slugs:
        if added >= args.limit:
            break
        if mc_slug in seed:
            continue
        print(f"fetch {mc_slug}")
        details = game_details(mc_slug)
        time.sleep(0.12)
        if not details:
            continue
        score = details["metascore"]
        if score is None or details["review_count"] < args.min_reviews:
            print(f"  skip (score={score}, n={details['review_count']})")
            continue

        reviews = []
        try:
            reviews = fetch_reviews(mc_slug, details.get("platform_slug"))
        except Exception as error:
            print(f"  reviews fail with platform: {error}")
        if len(reviews) < args.min_reviews:
            try:
                reviews = fetch_reviews(mc_slug, None)
            except Exception as error:
                print(f"  reviews fail: {error}")
                continue
        if len(reviews) < args.min_reviews:
            print(f"  skip (only {len(reviews)} critic reviews)")
            continue

        game_slug = details["slug"]
        cover = download_cover(game_slug, details.get("cover_url"))
        time.sleep(0.05)
        seed[game_slug] = {
            "title": details["title"],
            "cover": cover or f"/covers/{game_slug}.jpg",
            "metascore": score,
            "mc_slug": mc_slug,
            "reviews": reviews,
        }
        added += 1
        print(f"  ok title={details['title']!r} score={score} reviews={len(reviews)} cover={cover}")

    args.seed.write_text(json.dumps(seed, indent=2) + "\n")
    total_reviews = sum(len(g["reviews"]) for g in seed.values())
    print(f"wrote {args.seed} — {len(seed)} games, {total_reviews} reviews (+{added} new)")


if __name__ == "__main__":
    main()
