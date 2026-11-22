#!/usr/bin/env python3
"""One-shot integrity repair for the committed catalog.

Drops blocked sources, nulls missing scores, keeps yes/no verdicts as 100/0, merges duplicate
outlet spellings, normalizes review URLs, clips excerpts, and restores
original release years. Safe to run again.
"""

from __future__ import annotations

import argparse
import json
import re
import sqlite3
from collections import Counter, defaultdict
from html import unescape
from pathlib import Path
from urllib.parse import parse_qsl, urlencode, urlparse, urlunparse

ROOT = Path(__file__).resolve().parents[1]
BLOCKED_SLUGS = {"metacritic-users", "opencritic-recommend"}
# Yes/no outlets. 0 and 100 are the verdict and stay in the score.
VERDICT_SLUGS = {"kotaku"}
BAD_HOSTS = {
    "savethechildren.org",
    "metacritic.com",
    "opencritic.com",
    "urldefense.proofpoint.com",
    "proofpoint.com",
    "anyforsoft.com",
    "imdb.com",
}
MULTI_SUFFIX = {
    "co.uk", "com.au", "com.br", "co.jp", "com.tr", "com.mx", "co.nz",
    "com.tw", "co.kr", "com.ar", "com.pt", "com.es", "co.za", "com.sg",
    "com.hk", "co.in", "com.co", "com.ua", "co.il",
}
TRACKING = {
    "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content",
    "utm_id", "fbclid", "gclid", "mc_eid", "mc_cid", "mkt_tok", "igshid",
    "ref", "ref_src", "_ga", "yclid", "ocid", "cmpid",
}
# Spelling variants of the same publication. Regional IGN editions stay separate.
ALIASES = {
    "gamesradar": "gamesradar",
    "gamesradar+": "gamesradar",
    "games-radar": "gamesradar",
    "games-radar+": "gamesradar",
    "gamerant": "game-rant",
    "game-rant": "game-rant",
    "firingsquad": "firing-squad",
    "firing-squad": "firing-squad",
    "trustedreviews": "trusted-reviews",
    "trusted-reviews": "trusted-reviews",
    "fingerguns": "finger-guns",
    "finger-guns": "finger-guns",
    "ign-italy": "ign-italia",
    "ign-italia": "ign-italia",
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
    "gameblogfr": "gameblog-fr",
    "gameblog": "gameblog-fr",
    "gameblog-fr": "gameblog-fr",
    "gamecritics": "game-critics",
    "game-critics": "game-critics",
    "the-outerhaven": "the-outerhaven",
    "the-outerhaven-productions": "the-outerhaven",
    "xgn-nl": "xgn",
    "xgn": "xgn",
    "uagnait": "uagna",
    "uagna-it": "uagna",
    "uagna": "uagna",
    "oyungezer-online": "oyungezer",
    "oyungezer": "oyungezer",
}
# Original release, not a later port that overwrote the row.
RELEASES = {
    "resident-evil-2": (2019, ["ps4", "xbox-one", "pc"], {"ps4": 2019, "xbox-one": 2019, "pc": 2019}),
    "resident-evil-4": (2023, ["ps5", "ps4", "xbox-series", "pc"], {"ps5": 2023, "ps4": 2023, "xbox-series": 2023, "pc": 2023}),
    "resident-evil-7": (2017, ["ps4", "xbox-one", "pc"], {"ps4": 2017, "xbox-one": 2017, "pc": 2017}),
    "sonic-frontiers": (2022, ["ps5", "ps4", "xbox-series", "xbox-one", "switch", "pc"], {"ps5": 2022, "ps4": 2022, "xbox-series": 2022, "xbox-one": 2022, "switch": 2022, "pc": 2022}),
    "stellar-blade": (2024, ["ps5"], {"ps5": 2024}),
    "lies-of-p": (2023, ["ps5", "xbox-series", "pc"], {"ps5": 2023, "xbox-series": 2023, "pc": 2023}),
    "super-mario-bros-wonder": (2023, ["switch"], {"switch": 2023}),
    "xenoblade-chronicles-2": (2017, ["switch"], {"switch": 2017}),
}
MISSING_YEARS = {
    "spyparty": 2018,
    "cosmoteer-starship-architect-and-commander": 2023,
    "trans-siberian-railway-simulator": 2024,
    "worldbox-god-simulator": 2021,
    "ultrakill": 2020,
    "dead-as-disco": 2026,
    "goat-of-duty": 2019,
    "splitgate": 2019,
    "wild-west-pioneers": 2026,
}


