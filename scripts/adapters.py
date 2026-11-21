#!/usr/bin/env python3
"""Metacritic + OpenCritic review/detail adapters used by scrape_catalog."""

from __future__ import annotations

import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from html import unescape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OC_INDEX = ROOT / "data" / "opencritic_index.json"
COVERS = ROOT / "static" / "covers"
MC_API_KEY = "1MOZgmNFxvmljaQR1X9KAij9Mo4xAY3u"

MC_UA = {
    "User-Agent": "Mozilla/5.0",
    "Accept": "application/json",
    "Referer": "https://www.metacritic.com/",
}
OC_UA = {
    "User-Agent": "Mozilla/5.0",
    "Accept": "text/html,application/xhtml+xml",
    "Referer": "https://opencritic.com/",
}

_oc_index: dict | None = None


def slugify(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "item"


def norm(title: str) -> str:
    t = title.lower()
    t = re.sub(r"\([^)]*\)", "", t)
    t = re.sub(r"[^a-z0-9]+", " ", t)
    return re.sub(r"\s+", " ", t).strip()


def fold_key(title: str) -> str:
    t = title
    for pattern in [
        r"\s*[-:]\s*Game of the Year(?: Edition)?\s*$",
        r"\s*[-:]\s*GOTY(?: Edition)?\s*$",
        r"\s*[-:]\s*Complete Edition\s*$",
        r"\s*[-:]\s*Definitive Edition\s*$",
        r"\s*[-:]\s*Gold Edition\s*$",
        r"\s*[-:]\s*Deluxe Edition\s*$",
        r"\s+Legacy\s*$",
        r"\s+Windows Edition\s*$",
        r"\s*[-:]?\s*Remastered\s*$",
        r"\s*[-:]?\s*Remaster\s*$",
        r"\s*[-:]?\s*Director'?s Cut\s*$",
        r"\s+Edition\s*$",
        r"™|®",
    ]:
        t = re.sub(pattern, "", t, flags=re.I).strip(" -:")
    return norm(t)


def search_queries(title: str) -> list[str]:
    queries = [title]
    stripped = title
    for pattern in [
        r"\s*[-:]\s*Game of the Year(?: Edition)?\s*$",
        r"\s*[-:]\s*GOTY(?: Edition)?\s*$",
        r"\s*[-:]\s*Complete Edition\s*$",
        r"\s*[-:]\s*Definitive Edition\s*$",
        r"\s*[-:]\s*Gold Edition\s*$",
        r"\s*[-:]\s*Deluxe Edition\s*$",
        r"\s*[-:]\s*Emperor Edition\s*$",
        r"\s*[-:]\s*Legendary Edition\s*$",
        r"\s+Legacy\s*$",
        r"\s+Windows Edition\s*$",
        r"\s+Edition\s*$",
        r"™|®",
    ]:
        stripped = re.sub(pattern, "", stripped, flags=re.I).strip(" -:")
    if stripped and stripped.lower() != title.lower():
        queries.append(stripped)
    out: list[str] = []
    for q in queries:
        if q and q not in out:
            out.append(q)
    return out


def _title_tokens(s: str) -> set[str]:
    stop = {"the", "a", "an", "of", "and", "edition", "remastered", "remaster"}
    return {t for t in norm(s).split() if t and t not in stop}


def similar_enough(want: str, cand: str) -> bool:
    if want == cand:
        return True
    wt, ct = _title_tokens(want), _title_tokens(cand)
    if not wt or not ct:
        return False
    if len(wt & ct) / len(wt) < 0.7:
        return False
    if len(ct) > len(wt) * 2 + 2:
        return False
    return True


def http_json(url: str, headers: dict | None = None) -> dict | list:
    req = urllib.request.Request(url, headers=headers or MC_UA)
    with urllib.request.urlopen(req, timeout=45) as response:
        return json.loads(response.read().decode())


def http_bytes(url: str, referer: str = "https://www.metacritic.com/") -> bytes:
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "Mozilla/5.0", "Referer": referer},
    )
    with urllib.request.urlopen(req, timeout=45) as response:
        return response.read()


def http_text(url: str, headers: dict) -> str:
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=45) as response:
        return response.read().decode("utf-8", "ignore")


