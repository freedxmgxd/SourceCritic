#!/usr/bin/env python3
"""Collapse duplicate critic rows without dropping a second review.

A row's identity is the outlet slug plus the normalized review URL. The host
is lowercased and a leading ``www.`` is removed; the query, the fragment, and
a trailing slash are dropped. When the URL is empty, the identity is the
outlet plus the score plus the reviewer.

Two rows with that same identity become one. The kept row is the one that has
a URL, a score, and a date. A second review from the same outlet with a
different URL stays.
"""

from __future__ import annotations

import argparse
import json
import sqlite3
from collections import defaultdict
from pathlib import Path
from urllib.parse import urlparse, urlunparse

ROOT = Path(__file__).resolve().parents[1]
import sys

sys.path.insert(0, str(ROOT / "scripts"))

from adapters import canonicalize_outlet_slug, fold_name  # noqa: E402

OUTLETS_PATH = ROOT / "data" / "outlets.json"


def normalize_review_url(url: str | None) -> str:
    """Lowercase host, no query, no fragment, no trailing slash.

    A leading ``www.`` is part of host normalization, and the port is not
    part of the host, so ``https://www.jeuxvideo.com:443/a/?q=1#x`` and
    ``https://jeuxvideo.com/a`` are one URL.
    """
    if not url or not str(url).strip():
        return ""
    parsed = urlparse(str(url).strip())
    scheme = (parsed.scheme or "").lower()
    if scheme not in {"http", "https"}:
        return ""
    host = (parsed.hostname or "").lower()
    if host.startswith("www."):
        host = host[4:]
    if not host:
        return ""
    path = (parsed.path or "").rstrip("/")
    return urlunparse((scheme, host, path, "", "", ""))


def score_token(score) -> str:
    if score is None:
        return ""
    try:
        return f"{float(score):.2f}"
    except (TypeError, ValueError):
        return ""


def reviewer_token(reviewer: str | None) -> str:
    if not reviewer:
        return ""
    return " ".join(str(reviewer).split()).lower()


def dedupe_key(outlet_slug: str, url: str | None, score, reviewer: str | None) -> tuple:
    slug = canonicalize_outlet_slug(outlet_slug or "")
    norm = normalize_review_url(url)
    if norm:
        return ("url", slug, norm)
    return ("bare", slug, score_token(score), reviewer_token(reviewer))


def _completeness(url: str | None, score, date: str | None) -> tuple:
    has_url = bool(normalize_review_url(url))
    has_score = score is not None
    has_date = bool(date and str(date).strip())
    return (int(has_url) + int(has_score) + int(has_date), int(has_url), int(has_score), int(has_date))


def _pair_ok(raw, scale, norm) -> bool:
    if raw is None or scale is None:
        return False
    try:
        raw_f = float(raw)
        scale_f = float(scale)
        norm_f = float(norm)
    except (TypeError, ValueError):
        return False
    if scale_f <= 0:
        return False
    return abs(raw_f / scale_f * 100.0 - norm_f) < 1.5


def prefer_review(incumbent: dict, challenger: dict) -> dict:
    """Keep one review. A URL, a score, and a date outrank a thinner row."""
    inc_rank = _completeness(incumbent.get("url"), incumbent.get("score"), incumbent.get("date"))
    ch_rank = _completeness(challenger.get("url"), challenger.get("score"), challenger.get("date"))
    if ch_rank > inc_rank:
        base, other = dict(challenger), incumbent
    elif ch_rank < inc_rank:
        base, other = dict(incumbent), challenger
    elif _pair_ok(challenger.get("raw_score"), challenger.get("scale_max"), challenger.get("score")) and not _pair_ok(
        incumbent.get("raw_score"), incumbent.get("scale_max"), incumbent.get("score")
    ):
        base, other = dict(challenger), incumbent
    else:
        base, other = dict(incumbent), challenger
    for field in ("url", "date", "author", "excerpt", "title"):
        if not base.get(field) and other.get(field):
            base[field] = other[field]
    for field in ("score", "raw_score", "scale_max"):
        if base.get(field) is None and other.get(field) is not None:
            base[field] = other[field]
    return base


