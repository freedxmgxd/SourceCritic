#!/usr/bin/env python3
"""
1) Re-fetch OpenCritic reviews using true 0–100 `score` (not recommend `npScore`).
2) Pull Metacritic critic reviews when missing.
3) Merge remasters / Switch 2 editions into the base game (remakes stay separate).
"""

from __future__ import annotations

import json
import re
import time
import urllib.parse
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

# Legacy pseudo-outlets (stripped from seed; never treat as critics if reintroduced).
USER_SLUGS = {"metacritic-users", "opencritic-recommend"}

# Explicit remaster / enhanced-edition → base seed slug (when known).
REMASTER_MAP = {
    "the-last-of-us-remastered": "the-last-of-us",
    "metroid-prime-remastered": "metroid-prime",
    "the-legend-of-zelda-tears-of-the-kingdom-nintendo": (
        "the-legend-of-zelda-tears-of-the-kingdom"
    ),
    "the-legend-of-zelda-breath-of-the-wild-nintendo": (
        "the-legend-of-zelda-breath-of-the-wild"
    ),
    "xenoblade-chronicles-2-nintendo-switch-2-edition": "xenoblade-chronicles-2",
    "the-witcher-3-wild-hunt-remastered": "the-witcher-3-wild-hunt",
    "dark-souls-remastered": "dark-souls",
    "divinity-original-sin-ii-definitive-edition": "divinity-original-sin-ii",
    "northgard-definitive-edition": "northgard",
    "okami-hd": "okami",
    "fable-anniversary": "fable",
    "sonic-adventure-dx-directors-cut": "sonic-adventure",
    "sonic-adventure-dx": "sonic-adventure",
    "kingdom-come-deliverance-next-gen-update": "kingdom-come-deliverance",
}

# Remakes / distinct releases that must NEVER collapse into each other.
NEVER_MERGE_SLUGS = {
    "the-legend-of-zelda-ocarina-of-time",
    "the-legend-of-zelda-ocarina-of-time-3d",
    "the-legend-of-zelda-ocarina-of-time-2026",
    "god-of-war",
    "god-of-war-2005",
    "resident-evil-4",
    "resident-evil-4-2005",
    "final-fantasy-vii-remake-intergrade",
    "gothic-1-remake",
}


