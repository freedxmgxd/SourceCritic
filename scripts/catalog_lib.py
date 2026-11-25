#!/usr/bin/env python3
"""Shared catalog helpers: canonical remaster fold, catalog load/save/upsert."""

from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data"
CATALOG_GAMES = DATA / "catalog_games.json"
CATALOG_OUTLETS = DATA / "catalog_outlets.json"

# Remasters / enhanced editions → base catalog slug.
CANONICAL_SLUG: dict[str, str] = {
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
    # Scholar of the First Sin = base + DLC edition → keep SEPARATE from Dark Souls II.
    "divinity-original-sin-ii-definitive-edition": "divinity-original-sin-ii",
    # Steam slugify ("Sin 2") vs Metacritic roman numerals ("Sin II")
    "divinity-original-sin-2": "divinity-original-sin-ii",
    "divinity-original-sin-2-definitive-edition": "divinity-original-sin-ii",
    "northgard-definitive-edition": "northgard",
    "okami-hd": "okami",
    "fable-anniversary": "fable",
    "sonic-adventure-dx-directors-cut": "sonic-adventure",
    "sonic-adventure-dx": "sonic-adventure",
    "kingdom-come-deliverance-next-gen-update": "kingdom-come-deliverance",
    "bioshock-remastered": "bioshock",
    "bioshock-2-remastered": "bioshock-2",
    "ori-and-the-blind-forest-definitive-edition": "ori-and-the-blind-forest",
    "little-nightmares-enhanced-edition": "little-nightmares",
    "titan-quest-anniversary-edition": "titan-quest",
    "death-stranding-directors-cut": "death-stranding",
    # Slug twins / remaster folds caught as live duplicates
    "castlevania-lords-of-shadow-mirror-of-fate-hd": (
        "castlevania-lords-of-shadow-mirror-of-fate"
    ),
    "system-shock-2-25th-anniversary": "system-shock-2-25th-anniversary-remaster",
    "dying-light-the-beast-restored-land": (
        "dying-light-the-beast-restored-land-definitive"
    ),
    "metal-gear-solid-master-collection-version": (
        "metal-gear-solid-master-collection-vol-2"
    ),
    "tom-clancy-s-rainbow-six-siege-test-server": "tom-clancys-rainbow-six-siege",
}