def normalize_date(value) -> str | None:
    if not value:
        return None
    text = str(value).strip()
    if len(text) >= 10 and text[4:5] == "-" and text[7:8] == "-":
        return text[:10]
    return text or None


def relax_review_uniqueness(con: sqlite3.Connection) -> None:
    """Allow two reviews from one outlet when the URLs differ.

    The old UNIQUE(outlet_id, game_id) turned a second article into an
    overwrite, and it also rejected moving an alias onto a slug that already
    had a row.
    """
    cols = [row[1] for row in con.execute("PRAGMA table_info(reviews)")]
    if "reviewer" not in cols:
        con.execute("ALTER TABLE reviews ADD COLUMN reviewer TEXT")
        cols.append("reviewer")
    sql_row = con.execute(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='reviews'"
    ).fetchone()
    sql = (sql_row[0] or "") if sql_row else ""
    if "unique" not in sql.lower():
        con.execute(
            "CREATE INDEX IF NOT EXISTS reviews_outlet_game ON reviews(outlet_id, game_id)"
        )
        return
    defs = {
        "id": "id INTEGER PRIMARY KEY",
        "outlet_id": "outlet_id INTEGER NOT NULL REFERENCES outlets(id)",
        "game_id": "game_id INTEGER NOT NULL REFERENCES games(id)",
        "raw_score": "raw_score REAL",
        "scale_max": "scale_max REAL",
        "raw_score_norm": "raw_score_norm REAL NOT NULL",
        "z_score": "z_score REAL NOT NULL",
        "excerpt": "excerpt TEXT",
        "review_url": "review_url TEXT",
        "reviewed_on": "reviewed_on TEXT",
        "platforms": "platforms TEXT",
        "reviewer": "reviewer TEXT",
    }
    ordered = [name for name in defs if name in cols]
    con.execute("PRAGMA foreign_keys=OFF")
    con.execute(f"CREATE TABLE reviews_new ({', '.join(defs[name] for name in ordered)})")
    listed = ", ".join(ordered)
    con.execute(f"INSERT INTO reviews_new ({listed}) SELECT {listed} FROM reviews")
    con.execute("DROP TABLE reviews")
    con.execute("ALTER TABLE reviews_new RENAME TO reviews")
    con.execute("CREATE INDEX IF NOT EXISTS reviews_outlet_game ON reviews(outlet_id, game_id)")
    con.execute("PRAGMA foreign_keys=ON")


def _catalog_slugs() -> set[str]:
    if not OUTLETS_PATH.exists():
        return set()
    payload = json.loads(OUTLETS_PATH.read_text(encoding="utf-8"))
    return {
        outlet.get("slug")
        for outlet in payload.get("outlets") or []
        if isinstance(outlet, dict) and outlet.get("slug")
    }


def _retarget_outlet(con: sqlite3.Connection, loser_id: int, keeper_id: int) -> None:
    con.execute("UPDATE reviews SET outlet_id=? WHERE outlet_id=?", (keeper_id, loser_id))
    con.execute("DELETE FROM outlets WHERE id=?", (loser_id,))


def _merge_aliased_outlets(con: sqlite3.Connection) -> int:
    merged = 0
    rows = con.execute("SELECT id, slug FROM outlets").fetchall()
    for oid, slug in rows:
        if con.execute("SELECT 1 FROM outlets WHERE id=?", (oid,)).fetchone() is None:
            continue
        canon = canonicalize_outlet_slug(slug)
        if canon == slug:
            continue
        keeper = con.execute("SELECT id FROM outlets WHERE slug=?", (canon,)).fetchone()
        if keeper and keeper[0] != oid:
            _retarget_outlet(con, oid, keeper[0])
            merged += 1
        elif keeper is None:
            con.execute("UPDATE outlets SET slug=? WHERE id=?", (canon, oid))
    return merged


