#!/usr/bin/env python3
"""
Scrape 2026 games from Metacritic + OpenCritic into data/seed_reviews.json.

Selection (union):
  - top 50 / bottom 50 by critic score
  - top 50 / bottom 50 by user score (MC user score, OC % recommended)

Critic reviews come from both sites. Aggregate user scores / recommend %
are stored as game-level metadata only — never as pseudo-outlet reviews.
"""

from __future__ import annotations

import argparse
import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from html import unescape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "data" / "seed_reviews.json"
COVERS = ROOT / "static" / "covers"

YEAR = 2026
MC_API_KEY = "1MOZgmNFxvmljaQR1X9KAij9Mo4xAY3u"

MC_UA = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
    "Accept": "application/json",
    "Referer": "https://www.metacritic.com/",
}
OC_UA = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
    "Accept": "text/html,application/xhtml+xml",
    "Referer": "https://opencritic.com/",
}

MC_SKIP = {
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
    "score",
}


def http_bytes(url: str, headers: dict, timeout: int = 45) -> bytes:
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return response.read()


def http_json(url: str, headers: dict) -> dict | list:
    return json.loads(http_bytes(url, headers).decode())


def http_text(url: str, headers: dict) -> str:
    return http_bytes(url, headers).decode("utf-8", "ignore")