# Remakes / distinct releases that must never collapse.
NEVER_MERGE_SLUGS: set[str] = {
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

EDITION_STRIP_PATTERNS = [
    r"\s*[-:]\s*Game of the Year(?: Edition)?\s*$",
    r"\s*[-:]\s*GOTY(?: Edition)?\s*$",
    r"\s*[-:]\s*Complete Edition\s*$",
    r"\s*[-:]\s*Definitive Edition\s*$",
    r"\s*[-:]\s*Gold Edition\s*$",
    r"\s*[-:]\s*Deluxe Edition\s*$",
    r"\s*[-:]\s*Emperor Edition\s*$",
    r"\s*[-:]\s*Legendary Edition\s*$",
    r"\s*[-:]\s*Nintendo Switch 2 Edition(?:\s*\+.*)?$",
    r"\s*[-:]?\s*Director'?s Cut\s*$",
    r"\s*[-:]?\s*Remastered\s*$",
    r"\s*[-:]?\s*Remaster\s*$",
    r"\s*[-:]?\s*Enhanced Edition\s*$",
    r"\s*[-:]?\s*Anniversary Edition\s*$",
    r"\s*[-:]?\s*Next-Gen Update\s*$",
    r"\s+Legacy\s*$",
    r"\s+Windows Edition\s*$",
    r"\s+Edition\s*$",
    r"™|®",
]


def slugify(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "item"


def canonicalize_outlet_slug(slug: str) -> str:
    """Delegate to adapters so OUTLET_ALIASES stays the single source of truth."""
    from adapters import canonicalize_outlet_slug as _canon

    return _canon(slug)


def norm(title: str) -> str:
    t = title.lower()
    t = re.sub(r"\([^)]*\)", "", t)
    t = re.sub(r"[^a-z0-9]+", " ", t)
    return re.sub(r"\s+", " ", t).strip()


def fold_key(title: str) -> str:
    t = title
    for pattern in EDITION_STRIP_PATTERNS:
        t = re.sub(pattern, "", t, flags=re.I).strip(" -:")
    return norm(t)


def is_remaster_title(title: str) -> bool:
    if re.search(r"\bremake\b", title, re.I):
        return False
    if re.search(r"\bremaster(?:ed)?\b", title, re.I):
        return True
    if re.search(r"nintendo switch 2 edition", title, re.I):
        return True
    if re.search(r"\bdefinitive edition\b", title, re.I):
        return True
    if re.search(r"\benhanced edition\b", title, re.I):
        return True
    if re.search(r"\bdirector'?s cut\b", title, re.I):
        return True
    return False


def canonical_slug(slug: str, title: str | None = None) -> str:
    """Resolve remaster/enhanced slug to catalog base slug."""
    if slug in NEVER_MERGE_SLUGS:
        return slug
    if slug in CANONICAL_SLUG:
        target = CANONICAL_SLUG[slug]
        return slug if target in NEVER_MERGE_SLUGS else target
    if title and is_remaster_title(title):
        base = fold_key(title)
        candidate = slugify(base)
        if candidate and candidate != slug and candidate not in NEVER_MERGE_SLUGS:
            return candidate
    return slug


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def empty_games_catalog() -> dict[str, Any]:
    return {"version": 1, "updated_at": utc_now(), "games": {}}


def empty_outlets_catalog() -> dict[str, Any]:
    return {"version": 1, "updated_at": utc_now(), "outlets": {}}


def load_games_catalog(path: Path | None = None) -> dict[str, Any]:
    p = path or CATALOG_GAMES
    if not p.exists():
        return empty_games_catalog()
    data = json.loads(p.read_text())
    data.setdefault("version", 1)
    data.setdefault("games", {})
    return data


def save_games_catalog(catalog: dict[str, Any], path: Path | None = None) -> None:
    p = path or CATALOG_GAMES
    p.parent.mkdir(parents=True, exist_ok=True)
    catalog["updated_at"] = utc_now()
    tmp = p.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + "\n")
    tmp.replace(p)


def load_outlets_catalog(path: Path | None = None) -> dict[str, Any]:
    p = path or CATALOG_OUTLETS
    if not p.exists():
        return empty_outlets_catalog()
    data = json.loads(p.read_text())
    data.setdefault("version", 1)
    data.setdefault("outlets", {})
    return data


def save_outlets_catalog(catalog: dict[str, Any], path: Path | None = None) -> None:
    p = path or CATALOG_OUTLETS
    p.parent.mkdir(parents=True, exist_ok=True)
    catalog["updated_at"] = utc_now()
    tmp = p.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + "\n")
    tmp.replace(p)


def _index_catalog(games: dict[str, dict]) -> tuple[dict[str, str], dict[str, str]]:
    by_norm: dict[str, str] = {}
    by_fold: dict[str, str] = {}
    for slug, game in games.items():
        if game.get("status") == "merged_into":
            continue
        title = game.get("title") or slug
        by_norm[norm(title)] = slug
        by_fold[fold_key(title)] = slug
        for alias in game.get("aliases") or []:
            by_norm[norm(alias)] = slug
            by_fold[fold_key(alias)] = slug
        ext = game.get("external") or {}
        mc = ext.get("metacritic")
        if mc:
            by_norm[norm(mc.replace("-", " "))] = slug
    return by_norm, by_fold


def find_existing_slug(
    catalog: dict[str, Any],
    *,
    slug: str | None = None,
    title: str | None = None,
    metacritic: str | None = None,
    opencritic: int | None = None,
    steam_appid: int | None = None,
) -> str | None:
    games: dict[str, dict] = catalog.get("games") or {}
    if slug:
        canon = canonical_slug(slug, title)
        if canon in games and games[canon].get("status") != "merged_into":
            return canon
        if slug in games:
            merged = games[slug].get("merged_into")
            if merged and merged in games:
                return merged
            if games[slug].get("status") != "merged_into":
                return slug
    for game_slug, game in games.items():
        if game.get("status") == "merged_into":
            continue
        ext = game.get("external") or {}
        if metacritic and ext.get("metacritic") == metacritic:
            return game_slug
        if opencritic is not None and ext.get("opencritic") == opencritic:
            return game_slug
        appids = ext.get("steam_appids") or []
        if steam_appid is not None and steam_appid in appids:
            return game_slug
    if title:
        by_norm, by_fold = _index_catalog(games)
        hit = by_norm.get(norm(title)) or by_fold.get(fold_key(title))
        if hit:
            return hit
    return None