def _mc_score(game: dict) -> float | None:
    summary = game.get("criticScoreSummary") or {}
    if summary.get("score") is not None:
        return float(summary["score"])
    if game.get("metascore") is not None:
        return float(game["metascore"])
    return None


def search_mc(title: str) -> dict | None:
    scored_exact: list[dict] = []
    scored_fuzzy: list[dict] = []
    unscored: list[dict] = []
    for query in search_queries(title):
        q = urllib.parse.quote(query)
        url = (
            "https://backend.metacritic.com/finder/metacritic/search/"
            f"{q}/web?apiKey={MC_API_KEY}&limit=10"
        )
        try:
            data = http_json(url)
        except Exception:
            continue
        items = (data.get("data") or {}).get("items") or []
        games = [it for it in items if it.get("type") == "game-title" and it.get("slug")]
        want = norm(query)
        for game in games:
            cand = norm(game.get("title") or "")
            if not similar_enough(want, cand):
                continue
            has_score = _mc_score(game) is not None
            exact = cand == want
            if exact and has_score:
                scored_exact.append(game)
            elif has_score:
                scored_fuzzy.append(game)
            else:
                unscored.append(game)
        if scored_exact:
            return scored_exact[0]
    if scored_exact:
        return scored_exact[0]
    if scored_fuzzy:
        return scored_fuzzy[0]
    return unscored[0] if unscored else None


def mc_details(slug: str) -> dict | None:
    url = (
        "https://backend.metacritic.com/composer/metacritic/pages/games/"
        f"{slug}/web?apiKey={MC_API_KEY}"
    )
    try:
        data = http_json(url)
    except urllib.error.HTTPError:
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
    platforms = []
    for platform in item.get("platforms") or []:
        name = platform.get("name") or platform.get("shortName")
        if name:
            platforms.append(name)
    if not platforms and item.get("platform"):
        platforms.append(item["platform"])
    return {
        "title": item.get("title") or slug,
        "slug": item.get("slug") or slug,
        "metascore": summary.get("score"),
        "review_count": summary.get("reviewCount") or 0,
        "premiere_year": item.get("premiereYear"),
        "cover_url": cover_url,
        "platforms": platforms,
    }


def mc_reviews(slug: str) -> list[dict]:
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
        data = http_json(url)
        batch = ((data.get("data") or {}).get("items")) or []
        total = (data.get("data") or {}).get("totalResults") or 0
        items.extend(batch)
        offset += len(batch)
        if not batch or offset >= total:
            break
        time.sleep(0.06)
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


def load_oc_index() -> dict:
    global _oc_index
    if _oc_index is None:
        if not OC_INDEX.exists():
            _oc_index = {"by_slug": {}, "by_fold": {}, "id_to_slug": {}}
        else:
            _oc_index = json.loads(OC_INDEX.read_text())
            if "id_to_slug" not in _oc_index:
                _oc_index["id_to_slug"] = {
                    str(v): k for k, v in (_oc_index.get("by_slug") or {}).items()
                }
    return _oc_index


def search_oc(title: str, *, fuzzy: bool = False) -> tuple[int, str] | None:
    """Resolve OpenCritic id/slug. Fuzzy scan of the full index is opt-in (slow)."""
    idx = load_oc_index()
    by_slug = idx.get("by_slug") or {}
    by_fold = idx.get("by_fold") or {}
    id_to_slug = idx.get("id_to_slug") or {}
    for query in search_queries(title):
        slug = slugify(query)
        if slug in by_slug:
            return int(by_slug[slug]), slug
        fold = slug.replace("-", " ")
        if fold in by_fold:
            gid = int(by_fold[fold])
            return gid, id_to_slug.get(str(gid), slug)
        soft = fold_key(query)
        if soft in by_fold:
            gid = int(by_fold[soft])
            return gid, id_to_slug.get(str(gid), slugify(soft))
        if not fuzzy:
            continue
        want = soft or norm(query)
        for fold_name, gid in by_fold.items():
            if similar_enough(want, fold_name):
                gid = int(gid)
                return gid, id_to_slug.get(str(gid), slugify(fold_name))
    return None


def oc_parse_state(html: str) -> dict:
    match = re.search(
        r'<script[^>]*type="application/json"[^>]*>(.*?)</script>', html, re.S
    )
    if not match:
        return {}
    raw = unescape(match.group(1).replace("&q;", '"').replace("&s;", "'"))
    return json.loads(raw)