def canonical_slug(slug: str) -> str:
    key = re.sub(r"[^a-z0-9]+", "-", (slug or "").lower()).strip("-")
    return ALIASES.get(key, key)


def registrable(host: str) -> str:
    host = host.lower().removeprefix("www.").strip(".")
    parts = [part for part in host.split(".") if part]
    if len(parts) >= 3 and ".".join(parts[-2:]) in MULTI_SUFFIX:
        return ".".join(parts[-3:])
    if len(parts) >= 2:
        return ".".join(parts[-2:])
    return host


def unwrap_proofpoint(url: str) -> str:
    parsed = urlparse(url)
    host = (parsed.hostname or "").lower()
    if "proofpoint.com" not in host:
        return url
    query = dict(parse_qsl(parsed.query, keep_blank_values=True))
    raw = query.get("u") or ""
    if not raw:
        return url
    raw = raw.replace("-3A", ":").replace("-2F", "/").replace("_", "/")
    # proofpoint uses __ for // after the first replacement sometimes.
    raw = raw.replace("https:/", "https://").replace("http:/", "http://")
    if raw.startswith("http://") or raw.startswith("https://"):
        return raw.split("?", 1)[0]
    return url


def normalize_url(url: str | None) -> str | None:
    if not url:
        return None
    link = unescape(str(url)).strip().replace("&a;", "&").replace("&amp;", "&")
    link = unwrap_proofpoint(link)
    if not (link.startswith("http://") or link.startswith("https://")):
        return None
    parsed = urlparse(link)
    host = (parsed.hostname or "").lower().removeprefix("www.")
    if not host:
        return None
    if any(host == bad or host.endswith("." + bad) for bad in BAD_HOSTS):
        return None
    scheme = "https"
    kept = [
        (key, value)
        for key, value in parse_qsl(parsed.query, keep_blank_values=True)
        if key.lower() not in TRACKING and not key.lower().startswith("utm_")
    ]
    path = parsed.path.rstrip("/") or ""
    return urlunparse((scheme, host, path, "", urlencode(kept), ""))


def clip_excerpt(text: str | None) -> str | None:
    if not text:
        return None
    raw = " ".join(text.split())
    if not raw:
        return None
    sentences = re.split(r"(?<=[.!?])\s+", raw)
    kept: list[str] = []
    for sentence in sentences:
        if len(kept) >= 2:
            break
        candidate = " ".join(kept + [sentence]).strip()
        if len(candidate) > 300 and kept:
            break
        kept.append(sentence)
        if len(" ".join(kept)) >= 300:
            break
    out = " ".join(kept).strip()
    if len(out) > 300:
        window = out[:300]
        match = re.search(r"^.*[.!?](?=\s|$)", window)
        out = (match.group(0) if match else window.rsplit(" ", 1)[0]).strip()
    changed = out != raw
    if changed:
        out = out.rstrip()
        if not out.endswith("…"):
            out = out.rstrip(".!?") + "…"
    return out or None


def numeric_score(score, slug: str):
    if score is None:
        return None
    try:
        value = float(score)
    except (TypeError, ValueError):
        return None
    if slug in VERDICT_SLUGS:
        return value
    # A stored 0 on a numeric outlet is a missing score, not a real zero.
    if value == 0.0:
        return None
    return value