def _merge_same_publication(con: sqlite3.Connection, catalog: set[str]) -> int:
    """Daily Dot / The Daily Dot, and Geeks & Com's two slugs, are one outlet."""
    grouped: dict[str, list[dict]] = defaultdict(list)
    for oid, slug, name in con.execute("SELECT id, slug, name FROM outlets"):
        key = fold_name(name)
        if not key:
            continue
        count = con.execute("SELECT COUNT(*) FROM reviews WHERE outlet_id=?", (oid,)).fetchone()[0]
        grouped[key].append({"id": oid, "slug": slug, "n": count})
    merged = 0
    for members in grouped.values():
        live = []
        for member in members:
            if con.execute("SELECT 1 FROM outlets WHERE id=?", (member["id"],)).fetchone():
                live.append(member)
        if len(live) < 2:
            continue
        keeper = max(
            live,
            key=lambda member: (
                member["slug"] == canonicalize_outlet_slug(member["slug"]),
                member["slug"] in catalog,
                member["n"],
            ),
        )
        for member in live:
            if member["id"] == keeper["id"]:
                continue
            _retarget_outlet(con, member["id"], keeper["id"])
            merged += 1
    return merged


def _row_rank(url, raw, reviewed_on) -> tuple:
    return _completeness(url, raw, reviewed_on)


def _collapse_rows(rows: list[sqlite3.Row]) -> tuple[list[dict], list[int]]:
    """Return updates for kept ids and ids to delete."""
    kept: dict[tuple, dict] = {}
    delete_ids: list[int] = []
    for row in rows:
        key = dedupe_key(row["slug"], row["review_url"], row["raw_score_norm"], row["reviewer"])
        current = {
            "id": row["id"],
            "raw_score": row["raw_score"],
            "scale_max": row["scale_max"],
            "raw_score_norm": row["raw_score_norm"],
            "excerpt": row["excerpt"],
            "review_url": row["review_url"],
            "platforms": row["platforms"],
            "reviewed_on": row["reviewed_on"],
            "reviewer": row["reviewer"],
        }
        previous = kept.get(key)
        if previous is None:
            kept[key] = current
            continue
        prev_rank = _row_rank(previous["review_url"], previous["raw_score"], previous["reviewed_on"])
        next_rank = _row_rank(current["review_url"], current["raw_score"], current["reviewed_on"])
        if next_rank > prev_rank or (
            next_rank == prev_rank
            and _pair_ok(current["raw_score"], current["scale_max"], current["raw_score_norm"])
            and not _pair_ok(previous["raw_score"], previous["scale_max"], previous["raw_score_norm"])
        ):
            base, other = current, previous
        else:
            base, other = previous, current
        delete_ids.append(other["id"])
        for field in ("raw_score", "scale_max", "excerpt", "review_url", "platforms", "reviewed_on", "reviewer"):
            if base.get(field) in (None, "") and other.get(field) not in (None, ""):
                base[field] = other[field]
        if base.get("raw_score") is None and other.get("raw_score") is not None:
            base["raw_score_norm"] = other["raw_score_norm"]
        kept[key] = base
    updates = []
    for item in kept.values():
        updates.append(item)
    # Only write rows that survived a collision or were the keeper. Writing
    # every row is harmless; the caller updates by id.
    return updates, delete_ids