def upsert_game_candidate(catalog: dict[str, Any], candidate: dict[str, Any]) -> str:
    """
    Insert or merge a discovery candidate into the games catalog.
    Returns the canonical active slug.
    """
    games: dict[str, dict] = catalog.setdefault("games", {})
    title = (candidate.get("title") or "").strip()
    raw_slug = candidate.get("slug") or (slugify(title) if title else None)
    if not raw_slug and not title:
        raise ValueError("candidate needs title or slug")

    via = candidate.get("discovered_via") or "unknown"
    if isinstance(via, str):
        via_list = [via]
    else:
        via_list = list(via)

    ext_in = dict(candidate.get("external") or {})
    if candidate.get("mc_slug") and "metacritic" not in ext_in:
        ext_in["metacritic"] = candidate["mc_slug"]
    if candidate.get("opencritic_id") is not None and "opencritic" not in ext_in:
        ext_in["opencritic"] = int(candidate["opencritic_id"])
    if candidate.get("steam_appid") is not None:
        ext_in.setdefault("steam_appids", [])
        appid = int(candidate["steam_appid"])
        if appid not in ext_in["steam_appids"]:
            ext_in["steam_appids"].append(appid)
    # Prefer canonical Metacritic slug when the external id is a remaster page.
    if ext_in.get("metacritic"):
        ext_in["metacritic"] = canonical_slug(
            ext_in["metacritic"], title or None
        )

    slug_hint = raw_slug or slugify(title)
    canon = canonical_slug(slug_hint, title or None)

    existing = find_existing_slug(
        catalog,
        slug=canon,
        title=title or None,
        metacritic=ext_in.get("metacritic"),
        opencritic=ext_in.get("opencritic"),
        steam_appid=(ext_in.get("steam_appids") or [None])[0],
    )

    if existing:
        entry = games[existing]
        if title and title not in (entry.get("aliases") or []) and title != entry.get(
            "title"
        ):
            aliases = list(entry.get("aliases") or [])
            if title not in aliases:
                aliases.append(title)
            entry["aliases"] = aliases
        # If discovery title is cleaner / base and entry looks like remaster, prefer base.
        if title and is_remaster_title(entry.get("title") or "") and not is_remaster_title(
            title
        ):
            entry["title"] = title
        discovered = list(entry.get("discovered_via") or [])
        for v in via_list:
            if v not in discovered:
                discovered.append(v)
        entry["discovered_via"] = discovered
        ext = dict(entry.get("external") or {})
        if ext_in.get("metacritic") and not ext.get("metacritic"):
            ext["metacritic"] = ext_in["metacritic"]
        if ext_in.get("opencritic") is not None and ext.get("opencritic") is None:
            ext["opencritic"] = ext_in["opencritic"]
        appids = list(ext.get("steam_appids") or [])
        for aid in ext_in.get("steam_appids") or []:
            if aid not in appids:
                appids.append(aid)
        if appids:
            ext["steam_appids"] = appids
        entry["external"] = ext
        entry["status"] = "active"
        entry.pop("merged_into", None)
        # Mark remaster slug as merged stub if different.
        if slug_hint != existing and slug_hint not in games:
            games[slug_hint] = {
                "slug": slug_hint,
                "title": title or slug_hint,
                "aliases": [],
                "external": {},
                "discovered_via": via_list,
                "status": "merged_into",
                "merged_into": existing,
            }
        return existing

    # New active game at canonical slug.
    display_title = title or canon.replace("-", " ").title()
    if is_remaster_title(display_title) and canon != slug_hint:
        display_title = fold_key(display_title).title()  # fallback; prefer provided base
        # Prefer stripping edition noise for display when we folded.
        cleaned = title or display_title
        for pattern in EDITION_STRIP_PATTERNS:
            cleaned = re.sub(pattern, "", cleaned, flags=re.I).strip(" -:")
        display_title = cleaned or display_title

    entry = {
        "slug": canon,
        "title": display_title,
        "aliases": [],
        "external": {},
        "discovered_via": via_list,
        "status": "active",
    }
    if title and title != display_title:
        entry["aliases"] = [title]
    if ext_in.get("metacritic"):
        entry["external"]["metacritic"] = ext_in["metacritic"]
    if ext_in.get("opencritic") is not None:
        entry["external"]["opencritic"] = ext_in["opencritic"]
    if ext_in.get("steam_appids"):
        entry["external"]["steam_appids"] = list(ext_in["steam_appids"])

    games[canon] = entry
    if slug_hint != canon:
        games[slug_hint] = {
            "slug": slug_hint,
            "title": title or slug_hint,
            "aliases": [],
            "external": {},
            "discovered_via": via_list,
            "status": "merged_into",
            "merged_into": canon,
        }
    return canon