def oc_reviews(game_id: int, slug: str) -> list[dict]:
    out: list[dict] = []
    seen: set[str] = set()
    page = 1
    while page <= 30:
        url = f"https://opencritic.com/game/{game_id}/{slug}/reviews"
        if page > 1:
            url += f"?page={page}"
        try:
            html = http_text(url, OC_UA)
        except Exception:
            break
        state = oc_parse_state(html)
        batch = []
        for value in state.values():
            if isinstance(value, list) and value and isinstance(value[0], dict):
                if "score" in value[0] and "Outlet" in value[0]:
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


def oc_details(game_id: int, slug: str) -> dict | None:
    try:
        html = http_text(f"https://opencritic.com/game/{game_id}/{slug}", OC_UA)
    except Exception:
        return None
    state = oc_parse_state(html)
    game = state.get(f"game/{game_id}") or {}
    if not game:
        return None
    year = None
    date = game.get("firstReleaseDate") or ""
    if isinstance(date, str) and len(date) >= 4 and date[:4].isdigit():
        year = int(date[:4])
    platforms = []
    for platform in game.get("Platforms") or game.get("platforms") or []:
        name = platform.get("name") or platform.get("shortName")
        if name:
            platforms.append(name)
    images = game.get("images") or {}
    box = images.get("box") or {}
    cover_url = box.get("og") or box.get("sm") or (images.get("banner") or {}).get("og")
    if isinstance(cover_url, str) and cover_url.startswith("//"):
        cover_url = "https:" + cover_url
    return {
        "title": game.get("name") or slug.replace("-", " ").title(),
        "slug": slug,
        "opencritic_id": game_id,
        "opencritic_score": game.get("topCriticScore"),
        "review_count": game.get("numReviews") or 0,
        "premiere_year": year,
        "cover_url": cover_url,
        "platforms": platforms,
    }


def _source_rank(source: str | None) -> int:
    # Priority: direct outlet site > Metacritic > OpenCritic > unknown.
    return {
        "outlet": 3,
        "metacritic": 2,
        "opencritic": 1,
    }.get(source or "", 0)


# Collapse aggregator naming drift onto one critic identity.
# Keys are slugify(name) / raw slug forms; values are canonical outlet slugs.
OUTLET_ALIASES: dict[str, str] = {
    "gamesradar": "gamesradar",
    "gamesradar+": "gamesradar",
    "games-radar": "gamesradar",
    "games-radar+": "gamesradar",
    "mmorpgcom": "mmorpg-com",
    "mmorpg-com": "mmorpg-com",
    "sectorsk": "sector-sk",
    "sector-sk": "sector-sk",
    "multiplayerit": "multiplayer-it",
    "multiplayer-it": "multiplayer-it",
    "everyeyeit": "everyeye-it",
    "everyeye-it": "everyeye-it",
    "gamesch": "games-ch",
    "games-ch": "games-ch",
    "gryonlinepl": "gryonline-pl",
    "gryonline-pl": "gryonline-pl",
    "gamerant": "game-rant",
    "game-rant": "game-rant",
    "firingsquad": "firing-squad",
    "firing-squad": "firing-squad",
    "trustedreviews": "trusted-reviews",
    "trusted-reviews": "trusted-reviews",
    "ragequitgr": "ragequit-gr",
    "ragequit-gr": "ragequit-gr",
    "restartrun": "restart-run",
    "restart-run": "restart-run",
    "the-a-v-club": "the-av-club",
    "the-av-club": "the-av-club",
    "generaci-n-xbox": "generacion-xbox",
    "generacion-xbox": "generacion-xbox",
    "gamerno": "gamer-no",
    "gamer-no": "gamer-no",
    "fingerguns": "finger-guns",
    "finger-guns": "finger-guns",
    "gamecritics": "game-critics",
    "game-critics": "game-critics",
    "gamestm": "games-tm",
    "games-tm": "games-tm",
    "play-zine": "play-zine",
    "play!-zine": "play-zine",
    # Direct adapters: keep stable ids even when aggregator uses display names.
    "ign": "ign",
    "pc-gamer": "pc-gamer",
    "eurogamer": "eurogamer",
    "gamespot": "gamespot",
}