def repair_seed(path: Path) -> None:
    data = json.loads(path.read_text())
    for slug, game in data.items():
        if slug in RELEASES:
            year, platforms, _years = RELEASES[slug]
            game["release_year"] = year
            game["platforms"] = platforms
        elif slug in MISSING_YEARS and not game.get("release_year"):
            game["release_year"] = MISSING_YEARS[slug]
        seen_url: set[str] = set()
        seen_outlet: dict[str, dict] = {}
        kept = []
        for review in game.get("reviews") or []:
            outlet = canonical_slug(review.get("slug") or "")
            if outlet in BLOCKED_SLUGS:
                continue
            review["slug"] = outlet
            score = numeric_score(review.get("score"), outlet)
            review["score"] = score
            review["excerpt"] = clip_excerpt(review.get("excerpt"))
            review["url"] = normalize_url(review.get("url"))
            if review["url"]:
                if review["url"] in seen_url:
                    continue
                seen_url.add(review["url"])
            if outlet in seen_outlet:
                prev = seen_outlet[outlet]
                if score is not None and prev.get("score") is not None:
                    prev["score"] = (float(prev["score"]) + score) / 2
                elif prev.get("score") is None and score is not None:
                    prev["score"] = score
                if not prev.get("excerpt") and review.get("excerpt"):
                    prev["excerpt"] = review["excerpt"]
                if not prev.get("url") and review.get("url"):
                    prev["url"] = review["url"]
                continue
            seen_outlet[outlet] = review
            kept.append(review)
        game["reviews"] = kept
    # Drop links whose domain is not that outlet's own domain.
    domains = outlet_domains(data)
    for game in data.values():
        for review in game.get("reviews") or []:
            url = review.get("url")
            domain = domains.get(review.get("slug") or "")
            if not url or not domain:
                review["url"] = None
                continue
            host = urlparse(url).hostname or ""
            if registrable(host) != domain:
                review["url"] = None
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    print(f"seed {path} games={len(data)}")


def outlet_domains(data: dict) -> dict[str, str]:
    counts: dict[str, Counter] = defaultdict(Counter)
    for game in data.values():
        for review in game.get("reviews") or []:
            url = review.get("url")
            if not url:
                continue
            host = urlparse(url).hostname or ""
            if not host:
                continue
            counts[review.get("slug") or "" ][registrable(host)] += 1
    return {slug: counter.most_common(1)[0][0] for slug, counter in counts.items() if counter}


