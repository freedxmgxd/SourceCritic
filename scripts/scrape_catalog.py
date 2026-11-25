#!/usr/bin/env python3
"""
Scrape critic reviews for every active game in data/catalog_games.json.

Only direct outlet sites (IGN / PC Gamer / Eurogamer / GamesRadar / …).
Metacritic and OpenCritic fetches are disabled.

Does not grow catalog_games — add games to the catalog manually when needed.
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
PIPELINE_LOCK = Path("/tmp/sourcecritic_pipeline.lock")

from adapters import (  # noqa: E402
    canon_platforms,
    dedupe_seed_reviews,
    download_cover,
    is_mc_edition_slug,
    load_oc_index,
    mc_details,
    mc_reviews,
    merge_reviews,
    oc_details,
    oc_reviews,
    prefer_base_mc_slug,
    search_mc,
    search_oc,
)
from catalog_lib import (  # noqa: E402
    CATALOG_GAMES,
    NEVER_MERGE_SLUGS,
    active_games,
    load_games_catalog,
    load_outlets_catalog,
    merge_outlet_aliases,
    save_games_catalog,
    save_outlets_catalog,
    upsert_outlet_candidate,
    utc_now,
)
from outlet_adapters import (  # noqa: E402
    ADAPTERS,
    catalog_outlet_bootstrap,
    scrape_configured_outlets,
)

SEED = ROOT / "data" / "seed_reviews.json"


def load_seed() -> dict:
    if not SEED.exists():
        return {}
    return json.loads(SEED.read_text())


def save_seed(seed: dict) -> None:
    tmp = SEED.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(seed, indent=2, ensure_ascii=False) + "\n")
    tmp.replace(SEED)


def save_catalog_touched(memory: dict, touched: set[str]) -> None:
    """Merge only scraped games back to disk so discovery merges aren't wiped."""
    if not touched:
        return
    disk = load_games_catalog()
    mem_games = memory.get("games") or {}
    disk_games = disk.setdefault("games", {})
    for slug in touched:
        if slug in mem_games:
            disk_games[slug] = mem_games[slug]
    save_games_catalog(disk)


def sources_present(entry: dict | None) -> tuple[bool, bool]:
    if not entry:
        return False, False
    reviews = entry.get("reviews") or []
    has_mc = entry.get("mc_slug") is not None or any(
        r.get("source") == "metacritic" for r in reviews
    )
    has_oc = entry.get("opencritic_id") is not None or any(
        r.get("source") == "opencritic" for r in reviews
    )
    return has_mc, has_oc


def resolve_mc(game: dict) -> tuple[str | None, dict | None]:
    ext = game.get("external") or {}
    mc_slug = ext.get("metacritic")
    details = None

    def _load(slug: str | None) -> tuple[str | None, dict | None]:
        if not slug:
            return None, None
        meta = mc_details(slug)
        time.sleep(0.06)
        if not meta:
            return None, None
        return meta["slug"], meta

    if mc_slug:
        # Catalog sometimes points at Gold/Complete/port SKUs — prefer base.
        if is_mc_edition_slug(mc_slug):
            base = prefer_base_mc_slug(mc_slug)
            slug, details = _load(base)
            if details:
                return slug, details
        slug, details = _load(mc_slug)
        if details and not is_mc_edition_slug(slug):
            return slug, details
        if details and is_mc_edition_slug(slug):
            base = prefer_base_mc_slug(slug)
            if base and base != slug:
                base_slug, base_details = _load(base)
                if base_details:
                    return base_slug, base_details
            # Last resort: keep edition only if base missing.
            return slug, details
    hit = search_mc(game.get("title") or game["slug"])
    time.sleep(0.08)
    if not hit:
        return mc_slug, None
    mc_slug = hit["slug"]
    details = mc_details(mc_slug)
    time.sleep(0.06)
    if details and is_mc_edition_slug(details["slug"]):
        base = prefer_base_mc_slug(details["slug"])
        if base and base != details["slug"]:
            base_slug, base_details = _load(base)
            if base_details:
                return base_slug, base_details
    return (details["slug"], details) if details else (mc_slug, None)