def slugify(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "item"


def http_json(url: str) -> dict | list:
    req = urllib.request.Request(url, headers=MC_UA)
    with urllib.request.urlopen(req, timeout=45) as response:
        return json.loads(response.read().decode())


def http_text(url: str) -> str:
    req = urllib.request.Request(url, headers=OC_UA)
    with urllib.request.urlopen(req, timeout=45) as response:
        return response.read().decode("utf-8", "ignore")


def oc_parse_state(html: str) -> dict:
    match = re.search(
        r'<script[^>]*type="application/json"[^>]*>(.*?)</script>', html, re.S
    )
    if not match:
        return {}
    raw = unescape(match.group(1).replace("&q;", '"').replace("&s;", "'"))
    return json.loads(raw)


def fetch_oc_reviews(game_id: int, title: str) -> list[dict]:
    path = slugify(title)
    out: list[dict] = []
    seen: set[str] = set()
    page = 1
    while page <= 30:
        url = f"https://opencritic.com/game/{game_id}/{path}/reviews"
        if page > 1:
            url += f"?page={page}"
        try:
            html = http_text(url)
        except Exception as error:
            print(f"  OC fail {game_id} p{page}: {error}")
            break
        state = oc_parse_state(html)
        batch = []
        for key, value in state.items():
            if (
                isinstance(value, list)
                and value
                and isinstance(value[0], dict)
                and "Outlet" in value[0]
                and "score" in value[0]
            ):
                batch = value
                break
        if not batch:
            break
        for review in batch:
            outlet = (review.get("Outlet") or {}).get("name") or ""
            score = review.get("score")
            if not outlet or score is None or score < 0:
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
    return out


def fetch_mc_reviews(slug: str) -> list[dict]:
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
        url = (
            "https://backend.metacritic.com/reviews/metacritic/critic/games/"
            f"{slug}/web?" + urllib.parse.urlencode(params)
        )
        try:
            data = http_json(url)
        except Exception as error:
            print(f"  MC fail {slug}: {error}")
            return items
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


def merge_reviews(existing: list[dict], new_reviews: list[dict]) -> list[dict]:
    by_key: dict[str, dict] = {}
    for review in existing + new_reviews:
        key = (review.get("slug") or slugify(review["publication"])).lower()
        prev = by_key.get(key)
        if prev is None:
            by_key[key] = review
            continue
        # Prefer Metacritic numeric grades; otherwise keep the newer corrected OC score.
        if review.get("source") == "metacritic" and prev.get("source") != "metacritic":
            by_key[key] = review
        elif prev.get("source") == "metacritic" and review.get("source") != "metacritic":
            continue
        else:
            by_key[key] = review
    return list(by_key.values())


def critic_mean(reviews: list[dict]) -> float | None:
    scores = [
        r["score"]
        for r in reviews
        if r.get("slug") not in USER_SLUGS and r.get("score") is not None
    ]
    if not scores:
        return None
    return sum(scores) / len(scores)


def is_remaster_title(title: str) -> bool:
    if re.search(r"\bremake\b", title, re.I):
        return False
    if re.search(r"\bremaster(?:ed)?\b", title, re.I):
        return True
    if re.search(r"nintendo switch 2 edition", title, re.I):
        return True
    return False


def remaster_base_slug(slug: str, title: str, seed: dict) -> str | None:
    if slug in NEVER_MERGE_SLUGS:
        return None
    if slug in REMASTER_MAP:
        target = REMASTER_MAP[slug]
        if target in NEVER_MERGE_SLUGS:
            return None
        return target if target in seed or target != slug else target
    if not is_remaster_title(title):
        return None
    # Strip remaster / Switch 2 edition suffixes and slugify.
    base = title
    base = re.sub(
        r"\s*[-:]\s*Nintendo Switch 2 Edition(?:\s*\+.*)?$",
        "",
        base,
        flags=re.I,
    )
    base = re.sub(r"\s*[-:]?\s*Remastered\s*$", "", base, flags=re.I)
    base = re.sub(r"\s*[-:]?\s*Remaster\s*$", "", base, flags=re.I)
    base = base.strip(" -:")
    candidate = slugify(base)
    if candidate and candidate != slug:
        return candidate
    return None


def merge_game_into(seed: dict, source_slug: str, target_slug: str) -> None:
    source = seed[source_slug]
    if target_slug not in seed:
        # Promote remaster entry to the base slug/title.
        base_title = source["title"]
        base_title = re.sub(
            r"\s*[-:]\s*Nintendo Switch 2 Edition(?:\s*\+.*)?$",
            "",
            base_title,
            flags=re.I,
        )
        base_title = re.sub(r"\s*[-:]?\s*Remastered\s*$", "", base_title, flags=re.I)
        base_title = re.sub(r"\s*[-:]?\s*Remaster\s*$", "", base_title, flags=re.I)
        seed[target_slug] = {
            **source,
            "title": base_title.strip(" -:"),
            "reviews": list(source.get("reviews") or []),
        }
        if source_slug != target_slug:
            del seed[source_slug]
        print(f"  promote {source_slug} → {target_slug}")
        return

    target = seed[target_slug]
    target["reviews"] = merge_reviews(
        target.get("reviews") or [], source.get("reviews") or []
    )
    # Prefer earlier release year / union platforms.
    if source.get("release_year") and (
        target.get("release_year") is None
        or source["release_year"] < target.get("release_year", 9999)
    ):
        # keep the original game's year when merging a remaster into it
        pass
    if source.get("platforms"):
        plats = list(dict.fromkeys((target.get("platforms") or []) + source["platforms"]))
        target["platforms"] = plats
    for key in (
        "cover",
        "mc_slug",
        "metascore",
        "opencritic_id",
        "opencritic_score",
        "opencritic_recommend",
        "mc_user_score",
    ):
        if not target.get(key) and source.get(key) is not None:
            target[key] = source[key]
    del seed[source_slug]
    print(
        f"  merge {source_slug} → {target_slug} "
        f"(reviews now {len(target['reviews'])})"
    )


def main() -> None:
    raise SystemExit("Metacritic and OpenCritic re-fetch is disabled (compliance).")
    seed = json.loads(SEED.read_text())
    print(f"games before: {len(seed)}")

    # --- Fix OpenCritic scores ---
    oc_games = [(slug, game) for slug, game in seed.items() if game.get("opencritic_id")]
    print(f"re-fetching OpenCritic reviews for {len(oc_games)} games…")
    for slug, game in oc_games:
        oid = int(game["opencritic_id"])
        title = game.get("title") or slug
        print(f"OC {slug}")
        fresh = fetch_oc_reviews(oid, title)
        time.sleep(0.05)
        if not fresh:
            print("  (no reviews)")
            continue
        # Drop old OpenCritic critic rows, keep MC + user aggregates, add fresh OC.
        kept = [
            r
            for r in (game.get("reviews") or [])
            if r.get("source") != "opencritic"
        ]
        game["reviews"] = merge_reviews(kept, fresh)
        mean = critic_mean(game["reviews"])
        print(f"  n={len(fresh)} oc, total={len(game['reviews'])}, mean≈{mean}")

    # --- Add Metacritic reviews for thin / OC-skewed titles ---
    suspicious = []
    for slug, game in seed.items():
        mean = critic_mean(game.get("reviews") or [])
        crit = [
            r
            for r in (game.get("reviews") or [])
            if r.get("slug") not in USER_SLUGS
        ]
        if mean is None:
            continue
        if mean >= 99 or mean <= 5 or (
            game.get("opencritic_id") and not any(r.get("source") == "metacritic" for r in crit)
        ):
            suspicious.append(slug)

    print(f"Metacritic backfill candidates: {len(suspicious)}")
    for slug in suspicious:
        game = seed[slug]
        mc_slug = game.get("mc_slug") or slug
        # try a few slug variants
        candidates = [mc_slug]
        if mc_slug.endswith("-2026"):
            candidates.append(mc_slug[: -len("-2026")])
        if slug != mc_slug:
            candidates.append(slug)
        fetched = []
        for candidate in dict.fromkeys(candidates):
            fetched = fetch_mc_reviews(candidate)
            time.sleep(0.08)
            if fetched:
                game["mc_slug"] = candidate
                break
        if not fetched:
            print(f"  MC miss {slug}")
            continue
        before = critic_mean(game.get("reviews") or [])
        game["reviews"] = merge_reviews(game.get("reviews") or [], fetched)
        after = critic_mean(game["reviews"])
        print(
            f"  MC {slug}: +{len(fetched)} "
            f"mean {before} → {after}"
        )

    # --- Merge remasters into base games ---
    print("merging remasters…")
    # Iterate on a snapshot of keys because we mutate seed.
    for slug in list(seed.keys()):
        if slug not in seed:
            continue
        game = seed[slug]
        target = remaster_base_slug(slug, game.get("title") or "", seed)
        if not target or target == slug:
            continue
        # Don't merge remakes accidentally.
        if re.search(r"\bremake\b", game.get("title") or "", re.I):
            continue
        merge_game_into(seed, slug, target)

    # Drop empty / report extremes
    print("spot-check extremes:")
    ranked = []
    for slug, game in seed.items():
        mean = critic_mean(game.get("reviews") or [])
        if mean is None:
            continue
        ranked.append((mean, slug, game.get("title"), len(game.get("reviews") or [])))
    ranked.sort(reverse=True)
    for row in ranked[:8]:
        print("  top", row)
    for row in ranked[-8:]:
        print("  bot", row)

    SEED.write_text(json.dumps(seed, indent=2) + "\n")
    print(f"games after: {len(seed)}")
    print(
        "reviews",
        sum(len(g.get("reviews") or []) for g in seed.values()),
    )


if __name__ == "__main__":
    main()
