#!/usr/bin/env python3
"""Fill release_year + platforms on data/seed_reviews.json from Metacritic/OpenCritic."""

from __future__ import annotations

import json
import re
import time
import urllib.error
import urllib.request
from html import unescape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "data" / "seed_reviews.json"
MC_API_KEY = "1MOZgmNFxvmljaQR1X9KAij9Mo4xAY3u"

MC_UA = {
    "User-Agent": "Mozilla/5.0",
    "Accept": "application/json",
    "Referer": "https://www.metacritic.com/",
}
OC_UA = {
    "User-Agent": "Mozilla/5.0",
    "Accept": "text/html",
    "Referer": "https://opencritic.com/",
}

PLATFORM_ALIAS = {
    "playstation 5": "ps5",
    "ps5": "ps5",
    "playstation 4": "ps4",
    "ps4": "ps4",
    "xbox series x/s": "xbox-series",
    "xbox series x": "xbox-series",
    "xbox series s": "xbox-series",
    "xbxs": "xbox-series",
    "xbox one": "xbox-one",
    "nintendo switch 2": "switch-2",
    "switch 2": "switch-2",
    "nintendo switch": "switch",
    "switch": "switch",
    "pc": "pc",
    "windows pc": "pc",
    "windows": "pc",
    "ios": "ios",
    "iphone/ipad": "ios",
    "android": "android",
}


def get_json(url: str) -> dict:
    req = urllib.request.Request(url, headers=MC_UA)
    with urllib.request.urlopen(req, timeout=40) as response:
        return json.loads(response.read().decode())


def get_text(url: str) -> str:
    req = urllib.request.Request(url, headers=OC_UA)
    with urllib.request.urlopen(req, timeout=40) as response:
        return response.read().decode("utf-8", "ignore")


def slugify(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "game"


def canon_platforms(names: list[str]) -> list[str]:
    out: list[str] = []
    seen: set[str] = set()
    for name in names:
        key = PLATFORM_ALIAS.get(name.strip().lower())
        if key is None:
            key = re.sub(r"[^a-z0-9]+", "-", name.strip().lower()).strip("-")
        if not key or key in seen:
            continue
        seen.add(key)
        out.append(key)
    return out


def year_from_title(title: str) -> int | None:
    match = re.search(r"\((19|20)\d{2}\)\s*$", title)
    if match:
        return int(match.group(0)[1:5])
    return None


def from_metacritic(slug: str) -> tuple[int | None, list[str], str | None]:
    url = (
        "https://backend.metacritic.com/composer/metacritic/pages/games/"
        f"{slug}/web?apiKey={MC_API_KEY}"
    )
    data = get_json(url)
    item = ((data.get("components") or [{}])[0].get("data") or {}).get("item") or {}
    year = item.get("premiereYear")
    platforms = []
    for platform in item.get("platforms") or []:
        name = platform.get("name") or platform.get("shortName")
        if name:
            platforms.append(name)
    if not platforms and item.get("platform"):
        platforms.append(item["platform"])
    synopsis = (item.get("description") or "").strip() or None
    return (int(year) if year else None, canon_platforms(platforms), synopsis)


def from_opencritic(game_id: int, title: str = "") -> tuple[int | None, list[str], str | None]:
    path = slugify(title) if title else str(game_id)
    html = get_text(f"https://opencritic.com/game/{game_id}/{path}")
    match = re.search(
        r'<script[^>]*type="application/json"[^>]*>(.*?)</script>', html, re.S
    )
    if not match:
        return None, [], None
    raw = unescape(match.group(1).replace("&q;", '"').replace("&s;", "'"))
    state = json.loads(raw)
    game = state.get(f"game/{game_id}") or {}
    year = None
    date = game.get("firstReleaseDate") or ""
    if isinstance(date, str) and len(date) >= 4 and date[:4].isdigit():
        year = int(date[:4])
    platforms = []
    for platform in game.get("Platforms") or game.get("platforms") or []:
        name = platform.get("name") or platform.get("shortName")
        if name:
            platforms.append(name)
    synopsis = (
        (game.get("description") or game.get("shortDescription") or "").strip() or None
    )
    return year, canon_platforms(platforms), synopsis


def main() -> None:
    seed = json.loads(SEED.read_text())
    updated = 0
    for slug, game in seed.items():
        year = game.get("release_year")
        platforms = list(game.get("platforms") or [])
        synopsis = (game.get("synopsis") or "").strip() or None
        changed = False
        need_meta = year is None or not platforms or not synopsis

        if year is None:
            year = year_from_title(game.get("title") or "")

        mc_slug = game.get("mc_slug") or (slug if need_meta else None)
        if need_meta and mc_slug:
            try:
                mc_year, mc_platforms, mc_synopsis = from_metacritic(mc_slug)
                time.sleep(0.08)
                if year is None and mc_year:
                    year = mc_year
                if not platforms and mc_platforms:
                    platforms = mc_platforms
                if not synopsis and mc_synopsis:
                    synopsis = mc_synopsis
                print(
                    f"  MC {slug}: year={year} platforms={platforms} "
                    f"synopsis={'yes' if synopsis else 'no'}"
                )
            except Exception as error:
                print(f"  MC fail {slug}: {error}")

        if (year is None or not platforms or not synopsis) and game.get("opencritic_id"):
            try:
                oc_year, oc_platforms, oc_synopsis = from_opencritic(
                    int(game["opencritic_id"]), game.get("title") or ""
                )
                time.sleep(0.1)
                if year is None and oc_year:
                    year = oc_year
                if not platforms and oc_platforms:
                    platforms = oc_platforms
                if not synopsis and oc_synopsis:
                    synopsis = oc_synopsis
                print(
                    f"  OC {slug}: year={year} platforms={platforms} "
                    f"synopsis={'yes' if synopsis else 'no'}"
                )
            except Exception as error:
                print(f"  OC fail {slug}: {error}")

        if year is not None and game.get("release_year") != year:
            game["release_year"] = year
            changed = True
        if platforms and game.get("platforms") != platforms:
            game["platforms"] = platforms
            changed = True
        if synopsis and game.get("synopsis") != synopsis:
            game["synopsis"] = synopsis
            changed = True
        if changed:
            updated += 1

    SEED.write_text(json.dumps(seed, indent=2) + "\n")
    with_year = sum(1 for g in seed.values() if g.get("release_year"))
    with_plat = sum(1 for g in seed.values() if g.get("platforms"))
    with_syn = sum(1 for g in seed.values() if g.get("synopsis"))
    print(
        f"updated {updated} games; "
        f"{with_year}/{len(seed)} have year, {with_plat}/{len(seed)} have platforms, "
        f"{with_syn}/{len(seed)} have synopsis"
    )


if __name__ == "__main__":
    main()