def _oc_target_ok(game_slug: str, oc_slug: str | None) -> bool:
    """Refuse OpenCritic hits that belong to a different never-merge sibling.

    Classic OoT N64 must not pick up the 2026 remake's OpenCritic id just
    because titles fold to the same key after stripping years.
    """
    if not oc_slug:
        return True
    if game_slug not in NEVER_MERGE_SLUGS and oc_slug not in NEVER_MERGE_SLUGS:
        return True
    if game_slug == oc_slug:
        return True
    # Both in the never-merge set (or OC maps to another protected slug) → only
    # accept an exact slug match.
    if oc_slug in NEVER_MERGE_SLUGS and oc_slug != game_slug:
        return False
    if game_slug in NEVER_MERGE_SLUGS and oc_slug != game_slug:
        # e.g. catalog slug is …-2026 but OC returned bare …-ocarina-of-time
        return False
    return True


def resolve_oc(game: dict) -> tuple[int | None, str | None, dict | None]:
    ext = game.get("external") or {}
    oc_id = ext.get("opencritic")
    game_slug = game.get("slug") or ""
    idx = load_oc_index()
    id_to_slug = idx.get("id_to_slug") or {}
    if oc_id is not None:
        oc_id = int(oc_id)
        oc_slug = id_to_slug.get(str(oc_id)) or game_slug
        if not _oc_target_ok(game_slug, oc_slug):
            print(
                f"  skip OC id={oc_id} slug={oc_slug} (never-merge vs {game_slug})",
                flush=True,
            )
            return None, None, None
        details = oc_details(oc_id, oc_slug)
        time.sleep(0.06)
        if details:
            return oc_id, details.get("slug") or oc_slug, details
        return oc_id, oc_slug, None
    hit = search_oc(game.get("title") or game_slug)
    if not hit:
        return None, None, None
    oc_id, oc_slug = hit
    # Prefer index mapping of id → canonical catalog slug when present.
    mapped = id_to_slug.get(str(oc_id))
    if mapped:
        oc_slug = mapped
    if not _oc_target_ok(game_slug, oc_slug):
        print(
            f"  skip OC search hit id={oc_id} slug={oc_slug} (never-merge vs {game_slug})",
            flush=True,
        )
        return None, None, None
    details = oc_details(oc_id, oc_slug)
    time.sleep(0.06)
    return oc_id, oc_slug, details


def outlets_present(entry: dict | None, adapter_ids: list[str]) -> set[str]:
    """Outlets already fetched from the publication site (not aggregator copies)."""
    if not entry:
        return set()
    have: set[str] = set()
    for review in entry.get("reviews") or []:
        if review.get("source") != "outlet":
            continue
        slug = (review.get("slug") or "").lower()
        if slug in adapter_ids:
            have.add(slug)
    return have


def parse_scraped_at(value: str | None) -> datetime | None:
    if not value or not isinstance(value, str):
        return None
    text = value.strip()
    if not text:
        return None
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        dt = datetime.fromisoformat(text)
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


def is_stale(game: dict, max_age_days: float | None) -> bool:
    """True if never scraped or last_scraped_at older than max_age_days."""
    if max_age_days is None or max_age_days <= 0:
        return False
    scraped = parse_scraped_at(game.get("last_scraped_at"))
    if scraped is None:
        # In seed but never stamped — treat as due for a refresh pass.
        return True
    age = datetime.now(timezone.utc) - scraped
    return age.total_seconds() >= max_age_days * 86400


def _port_edition_mc_slugs(catalog: dict | None, base_slug: str, base_mc: str | None) -> list[str]:
    """MC slugs of catalog stubs merged into this game (Complete/Deluxe ports)."""
    if not catalog:
        return []
    out: list[str] = []
    for entry in (catalog.get("games") or {}).values():
        if entry.get("status") != "merged_into":
            continue
        if entry.get("merged_into") != base_slug:
            continue
        mc = (entry.get("external") or {}).get("metacritic")
        if mc and mc != base_mc and mc not in out:
            out.append(mc)
    return out