def slugify(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "item"


def top_bottom(items: list, key, n: int = 50) -> list:
    scored = [x for x in items if key(x) is not None]
    scored.sort(key=key, reverse=True)
    if len(scored) <= n * 2:
        return scored
    # preserve order uniqueness
    chosen = []
    seen = set()
    for item in scored[:n] + list(reversed(scored[-n:])):
        ident = id(item)
        if ident in seen:
            continue
        seen.add(ident)
        chosen.append(item)
    return chosen


# --- OpenCritic -------------------------------------------------------------


def oc_parse_state(html: str) -> dict:
    match = re.search(
        r'<script[^>]*type="application/json"[^>]*>(.*?)</script>', html, re.S
    )
    if not match:
        return {}
    raw = (
        unescape(match.group(1))
        .replace("&q;", '"')
        .replace("&s;", "'")
        .replace("&a;", "&")
    )
    return json.loads(raw)


def oc_list_year(year: int, max_pages: int = 80) -> list[dict]:
    games: list[dict] = []
    seen: set[int] = set()
    max_page = 1
    page = 1
    while page <= max_page and page <= max_pages:
        url = f"https://opencritic.com/browse/all/{year}"
        if page > 1:
            url += f"?page={page}"
        html = http_text(url, OC_UA)
        state = oc_parse_state(html)
        batch = []
        for key, value in state.items():
            if "api/game" in key and isinstance(value, list):
                batch = value
                break
        pages = [int(x) for x in re.findall(r"[?&]page=(\d+)", html)]
        if pages:
            max_page = max(max_page, max(pages))
        for game in batch:
            gid = game.get("id")
            if gid is None or gid in seen:
                continue
            seen.add(gid)
            games.append(game)
        print(f"  OC {year} page {page}/{max_page}: +{len(batch)} (total {len(games)})")
        if not batch:
            break
        page += 1
        time.sleep(0.12)
    return games


def oc_select(games: list[dict], n: int = 50) -> list[dict]:
    critics = [
        g
        for g in games
        if (g.get("topCriticScore") or -1) > 0 and (g.get("numReviews") or 0) >= 5
    ]
    users = [
        g
        for g in critics
        if (g.get("percentRecommended") or -1) >= 0
    ]
    picked = {}
    for game in top_bottom(critics, lambda g: g["topCriticScore"], n) + top_bottom(
        users, lambda g: g["percentRecommended"], n
    ):
        picked[game["id"]] = game
    print(
        f"  OC select: {len(picked)} games "
        f"(from {len(critics)} scored / {len(games)} listed)"
    )
    return list(picked.values())


def oc_reviews(game_id: int, slug: str, min_needed: int = 1) -> list[dict]:
    out: list[dict] = []
    seen: set[str] = set()
    page = 1
    while page <= 30:
        url = f"https://opencritic.com/game/{game_id}/{slug}/reviews"
        if page > 1:
            url += f"?page={page}"
        try:
            html = http_text(url, OC_UA)
        except Exception as error:
            print(f"    OC reviews fail p{page}: {error}")
            break
        state = oc_parse_state(html)
        batch = []
        for key, value in state.items():
            if isinstance(value, list) and value and isinstance(value[0], dict):
                if "score" in value[0] and "Outlet" in value[0]:
                    batch = value
                    break
        if not batch:
            break
        for review in batch:
            outlet = (review.get("Outlet") or {}).get("name") or ""
            # OpenCritic `score` is already 0–100. `npScore` is recommend (0/100), not the grade.
            score = review.get("score")
            if score is None:
                continue
            if not outlet or score < 0:
                continue
            key = outlet.lower()
            if key in seen:
                continue
            seen.add(key)
            item = {
                "publication": outlet,
                "slug": slugify(outlet),
                "score": float(score),
                "source": "opencritic",
            }
            snippet = (review.get("snippet") or "").strip()
            if snippet:
                item["excerpt"] = snippet
            link = (review.get("externalUrl") or "").strip()
            if link.startswith("http://") or link.startswith("https://"):
                item["url"] = link
            out.append(item)
        if len(batch) < 20:
            break
        page += 1
        time.sleep(0.08)
    if len(out) < min_needed:
        return out
    return out


def oc_cover(game: dict, game_slug: str) -> str | None:
    images = game.get("images") or {}
    box = images.get("box") or {}
    url = box.get("og") or box.get("sm") or images.get("banner", {}).get("og")
    if not url:
        return None
    if url.startswith("//"):
        url = "https:" + url
    COVERS.mkdir(parents=True, exist_ok=True)
    dest = COVERS / f"{game_slug}.jpg"
    if dest.exists() and dest.stat().st_size > 1000:
        return f"/covers/{game_slug}.jpg"
    try:
        dest.write_bytes(
            http_bytes(
                url,
                {
                    "User-Agent": OC_UA["User-Agent"],
                    "Referer": "https://opencritic.com/",
                },
            )
        )
        return f"/covers/{game_slug}.jpg"
    except Exception as error:
        print(f"    OC cover fail: {error}")
        return None


# --- Metacritic -------------------------------------------------------------


def mc_browse_year(year: int, sort_path: str = "") -> list[str]:
    """Return ordered unique slugs for a year browse listing."""
    ordered: list[str] = []
    seen: set[str] = set()
    if sort_path == "userscore":
        base = (
            "https://www.metacritic.com/browse/game/all/all/all-time/userscore/"
            f"?releaseYearMin={year}&releaseYearMax={year}&page={{}}"
        )
    else:
        base = (
            "https://www.metacritic.com/browse/game/"
            f"?releaseYearMin={year}&releaseYearMax={year}&page={{}}"
        )
    max_page = 1
    page = 1
    while page <= max_page and page <= 30:
        html = http_text(base.format(page), {**MC_UA, "Accept": "text/html"})
        pages = [int(x) for x in re.findall(r"[?&]page=(\d+)", html)]
        if pages:
            max_page = max(max_page, max(pages))
        found = 0
        for slug in re.findall(r"/game/([a-z0-9-]+)/", html):
            if slug in MC_SKIP or slug in seen:
                continue
            seen.add(slug)
            ordered.append(slug)
            found += 1
        print(f"  MC {sort_path or 'metascore'} page {page}: +{found} (total {len(ordered)})")
        if found == 0:
            break
        page += 1
        time.sleep(0.12)
    return ordered


def mc_details(slug: str) -> dict | None:
    url = (
        "https://backend.metacritic.com/composer/metacritic/pages/games/"
        f"{slug}/web?apiKey={MC_API_KEY}"
    )
    try:
        data = http_json(url, MC_UA)
    except urllib.error.HTTPError as error:
        print(f"    MC details fail {slug}: {error.code}")
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
    platform_slug = platforms[0].get("slug") if platforms else None
    return {
        "title": item.get("title") or slug,
        "slug": item.get("slug") or slug,
        "metascore": summary.get("score"),
        "review_count": summary.get("reviewCount") or 0,
        "platform_slug": platform_slug,
        "cover_url": cover_url,
        "premiere_year": item.get("premiereYear"),
    }


def mc_user_score(slug: str) -> tuple[float | None, int]:
    query = urllib.parse.urlencode(
        {
            "apiKey": MC_API_KEY,
            "componentName": "user-score-summary",
            "componentDisplayName": "User Score Summary",
            "componentType": "MetaScoreSummary",
        }
    )
    url = (
        "https://backend.metacritic.com/reviews/metacritic/user/games/"
        f"{slug}/stats/web?{query}"
    )
    try:
        data = http_json(url, MC_UA)
    except Exception:
        return None, 0
    item = (data.get("data") or {}).get("item") or {}
    score = item.get("score")
    count = item.get("reviewCount") or item.get("count") or 0
    if score is None or score == "tbd":
        return None, int(count or 0)
    try:
        return float(score), int(count or 0)
    except (TypeError, ValueError):
        return None, int(count or 0)


def mc_critic_reviews(slug: str, platform: str | None = None) -> list[dict]:
    items: list[dict] = []
    offset = 0
    while True:
        params = {
            "apiKey": MC_API_KEY,
            "offset": str(offset),
            "filter": "all",
            "sortOptions": "date",
            "componentName": "critic-reviews",
        }
        if platform:
            params["platform"] = platform
        url = (
            "https://backend.metacritic.com/reviews/metacritic/critic/games/"
            f"{slug}/web?" + urllib.parse.urlencode(params)
        )
        data = http_json(url, MC_UA)
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
        row = {
            "publication": name,
            "slug": it.get("publicationSlug") or slugify(name),
            "score": float(score),
            "source": "metacritic",
        }
        quote = (it.get("quote") or "").strip()
        if quote:
            row["excerpt"] = quote
        link = (it.get("url") or "").strip()
        if link.startswith("http://") or link.startswith("https://"):
            row["url"] = link
        by_pub[name] = row
    return list(by_pub.values())


def mc_download_cover(game_slug: str, cover_url: str | None) -> str | None:
    if not cover_url:
        return None
    COVERS.mkdir(parents=True, exist_ok=True)
    dest = COVERS / f"{game_slug}.jpg"
    if dest.exists() and dest.stat().st_size > 1000:
        return f"/covers/{game_slug}.jpg"
    try:
        dest.write_bytes(
            http_bytes(
                cover_url,
                {
                    "User-Agent": MC_UA["User-Agent"],
                    "Referer": "https://www.metacritic.com/",
                },
            )
        )
        return f"/covers/{game_slug}.jpg"
    except Exception as error:
        print(f"    MC cover fail: {error}")
        return None


def merge_reviews(existing: list[dict], new_reviews: list[dict]) -> list[dict]:
    by_key: dict[str, dict] = {}
    for review in existing + new_reviews:
        key = (review.get("slug") or slugify(review["publication"])).lower()
        # Prefer entry that has excerpt/url; keep highest score source diversity by outlet
        prev = by_key.get(key)
        if prev is None:
            by_key[key] = review
            continue
        # same outlet: keep the one with excerpt if other lacks it
        if not prev.get("excerpt") and review.get("excerpt"):
            by_key[key] = review
        elif prev.get("excerpt") and not review.get("excerpt"):
            continue
        else:
            # average-ish: keep metacritic if both, else first
            if review.get("source") == "metacritic" and prev.get("source") != "metacritic":
                by_key[key] = review
    return list(by_key.values())


def ensure_game(seed: dict, game_slug: str, title: str, cover: str | None) -> dict:
    entry = seed.get(game_slug)
    if entry is None:
        entry = {
            "title": title,
            "cover": cover or f"/covers/{game_slug}.jpg",
            "reviews": [],
        }
        seed[game_slug] = entry
    else:
        if title and (
            not entry.get("title")
            or len(title) > len(entry.get("title") or "")
        ):
            entry["title"] = title
        if cover:
            entry["cover"] = cover
    return entry


def main() -> None:
    raise SystemExit("Metacritic and OpenCritic scraping is disabled (compliance).")
    parser = argparse.ArgumentParser()
    parser.add_argument("--year", type=int, default=YEAR)
    parser.add_argument("--top", type=int, default=50)
    parser.add_argument("--seed", type=Path, default=SEED)
    parser.add_argument("--keep-existing", action="store_true", default=True)
    args = parser.parse_args()

    seed: dict = {}
    if args.keep_existing and args.seed.exists():
        seed = json.loads(args.seed.read_text())
    print(f"existing seed games: {len(seed)}")

    # ----- OpenCritic selection + scrape -----
    print(f"OpenCritic {args.year}…")
    oc_games = oc_list_year(args.year)
    oc_picked = oc_select(oc_games, args.top)
    for game in oc_picked:
        title = game.get("name") or f"game-{game['id']}"
        game_slug = slugify(title)
        # avoid colliding with unrelated older titles when OC drops year suffix
        if game.get("firstReleaseDate", "").startswith(str(args.year)):
            # keep slug; if seed already has different year title under slug, namespace
            existing = seed.get(game_slug)
            if existing and str(args.year) not in (existing.get("title") or ""):
                if existing.get("mc_slug") and "2026" not in (existing.get("mc_slug") or ""):
                    game_slug = f"{game_slug}-{args.year}"
        print(f"  OC {game['id']} {title}")
        path_slug = slugify(title)
        reviews = oc_reviews(game["id"], path_slug)
        cover = oc_cover(game, game_slug)
        entry = ensure_game(seed, game_slug, title, cover)
        entry["opencritic_id"] = game["id"]
        entry["opencritic_score"] = game.get("topCriticScore")
        entry["opencritic_recommend"] = game.get("percentRecommended")
        entry["reviews"] = merge_reviews(entry.get("reviews") or [], reviews)
        print(f"    reviews now {len(entry['reviews'])}")
        time.sleep(0.1)

    # ----- Metacritic selection + scrape -----
    print(f"Metacritic {args.year}…")
    meta_slugs = mc_browse_year(args.year, "")
    user_slugs = mc_browse_year(args.year, "userscore")
    candidates = list(dict.fromkeys(meta_slugs + user_slugs))
    print(f"  MC candidates: {len(candidates)}")

    detailed: list[dict] = []
    for slug in candidates:
        details = mc_details(slug)
        time.sleep(0.1)
        if not details:
            continue
        if details.get("premiere_year") not in (None, args.year):
            # still allow if browse filtered by year
            pass
        user_score, user_count = mc_user_score(slug)
        time.sleep(0.08)
        details["user_score"] = user_score  # 0-10
        details["user_count"] = user_count
        details["mc_slug"] = slug
        detailed.append(details)
        print(
            f"    {slug}: meta={details.get('metascore')} "
            f"user={user_score} n_crit={details.get('review_count')}"
        )

    with_meta = [d for d in detailed if d.get("metascore") is not None]
    with_user = [d for d in detailed if d.get("user_score") is not None]
    picked_mc = {}
    for row in top_bottom(with_meta, lambda d: d["metascore"], args.top) + top_bottom(
        with_user, lambda d: d["user_score"], args.top
    ):
        picked_mc[row["slug"]] = row
    print(f"  MC select: {len(picked_mc)} games")

    for details in picked_mc.values():
        slug = details["slug"]
        print(f"  MC scrape {slug}")
        reviews = mc_critic_reviews(slug, details.get("platform_slug"))
        if len(reviews) < 5:
            reviews = mc_critic_reviews(slug, None)
        cover = mc_download_cover(slug, details.get("cover_url"))
        entry = ensure_game(seed, slug, details["title"], cover)
        entry["metascore"] = details.get("metascore")
        entry["mc_slug"] = details.get("mc_slug") or slug
        entry["mc_user_score"] = details.get("user_score")
        entry["reviews"] = merge_reviews(entry.get("reviews") or [], reviews)
        print(f"    reviews now {len(entry['reviews'])}")

    args.seed.write_text(json.dumps(seed, indent=2) + "\n")
    total_reviews = sum(len(g.get("reviews") or []) for g in seed.values())
    print(f"wrote {args.seed} — {len(seed)} games, {total_reviews} reviews")


if __name__ == "__main__":
    main()