def dedupe_database(con: sqlite3.Connection) -> int:
    """Merge alias slugs, then drop duplicate (outlet, URL) rows. Returns rows removed."""
    relax_review_uniqueness(con)
    catalog = _catalog_slugs()
    _merge_aliased_outlets(con)
    _merge_same_publication(con, catalog)
    con.row_factory = sqlite3.Row
    grouped: dict[int, list[sqlite3.Row]] = defaultdict(list)
    for row in con.execute(
        """SELECT r.id, r.game_id, o.slug, r.review_url, r.raw_score, r.scale_max,
                  r.raw_score_norm, r.excerpt, r.platforms, r.reviewed_on, r.reviewer
           FROM reviews r JOIN outlets o ON o.id = r.outlet_id
           ORDER BY r.game_id, r.id"""
    ):
        grouped[row["game_id"]].append(row)
    removed = 0
    for rows in grouped.values():
        updates, delete_ids = _collapse_rows(rows)
        if not delete_ids:
            continue
        for item in updates:
            if item["id"] in delete_ids:
                continue
            con.execute(
                """UPDATE reviews
                   SET raw_score=?, scale_max=?, raw_score_norm=?, excerpt=?,
                       review_url=?, platforms=?, reviewed_on=?, reviewer=?
                   WHERE id=?""",
                (
                    item["raw_score"],
                    item["scale_max"],
                    item["raw_score_norm"],
                    item["excerpt"],
                    item["review_url"],
                    item["platforms"],
                    item["reviewed_on"],
                    item["reviewer"],
                    item["id"],
                ),
            )
        con.executemany("DELETE FROM reviews WHERE id=?", [(rid,) for rid in delete_ids])
        removed += len(delete_ids)
    con.execute("DELETE FROM outlets WHERE id NOT IN (SELECT DISTINCT outlet_id FROM reviews)")
    con.commit()
    print(f"deduped reviews removed={removed}")
    return removed


def resolve_outlet(con: sqlite3.Connection, slug: str, name: str) -> tuple[int, str]:
    """Reuse the canonical outlet so a compact slug cannot open a second row."""
    canon = canonicalize_outlet_slug(slug or "")
    row = con.execute("SELECT id, slug FROM outlets WHERE slug=?", (canon,)).fetchone()
    if row:
        return row[0], row[1]
    folded = fold_name(name) or fold_name(canon)
    if folded:
        for oid, oslug, oname in con.execute("SELECT id, slug, name FROM outlets"):
            if fold_name(oname) == folded:
                return oid, oslug
    con.execute(
        """INSERT INTO outlets (slug, name, review_count, mu, sigma, mu_prime, sigma_prime)
           VALUES (?, ?, 0, 0, 0, 0, 0)""",
        (canon, name or canon),
    )
    oid = con.execute("SELECT id FROM outlets WHERE slug=?", (canon,)).fetchone()[0]
    return oid, canon


def upsert_review(con: sqlite3.Connection, game_slug: str, review: dict) -> None:
    """Insert a critic row, or fold it into the existing (outlet, URL) row."""
    outlet_id, outlet_slug = resolve_outlet(
        con, review.get("outlet_slug") or "", review.get("outlet_name") or review.get("outlet_slug") or ""
    )
    game = con.execute("SELECT id FROM games WHERE slug=?", (game_slug,)).fetchone()
    if game is None:
        return
    game_id = game[0]
    url = review.get("url") or review.get("review_url")
    reviewer = review.get("author") or review.get("reviewer")
    reviewed_on = normalize_date(review.get("date") or review.get("reviewed_on"))
    score = review.get("score")
    if score is None:
        score = review.get("raw_score_norm")
    key = dedupe_key(outlet_slug, url, score, reviewer)
    con.row_factory = sqlite3.Row
    existing = con.execute(
        """SELECT id, review_url, raw_score, scale_max, raw_score_norm, excerpt, platforms,
                  reviewed_on, reviewer
           FROM reviews WHERE outlet_id=? AND game_id=? ORDER BY id""",
        (outlet_id, game_id),
    ).fetchall()
    match = None
    for row in existing:
        if dedupe_key(outlet_slug, row["review_url"], row["raw_score_norm"], row["reviewer"]) == key:
            match = row
            break
    incoming = {
        "url": url,
        "score": score,
        "date": reviewed_on,
        "author": reviewer,
        "excerpt": review.get("excerpt"),
        "raw_score": review.get("raw_score"),
        "scale_max": review.get("scale_max"),
    }
    if match is None:
        con.execute(
            """INSERT INTO reviews (
                   outlet_id, game_id, raw_score, scale_max, raw_score_norm, z_score,
                   excerpt, review_url, reviewed_on, reviewer
               ) VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, ?)""",
            (
                outlet_id,
                game_id,
                incoming["raw_score"],
                incoming["scale_max"],
                0.0 if score is None else float(score),
                incoming["excerpt"] or None,
                url,
                reviewed_on,
                reviewer,
            ),
        )
        return
    incumbent = {
        "url": match["review_url"],
        "score": match["raw_score_norm"],
        "date": match["reviewed_on"],
        "author": match["reviewer"],
        "excerpt": match["excerpt"],
        "raw_score": match["raw_score"],
        "scale_max": match["scale_max"],
    }
    chosen = prefer_review(incumbent, incoming)
    con.execute(
        """UPDATE reviews
           SET raw_score=?, scale_max=?, raw_score_norm=?, excerpt=?,
               review_url=?, reviewed_on=?, reviewer=?
           WHERE id=?""",
        (
            chosen.get("raw_score"),
            chosen.get("scale_max"),
            0.0 if chosen.get("score") is None else float(chosen["score"]),
            chosen.get("excerpt") or None,
            chosen.get("url"),
            normalize_date(chosen.get("date")),
            chosen.get("author"),
            match["id"],
        ),
    )