def scrape_one(
    slug: str,
    game: dict,
    seed: dict,
    *,
    refresh: bool,
    need_mc: bool,
    need_oc: bool,
    need_outlets: bool,
    outlet_ids: list[str],
    catalog: dict | None = None,
) -> str:
    """
    Returns status: skipped|updated|added|no_reviews|error
    """
    existing = seed.get(slug)
    has_mc, has_oc = sources_present(existing)
    have_outlets = outlets_present(existing, outlet_ids)
    # refresh=True forces a full re-fetch so new outlet scores can land.
    want_mc = need_mc and (refresh or not has_mc)
    want_oc = need_oc and (refresh or not has_oc)
    missing_outlets = [o for o in outlet_ids if o not in have_outlets] if need_outlets else []
    want_outlets = need_outlets and (refresh or bool(missing_outlets))

    reviews: list[dict] = merge_reviews(list((existing or {}).get("reviews") or []), [])
    pre_dedupe_n = len((existing or {}).get("reviews") or [])
    deduped_only = pre_dedupe_n > len(reviews)

    if (
        not want_mc
        and not want_oc
        and not want_outlets
        and existing
        and reviews
    ):
        if deduped_only:
            existing["reviews"] = reviews
            seed[slug] = existing
            print(
                f"  dedupe {pre_dedupe_n}→{len(reviews)} (no fetch)",
                flush=True,
            )
            return "updated"
        print("  skip (sources already present)", flush=True)
        return "skipped"

    if refresh:
        print("  refresh (stale or --refresh)", flush=True)

    if refresh:
        # Drop aggregator/outlet rows we're about to replace; keep unknown/other.
        drop = set()
        if want_mc:
            drop.add("metacritic")
        if want_oc:
            drop.add("opencritic")
        if want_outlets:
            drop.add("outlet")
        reviews = [r for r in reviews if r.get("source") not in drop]

    # 1) Direct outlet sites first — highest-trust scores win merge conflicts.
    outlet_hits = 0
    if want_outlets:
        try:
            targets = outlet_ids if refresh else missing_outlets
            direct = scrape_configured_outlets(
                game.get("title") or slug,
                reviews,
                adapters=targets,
            )
            outlet_hits = len(direct)
            if direct:
                reviews = merge_reviews(reviews, direct)
                print(
                    f"  outlets +{outlet_hits}: "
                    + ", ".join(f"{r['slug']}={r['score']:.0f}" for r in direct),
                    flush=True,
                )
        except Exception as error:
            print(f"  outlet scrape fail: {error}", flush=True)

    # 2) Metacritic aggregator (fills outlets we don't have direct).
    mc_slug = None
    mc_meta = None
    if want_mc:
        mc_slug, mc_meta = resolve_mc(game)
        if mc_slug:
            try:
                reviews = merge_reviews(reviews, mc_reviews(mc_slug))
                time.sleep(0.05)
            except Exception as error:
                print(f"  MC reviews fail {mc_slug}: {error}", flush=True)
            # Fold port/Complete SKU critic rows into the same game (tagged by platform).
            for port_slug in _port_edition_mc_slugs(catalog, slug, mc_slug):
                try:
                    port_rows = mc_reviews(port_slug)
                    if port_rows:
                        reviews = merge_reviews(reviews, port_rows)
                        print(
                            f"  MC port +{len(port_rows)} from {port_slug}",
                            flush=True,
                        )
                    time.sleep(0.05)
                except Exception as error:
                    print(f"  MC port fail {port_slug}: {error}", flush=True)

    # 3) OpenCritic last — only adds outlets still missing after outlet+MC.
    oc_id = None
    oc_slug = None
    oc_meta = None
    if want_oc:
        oc_id, oc_slug, oc_meta = resolve_oc(game)
        if oc_id and oc_slug:
            try:
                reviews = merge_reviews(reviews, oc_reviews(oc_id, oc_slug))
                time.sleep(0.05)
            except Exception as error:
                print(f"  OC reviews fail {oc_id}/{oc_slug}: {error}", flush=True)

    # Final collapse in case MC/OC reintroduced a dup against an outlet row.
    reviews = merge_reviews(reviews, [])

    # If we only needed one source and already had reviews, keep going even if
    # the new source returned nothing — but brand-new entries need ≥1 review.
    if not reviews:
        print("  no reviews from MC/OC/outlets", flush=True)
        return "no_reviews"

    cover_url = None
    platforms: list[str] = []
    year = None
    metascore = None
    oc_score = None
    title = game.get("title") or slug

    if mc_meta:
        title = mc_meta.get("title") or title
        cover_url = mc_meta.get("cover_url")
        platforms = mc_meta.get("platforms") or []
        year = mc_meta.get("premiere_year")
        metascore = mc_meta.get("metascore")
        mc_slug = mc_meta.get("slug") or mc_slug
    if oc_meta:
        if not cover_url:
            cover_url = oc_meta.get("cover_url")
        if not platforms:
            platforms = oc_meta.get("platforms") or []
        if year is None:
            year = oc_meta.get("premiere_year")
        oc_score = oc_meta.get("opencritic_score")
        if not mc_meta:
            title = oc_meta.get("title") or title

    if existing:
        title = existing.get("title") or title
        # When we resolved a base MC SKU, prefer its year over a stale port year
        # (e.g. Switch 2 2026 overwriting a 2023 remake).
        if year is None:
            year = existing.get("release_year")
        elif (
            existing.get("release_year") is not None
            and mc_meta
            and not is_mc_edition_slug(mc_slug)
            and int(year) < int(existing["release_year"])
        ):
            pass  # keep earlier base release year from mc_meta
        elif existing.get("release_year") is not None and not mc_meta:
            year = existing["release_year"]
        existing_plats = existing.get("platforms") or []
        if platforms and existing_plats:
            platforms = list(dict.fromkeys([*existing_plats, *platforms]))
        elif not platforms:
            platforms = list(existing_plats)
        if existing.get("metascore") is not None and metascore is None:
            metascore = existing["metascore"]
        if existing.get("opencritic_score") is not None and oc_score is None:
            oc_score = existing["opencritic_score"]
        # Don't keep a Gold/Complete edition slug once we have a base SKU.
        if mc_slug and is_mc_edition_slug(existing.get("mc_slug")):
            pass
        else:
            mc_slug = mc_slug or existing.get("mc_slug")
        oc_id = oc_id if oc_id is not None else existing.get("opencritic_id")

    cover = download_cover(slug, cover_url) or (existing or {}).get("cover") or f"/covers/{slug}.jpg"

    entry = {
        "title": title,
        "cover": cover,
        "reviews": reviews,
    }
    if metascore is not None:
        entry["metascore"] = metascore
    if mc_slug:
        entry["mc_slug"] = mc_slug
    if oc_id is not None:
        entry["opencritic_id"] = int(oc_id)
    if oc_score is not None:
        entry["opencritic_score"] = oc_score
    if year is not None:
        entry["release_year"] = year
    if existing and existing.get("platforms") and not mc_meta and not oc_meta:
        entry["platforms"] = existing["platforms"]
    elif platforms:
        if all(isinstance(p, str) and re_is_slug(p) for p in platforms):
            entry["platforms"] = list(platforms)
        else:
            entry["platforms"] = canon_platforms(
                [p if isinstance(p, str) else str(p) for p in platforms]
            )
    if existing and existing.get("synopsis"):
        entry["synopsis"] = existing["synopsis"]

    seed[slug] = entry

    # Persist resolved external ids back onto the catalog game.
    ext = dict(game.get("external") or {})
    if mc_slug:
        ext["metacritic"] = mc_slug
    if oc_id is not None:
        ext["opencritic"] = int(oc_id)
    game["external"] = ext
    game["last_scraped_at"] = utc_now()

    n_mc = sum(1 for r in reviews if r.get("source") == "metacritic")
    n_oc = sum(1 for r in reviews if r.get("source") == "opencritic")
    n_out = sum(1 for r in reviews if r.get("source") == "outlet")
    print(
        f"  {'~' if existing else '+'} {slug} n={len(reviews)} "
        f"(mc={n_mc} oc={n_oc} outlet={n_out}) "
        f"meta={metascore} oc_score={oc_score}",
        flush=True,
    )
    return "updated" if existing else "added"