def repair_db(path: Path, seed_path: Path) -> None:
    seed = json.loads(seed_path.read_text())
    domains = outlet_domains(seed)
    con = sqlite3.connect(path)
    con.execute("PRAGMA foreign_keys=ON")
    con.execute("ALTER TABLE games ADD COLUMN platform_years TEXT") if "platform_years" not in {
        row[1] for row in con.execute("PRAGMA table_info(games)")
    } else None
    # Reviews may already have platforms from a previous open.
    cols = {row[1] for row in con.execute("PRAGMA table_info(reviews)")}
    if "reviewed_on" not in cols:
        con.execute("ALTER TABLE reviews ADD COLUMN reviewed_on TEXT")

    outlets = {slug: (oid, name) for oid, slug, name in con.execute("SELECT id, slug, name FROM outlets")}
    # Create canonical outlet rows and move reviews onto them.
    for slug, (oid, name) in list(outlets.items()):
        canon = canonical_slug(slug)
        if canon == slug or canon in BLOCKED_SLUGS:
            continue
        if canon not in outlets:
            con.execute(
                "UPDATE outlets SET slug=? WHERE id=?",
                (canon, oid),
            )
            outlets[canon] = (oid, name)
            del outlets[slug]
        else:
            target_id = outlets[canon][0]
            rows = con.execute(
                "SELECT id, game_id, raw_score, raw_score_norm, excerpt, review_url FROM reviews WHERE outlet_id=?",
                (oid,),
            ).fetchall()
            for rid, game_id, raw, norm, excerpt, url in rows:
                existing = con.execute(
                    "SELECT id, raw_score, raw_score_norm, excerpt, review_url FROM reviews WHERE outlet_id=? AND game_id=?",
                    (target_id, game_id),
                ).fetchone()
                if existing is None:
                    con.execute("UPDATE reviews SET outlet_id=? WHERE id=?", (target_id, rid))
                else:
                    eid, eraw, enorm, eexcerpt, eurl = existing
                    if raw is not None and eraw is not None:
                        raw = (raw + eraw) / 2
                        norm = (norm + enorm) / 2
                    elif raw is None:
                        raw, norm = eraw, enorm
                    excerpt = eexcerpt or excerpt
                    url = eurl or url
                    con.execute(
                        "UPDATE reviews SET raw_score=?, raw_score_norm=?, excerpt=?, review_url=? WHERE id=?",
                        (raw, norm if raw is not None else enorm, excerpt, url, eid),
                    )
                    con.execute("DELETE FROM reviews WHERE id=?", (rid,))
            con.execute("DELETE FROM outlets WHERE id=?", (oid,))
            del outlets[slug]
    con.commit()

    # Reload ids after merges.
    outlets = {slug: oid for oid, slug in con.execute("SELECT id, slug FROM outlets")}
    removed = 0
    nulled = 0
    unlinked = 0
    clipped = 0
    for rid, slug, raw, norm, excerpt, url in con.execute(
        """SELECT r.id, o.slug, r.raw_score, r.raw_score_norm, r.excerpt, r.review_url
           FROM reviews r JOIN outlets o ON o.id = r.outlet_id"""
    ):
        if slug in BLOCKED_SLUGS:
            con.execute("DELETE FROM reviews WHERE id=?", (rid,))
            removed += 1
            continue
        if slug in VERDICT_SLUGS and raw is None and norm is not None:
            raw = norm
        score = numeric_score(raw, slug)
        new_excerpt = clip_excerpt(excerpt)
        new_url = normalize_url(url)
        domain = domains.get(slug)
        if new_url and domain:
            host = urlparse(new_url).hostname or ""
            if registrable(host) != domain:
                new_url = None
                unlinked += 1
        elif new_url and not domain:
            new_url = None
            unlinked += 1
        if new_excerpt and excerpt and new_excerpt != excerpt:
            clipped += 1
        if score is None and raw is not None:
            nulled += 1
        con.execute(
            "UPDATE reviews SET raw_score=?, excerpt=?, review_url=? WHERE id=?",
            (score, new_excerpt, new_url, rid),
        )
    # Drop duplicate normalized URLs inside one game, keeping the first row.
    seen: dict[tuple[int, str], int] = {}
    dupes = 0
    for rid, game_id, url in con.execute(
        "SELECT id, game_id, review_url FROM reviews WHERE review_url IS NOT NULL ORDER BY id"
    ):
        key = (game_id, url)
        if key in seen:
            con.execute("UPDATE reviews SET review_url=NULL WHERE id=?", (rid,))
            dupes += 1
        else:
            seen[key] = rid

    for slug, (year, platforms, years) in RELEASES.items():
        con.execute(
            "UPDATE games SET release_year=?, platforms=?, platform_years=? WHERE slug=?",
            (year, json.dumps(platforms), json.dumps(years), slug),
        )
    for slug, year in MISSING_YEARS.items():
        con.execute(
            "UPDATE games SET release_year=? WHERE slug=? AND release_year IS NULL",
            (year, slug),
        )
    con.commit()
    print(
        f"db removed={removed} nulled={nulled} unlinked={unlinked} clipped={clipped} dup_urls={dupes}"
    )
    con.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=Path, default=ROOT / "sourcecritic.db")
    parser.add_argument("--seed", type=Path, default=ROOT / "data" / "seed_reviews.json")
    args = parser.parse_args()
    repair_seed(args.seed)
    repair_db(args.db, args.seed)


if __name__ == "__main__":
    main()