def duplicate_pairs(con: sqlite3.Connection) -> list[tuple[str, str, str]]:
    """Games that still list one outlet and one normalized URL twice."""
    seen: dict[tuple[str, str, str], int] = {}
    dupes = []
    for game, slug, url in con.execute(
        """SELECT g.slug, o.slug, r.review_url
           FROM reviews r
           JOIN games g ON g.id = r.game_id
           JOIN outlets o ON o.id = r.outlet_id"""
    ):
        norm = normalize_review_url(url)
        if not norm:
            continue
        key = (game, canonicalize_outlet_slug(slug), norm)
        seen[key] = seen.get(key, 0) + 1
    for key, count in seen.items():
        if count > 1:
            dupes.append(key)
    return dupes


def self_check() -> None:
    same = "https://jeuxvideo.com/test/829186/god-of-war.htm"
    assert (
        normalize_review_url("https://www.Jeuxvideo.com:443/test/829186/god-of-war.htm?utm=1#frag")
        == same
    )
    assert normalize_review_url("https://gameover.gr/reviews/1/") == "https://gameover.gr/reviews/1"
    assert normalize_review_url("http://gamer.no/artikler/x/") == "http://gamer.no/artikler/x"
    assert normalize_review_url("") == ""
    assert dedupe_key("gameovergr", "https://gameover.gr/reviews/1", 100, None) == dedupe_key(
        "gameover-gr", "https://gameover.gr/reviews/1/", 90, "Ann"
    )
    assert dedupe_key("ign", "https://ign.com/ps5-review", 90, None) != dedupe_key(
        "ign", "https://ign.com/pc-review", 90, None
    )
    assert dedupe_key("gamestm", None, 90, None) == dedupe_key("games-tm", "", 90.0, None)
    assert dedupe_key("games-tm", None, 90, "Ann") != dedupe_key("games-tm", None, 90, "Bob")
    rich = prefer_review(
        {"url": "", "score": 90, "date": None, "excerpt": "short"},
        {"url": "https://example.com/review", "score": 90, "date": "2026-11-01", "excerpt": ""},
    )
    assert rich["url"] == "https://example.com/review"
    assert rich["date"] == "2026-11-01"
    assert rich["excerpt"] == "short"
    print("dedupe self-check ok")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=ROOT / "sourcecritic.db")
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    if args.self_check:
        self_check()
        return
    con = sqlite3.connect(args.db)
    try:
        dedupe_database(con)
        leftover = duplicate_pairs(con)
        if leftover:
            sample = leftover[0]
            raise SystemExit(
                f"{len(leftover)} duplicate (outlet, URL) pairs remain, e.g. {sample[0]} {sample[1]} {sample[2]}"
            )
        for slug in ("god-of-war", "gears-of-war-e-day", "total-war-warhammer-iii"):
            count = con.execute(
                """SELECT COUNT(*) FROM reviews r
                   JOIN games g ON g.id = r.game_id WHERE g.slug=?""",
                (slug,),
            ).fetchone()[0]
            print(f"{slug} reviews={count}")
    finally:
        con.close()


if __name__ == "__main__":
    main()