def re_is_slug(value: str) -> bool:
    return bool(value) and value == value.lower() and " " not in value


def acquire_direct_lock():
    """Prevent overlapping scrapes when not launched by run_pipeline.py."""
    if os.environ.get("SOURCECRITIC_PIPELINE_CHILD") == "1":
        return None
    PIPELINE_LOCK.parent.mkdir(parents=True, exist_ok=True)
    handle = PIPELINE_LOCK.open("w")
    try:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as error:
        handle.close()
        raise SystemExit(
            f"another pipeline/scrape holds {PIPELINE_LOCK}; aborting"
        ) from error
    handle.write(f"pid={os.getpid()}\nscrape_catalog\n")
    handle.flush()
    return handle


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report-only", action="store_true", help="Coverage report only")
    parser.add_argument("--limit", type=int, default=0, help="Max games to process (0=all)")
    parser.add_argument("--refresh", action="store_true", help="Re-fetch sources even if present")
    parser.add_argument(
        "--stale-days",
        type=float,
        default=7.0,
        help="Re-scrape games whose last_scraped_at is older than N days (0 disables)",
    )
    parser.add_argument("--only-missing", action="store_true", help="Only games absent from seed")
    parser.add_argument("--skip-mc", action="store_true")
    parser.add_argument("--skip-oc", action="store_true")
    parser.add_argument(
        "--with-outlets",
        action="store_true",
        default=True,
        help="Scrape configured outlet sites first (default: on)",
    )
    parser.add_argument(
        "--skip-outlets",
        action="store_true",
        help="Disable direct outlet-site scraping",
    )
    parser.add_argument(
        "--outlets-only",
        action="store_true",
        help="Skip MC/OC; only fill missing/stale outlet-site reviews",
    )
    parser.add_argument(
        "--dedupe-only",
        action="store_true",
        help="Collapse MC/OC/outlet duplicate rows in seed and exit (no network)",
    )
    parser.add_argument(
        "--outlet",
        action="append",
        default=[],
        help="Outlet adapter id (repeatable). Default: ign,pc-gamer,eurogamer,gamesradar",
    )
    parser.add_argument("--slug", action="append", default=[], help="Only these catalog slugs")
    parser.add_argument(
        "--save-every",
        type=int,
        default=1,
        help="Persist seed/catalog every N successful writes",
    )
    args = parser.parse_args()
    if args.skip_outlets:
        args.with_outlets = False

    lock_handle = None
    if not args.report_only and not args.dedupe_only:
        lock_handle = acquire_direct_lock()

    catalog = load_games_catalog()
    games = active_games(catalog)
    seed = load_seed()

    if args.dedupe_only:
        before_rows = sum(len(g.get("reviews") or []) for g in seed.values())
        seed, removed = dedupe_seed_reviews(seed)
        after_rows = sum(len(g.get("reviews") or []) for g in seed.values())
        save_seed(seed)
        print(
            f"dedupe-only: removed={removed} rows {before_rows}→{after_rows} "
            f"across {len(seed)} games",
            flush=True,
        )
        return

    # Ensure configured outlet adapters are registered; fold near-dup outlet rows.
    outlets_cat = load_outlets_catalog()
    alias_merges = merge_outlet_aliases(outlets_cat)
    for cand in catalog_outlet_bootstrap():
        upsert_outlet_candidate(outlets_cat, cand)
    save_outlets_catalog(outlets_cat)
    if alias_merges:
        print(f"outlet catalog aliases folded: {alias_merges}", flush=True)

    stale_days = None if args.stale_days <= 0 else float(args.stale_days)
    missing = [s for s in sorted(games) if s not in seed]
    stale_slugs = [s for s in games if is_stale(games[s], stale_days)]
    print(
        f"catalog active={len(games)} seed={len(seed)} missing_from_seed={len(missing)} "
        f"stale(>{stale_days or 0:g}d)={len(stale_slugs)} → {CATALOG_GAMES}",
        flush=True,
    )
    if args.report_only:
        if missing:
            print("sample missing:", ", ".join(missing[:25]))
        if stale_slugs:
            print("sample stale:", ", ".join(sorted(stale_slugs)[:25]))
        return

    outlet_ids = args.outlet or ["ign", "pc-gamer", "eurogamer", "gamesradar"]
    outlet_ids = [o for o in outlet_ids if o in ADAPTERS]
    # Aggregator APIs stay off regardless of flags.
    need_mc = False
    need_oc = False
    need_outlets = not args.skip_outlets

    if need_mc or need_oc:
        load_oc_index()

    if args.slug:
        work = [s for s in args.slug if s in games]
    else:
        work = sorted(games)
        if args.only_missing:
            work = [s for s in work if s not in seed]
        elif args.outlets_only:
            work = [
                s
                for s in work
                if s in seed
                and (
                    len(outlets_present(seed.get(s), outlet_ids)) < len(outlet_ids)
                    or is_stale(games[s], stale_days)
                )
            ]
        else:
            # missing → incomplete sources → stale → fresh
            def sort_key(slug: str) -> tuple[int, str]:
                if slug not in seed:
                    return (0, slug)
                has_mc, has_oc = sources_present(seed.get(slug))
                if need_mc and not has_mc:
                    return (1, slug)
                if need_oc and not has_oc:
                    return (1, slug)
                if need_outlets and len(outlets_present(seed.get(slug), outlet_ids)) < len(
                    outlet_ids
                ):
                    return (1, slug)
                if is_stale(games[slug], stale_days):
                    return (2, slug)
                return (3, slug)

            work = sorted(work, key=sort_key)

    if args.limit and args.limit > 0:
        work = work[: args.limit]

    print(
        f"work={len(work)} mc={need_mc} oc={need_oc} outlets={need_outlets} "
        f"stale_days={stale_days} adapters={outlet_ids}",
        flush=True,
    )

    try:
        stats = {"added": 0, "updated": 0, "skipped": 0, "no_reviews": 0, "error": 0}
        writes = 0
        touched: set[str] = set()
        for i, slug in enumerate(work):
            game = games[slug]
            print(f"[{i+1}/{len(work)}] {game.get('title') or slug}", flush=True)
            do_refresh = bool(args.refresh) or is_stale(game, stale_days)
            try:
                status = scrape_one(
                    slug,
                    game,
                    seed,
                    refresh=do_refresh,
                    need_mc=need_mc,
                    need_oc=need_oc,
                    need_outlets=need_outlets,
                    outlet_ids=outlet_ids,
                    catalog=catalog,
                )
            except Exception as error:
                print(f"  error: {error}", flush=True)
                status = "error"
            stats[status] = stats.get(status, 0) + 1
            if status in ("added", "updated"):
                touched.add(slug)
                writes += 1
                if writes % max(1, args.save_every) == 0:
                    save_seed(seed)
                    save_catalog_touched(catalog, touched)
                    touched.clear()

        save_seed(seed)
        save_catalog_touched(catalog, touched)
        print(
            f"done seed={len(seed)} added={stats['added']} updated={stats['updated']} "
            f"skipped={stats['skipped']} no_reviews={stats['no_reviews']} "
            f"error={stats['error']}",
            flush=True,
        )
    finally:
        if lock_handle is not None:
            try:
                fcntl.flock(lock_handle.fileno(), fcntl.LOCK_UN)
            finally:
                lock_handle.close()

if __name__ == "__main__":
    main()