def canonicalize_outlet_slug(slug: str) -> str:
    key = (slug or "").lower().strip()
    if not key:
        return "item"
    # Strip residual punctuation the aggregator sometimes leaves in.
    key = re.sub(r"[^a-z0-9]+", "-", key).strip("-") or "item"
    return OUTLET_ALIASES.get(key, key)


def outlet_merge_key(review: dict) -> str:
    """Stable outlet identity across Metacritic / OpenCritic / outlet scrapes.

    Prefer the publication display name (slugified) over API-provided slugs —
    MC and OC often disagree on slugs (multiplayer-it vs multiplayerit) for the
    same outlet, which used to create duplicate critic rows.
    """
    name = (review.get("publication") or review.get("outlet") or "").strip()
    if name:
        return canonicalize_outlet_slug(slugify(name))
    return canonicalize_outlet_slug(review.get("slug") or "item")


def _norm_url(url: str | None) -> str:
    if not url:
        return ""
    u = url.strip().split("?", 1)[0].rstrip("/").lower()
    for prefix in ("https://", "http://"):
        if u.startswith(prefix):
            u = u[len(prefix) :]
            break
    if u.startswith("www."):
        u = u[4:]
    return u


def merge_reviews(existing: list[dict], new_reviews: list[dict]) -> list[dict]:
    """One row per outlet. Prefer outlet-site > MC > OC when collapsing dups."""
    by_key: dict[str, dict] = {}
    url_to_key: dict[str, str] = {}

    def _prefer(prev: dict, review: dict) -> dict:
        prev_rank = _source_rank(prev.get("source"))
        new_rank = _source_rank(review.get("source"))
        if new_rank > prev_rank:
            return review
        if new_rank < prev_rank:
            return prev
        if not prev.get("excerpt") and review.get("excerpt"):
            return review
        if prev.get("excerpt") and not review.get("excerpt"):
            return prev
        # Same rank: prefer the one with a cleaner URL / keep existing.
        return prev

    for review in existing + new_reviews:
        key = outlet_merge_key(review)
        url_key = _norm_url(review.get("url"))
        # Same review URL from MC+OC → collapse even if outlet names differ slightly.
        if url_key and url_key in url_to_key:
            key = url_to_key[url_key]
        prev = by_key.get(key)
        if prev is None:
            by_key[key] = dict(review)
        else:
            by_key[key] = _prefer(prev, review)
        if url_key:
            url_to_key[url_key] = key
        chosen = by_key[key]
        chosen["slug"] = key
    return list(by_key.values())


def dedupe_seed_reviews(seed: dict) -> tuple[dict, int]:
    """Collapse MC/OC/outlet duplicates in-place across the whole seed.

    Returns (seed, total_rows_removed).
    """
    removed = 0
    for entry in seed.values():
        reviews = entry.get("reviews")
        if not isinstance(reviews, list) or not reviews:
            continue
        before = len(reviews)
        entry["reviews"] = merge_reviews(reviews, [])
        removed += max(0, before - len(entry["reviews"]))
    return seed, removed


def canon_platforms(names: list[str]) -> list[str]:
    alias = {
        "playstation 5": "ps5",
        "ps5": "ps5",
        "playstation 4": "ps4",
        "ps4": "ps4",
        "xbox series x/s": "xbox-series",
        "xbox series x": "xbox-series",
        "xbox series s": "xbox-series",
        "xbox one": "xbox-one",
        "nintendo switch 2": "switch-2",
        "nintendo switch": "switch",
        "pc": "pc",
        "windows pc": "pc",
        "windows": "pc",
    }
    out: list[str] = []
    seen: set[str] = set()
    for name in names:
        key = alias.get(name.strip().lower())
        if key is None:
            key = re.sub(r"[^a-z0-9]+", "-", name.strip().lower()).strip("-")
        if key and key not in seen:
            seen.add(key)
            out.append(key)
    return out


def download_cover(game_slug: str, cover_url: str | None) -> str | None:
    if not cover_url:
        return None
    COVERS.mkdir(parents=True, exist_ok=True)
    dest = COVERS / f"{game_slug}.jpg"
    if dest.exists() and dest.stat().st_size > 1000:
        return f"/covers/{game_slug}.jpg"
    try:
        dest.write_bytes(http_bytes(cover_url))
        return f"/covers/{game_slug}.jpg"
    except Exception:
        return None