def active_games(catalog: dict[str, Any]) -> dict[str, dict]:
    return {
        slug: game
        for slug, game in (catalog.get("games") or {}).items()
        if game.get("status") != "merged_into"
    }


def merge_outlet_aliases(catalog: dict[str, Any]) -> int:
    """Fold alias outlet rows into their canonical slug. Returns merges applied."""
    outlets: dict[str, dict] = catalog.setdefault("outlets", {})
    merges = 0
    for raw_slug in list(outlets.keys()):
        canon = canonicalize_outlet_slug(raw_slug)
        if canon == raw_slug:
            continue
        entry = outlets.pop(raw_slug)
        merges += 1
        if canon in outlets:
            target = outlets[canon]
            discovered = list(target.get("discovered_via") or [])
            for v in entry.get("discovered_via") or []:
                if v not in discovered:
                    discovered.append(v)
            target["discovered_via"] = discovered
            aliases = list(target.get("aliases") or [])
            for a in [raw_slug, entry.get("name"), *(entry.get("aliases") or [])]:
                if a and a not in aliases and a != target.get("name"):
                    aliases.append(a)
            target["aliases"] = aliases
            if entry.get("homepage") and not target.get("homepage"):
                target["homepage"] = entry["homepage"]
            new_adapter = entry.get("scrape_adapter")
            old_adapter = target.get("scrape_adapter") or "via_aggregator"
            if new_adapter and (
                not old_adapter
                or old_adapter == "via_aggregator"
                or (
                    str(new_adapter).startswith("custom:")
                    and not str(old_adapter).startswith("custom:")
                )
            ):
                target["scrape_adapter"] = new_adapter
        else:
            entry["slug"] = canon
            aliases = list(entry.get("aliases") or [])
            if raw_slug not in aliases:
                aliases.append(raw_slug)
            entry["aliases"] = aliases
            outlets[canon] = entry
    if merges:
        catalog["updated_at"] = utc_now()
    return merges


def upsert_outlet_candidate(catalog: dict[str, Any], candidate: dict[str, Any]) -> str:
    outlets: dict[str, dict] = catalog.setdefault("outlets", {})
    name = (candidate.get("name") or candidate.get("publication") or "").strip()
    slug = canonicalize_outlet_slug(candidate.get("slug") or slugify(name))
    if not slug:
        raise ValueError("outlet candidate needs name or slug")
    via = candidate.get("discovered_via") or "unknown"
    if isinstance(via, str):
        via_list = [via]
    else:
        via_list = list(via)

    if slug in outlets:
        entry = outlets[slug]
        discovered = list(entry.get("discovered_via") or [])
        for v in via_list:
            if v not in discovered:
                discovered.append(v)
        entry["discovered_via"] = discovered
        if name and not entry.get("name"):
            entry["name"] = name
        if candidate.get("homepage") and not entry.get("homepage"):
            entry["homepage"] = candidate["homepage"]
        # Allow upgrading via_aggregator → custom:* when an adapter ships.
        new_adapter = candidate.get("scrape_adapter")
        old_adapter = entry.get("scrape_adapter") or "via_aggregator"
        if new_adapter and (
            not old_adapter
            or old_adapter == "via_aggregator"
            or (
                new_adapter.startswith("custom:")
                and not old_adapter.startswith("custom:")
            )
        ):
            entry["scrape_adapter"] = new_adapter
        if candidate.get("scale"):
            entry["scale"] = candidate["scale"]
        return slug

    outlets[slug] = {
        "slug": slug,
        "name": name or slug,
        "homepage": candidate.get("homepage"),
        "review_index_urls": list(candidate.get("review_index_urls") or []),
        "scrape_adapter": candidate.get("scrape_adapter") or "via_aggregator",
        "scale": candidate.get("scale") or "0-100",
        "discovered_via": via_list,
        "status": "active",
        "last_scraped_at": None,
    }
    return slug
