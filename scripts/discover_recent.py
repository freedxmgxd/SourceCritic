#!/usr/bin/env python3
"""Pull the IGDB release window and attach outlet reviews.

Every run:
  * games released in the last 45 days, plus releases in the next 30 days
  * critic reviews from outlet RSS feeds and schema.org JSON-LD
  * a game page only when at least one critic review matched

Edition suffixes (Deluxe, Remastered, …) are stripped only when IGDB marks
that name as a version of the same game. A port or remaster IGDB lists on
its own keeps its own page and the year of its own first release.

OpenCritic and Metacritic are not requested.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sqlite3
import sys
import unicodedata
import urllib.parse
import urllib.request
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from adapters import is_hub_url, repair_mojibake  # noqa: E402
from dedupe_reviews import dedupe_database, dedupe_key, prefer_review, relax_review_uniqueness, upsert_review  # noqa: E402
from collect import (  # noqa: E402
    KNOWN_FEEDS,
    decode_response,
    excerpt_of,
    fetch,
    jsonld_reviews,
    norm_title,
    rating_to_100,
)

DATA = ROOT / "data"
OUTLETS_PATH = DATA / "outlets.json"
CATALOG_PATH = DATA / "catalog_games.json"
SEED_PATH = DATA / "seed_reviews.json"
DB_PATH = ROOT / "sourcecritic.db"

# Reviews that have already left the short RSS window. Each URL is an outlet
# article; the score is read from its JSON-LD on every build.
SUPPLEMENTAL_URLS = [
    "https://www.pcgamer.com/games/sports/football-manager-27-review/",
    "https://www.destructoid.com/reviews/football-manager-27-review-high-potential-ability-stuck-with-silver-stars/",
    "https://www.cubed3.com/games/reviews/pc/football-manager-2027",
    "https://www.thegamer.com/football-manager-27-review/",
    "https://www.pcgamer.com/games/roguelike/warhammer-survivors-review/",
    "https://www.shacknews.com/article/151331/warhammer-survivors-review-score",
    "https://www.dualshockers.com/warhammer-survivors-review-a-brutal-new-bullet-heaven-game/",
    "https://www.thesixthaxis.com/2026/11/11/warhammer-survivors-review/",
    "https://www.thegamer.com/warhammer-survivors-review/",
    "https://www.pcgamer.com/games/world-of-warcraft/world-of-warcraft-forever-review/",
    "https://www.gamesradar.com/games/world-of-warcraft/world-of-warcraft-forever-review/",
    "https://www.noobfeed.com/reviews/world-of-warcraft-forever-review",
]

# Used only when IGDB cannot be queried. Dates are the edition's first release.
FALLBACK_GAMES = [
    {
        "title": "Football Manager 27",
        "released_on": "2026-11-10",
        "platforms": ["pc", "ps5", "xbox-series"],
        "aliases": ["football manager 2027"],
    },
    {
        "title": "Warhammer Survivors",
        "released_on": "2026-11-10",
        "platforms": ["pc", "ps5", "xbox-series", "switch"],
        "aliases": [],
    },
    {
        "title": "World of Warcraft: Forever",
        "released_on": "2026-11-04",
        "platforms": ["pc"],
        "aliases": ["wow forever"],
    },
]

WATCH = [game["title"] for game in FALLBACK_GAMES]

EDITION_SUFFIX = re.compile(
    r"\b(?:deluxe|remastered|remaster|definitive edition|goty|game of the year|"
    r"complete edition|gold edition|ultimate edition|enhanced edition|anniversary edition)\b.*$",
    re.I,
)

# UTF-8 bytes that were decoded as Windows-1252. The build rejects these.
MOJIBAKE_SEQUENCES = ("Ã©", "Ã¡", "Ã­", "Ã³", "Ãº", "Ã±", "Ã£", "Ã§", "â€™", "â€œ", "â€“", "â€”")


def today_utc() -> date:
    return datetime.now(timezone.utc).date()


def slugify(title: str) -> str:
    text = unicodedata.normalize("NFKD", title)
    text = "".join(ch for ch in text if not unicodedata.combining(ch))
    text = text.replace("'", "").replace("’", "").replace("ʼ", "")
    text = re.sub(r"[^a-zA-Z0-9]+", "-", text.lower()).strip("-")
    return text or "game"


def collapse_words(value: str) -> str:
    words = norm_title(value).split()
    kept: list[str] = []
    for word in words:
        if not kept or kept[-1] != word:
            kept.append(word)
    return " ".join(kept)


def mentions(text: str, title: str, aliases: list[str] | None = None) -> bool:
    """True when text is the title, or the title with a publisher in front of it."""
    hay = collapse_words(text)
    names = [title, *(aliases or [])]
    for name in names:
        needle = collapse_words(name)
        if not hay or not needle:
            continue
        if hay == needle or hay.endswith(" " + needle):
            return True
    return False


def slug_hit(url: str, game: dict) -> bool:
    """The article path contains this game's slug, not a longer edition slug."""
    path = urllib.parse.urlparse(url).path.lower()
    slugs = [slugify(game["title"])]
    slugs.extend(slugify(alias) for alias in game.get("aliases") or [])
    for slug in slugs:
        if slug and re.search(rf"(^|/){re.escape(slug)}(/|$|-review)", path):
            return True
    return False


def edition_key(title: str) -> str:
    stripped = EDITION_SUFFIX.sub("", title)
    return collapse_words(stripped)


def same_game(review_title: str, game: dict, versions: dict[str, str]) -> bool:
    """Match a review title. Strip Deluxe/Remastered only when IGDB says so."""
    if mentions(review_title, game["title"], game.get("aliases")):
        return True
    review_norm = collapse_words(review_title)
    parent = versions.get(review_norm)
    if parent and parent == collapse_words(game["title"]):
        stripped = edition_key(review_title)
        if stripped and stripped != review_norm and mentions(stripped, game["title"], game.get("aliases")):
            return True
    return False


def igdb_token() -> str | None:
    client_id = os.environ.get("TWITCH_CLIENT_ID")
    secret = os.environ.get("TWITCH_CLIENT_SECRET")
    if not client_id or not secret:
        return None
    body = urllib.parse.urlencode(
        {"client_id": client_id, "client_secret": secret, "grant_type": "client_credentials"}
    ).encode()
    request = urllib.request.Request("https://id.twitch.tv/oauth2/token", data=body)
    with urllib.request.urlopen(request, timeout=30) as response:
        payload = json.loads(response.read().decode("utf-8"))
    return payload.get("access_token")


def fetch_igdb_window(start: date, end: date) -> tuple[list[dict], dict[str, str], str | None]:
    """Return (games, version->parent title, error). error is set when IGDB was not queried."""
    token = igdb_token()
    client_id = os.environ.get("TWITCH_CLIENT_ID")
    if not token or not client_id:
        return [], {}, "IGDB credentials are not set"
    start_unix = int(datetime(start.year, start.month, start.day, tzinfo=timezone.utc).timestamp())
    end_unix = int(datetime(end.year, end.month, end.day, tzinfo=timezone.utc).timestamp())
    query = (
        "fields name,first_release_date,version_parent.name,version_parent.first_release_date,"
        "category,platforms.name,summary,slug;"
        f" where first_release_date >= {start_unix} & first_release_date < {end_unix}"
        " & category = (0,8,9,10,11);"
        " limit 500;"
    )
    request = urllib.request.Request(
        "https://api.igdb.com/v4/games",
        data=query.encode(),
        headers={
            "Client-ID": client_id,
            "Authorization": f"Bearer {token}",
            "Accept": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=40) as response:
            rows = json.loads(response.read().decode("utf-8"))
    except Exception as error:  # noqa: BLE001 - the build must log the miss and continue
        return [], {}, f"IGDB request failed ({error})"
    games = []
    versions: dict[str, str] = {}
    for row in rows:
        if not isinstance(row, dict) or not row.get("name"):
            continue
        parent = row.get("version_parent") if isinstance(row.get("version_parent"), dict) else None
        if parent and parent.get("name"):
            # IGDB says this name is the same game. Do not give the edition its own page.
            versions[collapse_words(row["name"])] = collapse_words(parent["name"])
            released = parent.get("first_release_date") or row.get("first_release_date")
            title = parent["name"]
        else:
            released = row.get("first_release_date")
            title = row["name"]
        released_on = None
        if isinstance(released, (int, float)):
            released_on = datetime.fromtimestamp(int(released), timezone.utc).date().isoformat()
        platforms = []
        for item in row.get("platforms") or []:
            if isinstance(item, dict) and item.get("name"):
                platforms.append(str(item["name"]))
        games.append(
            {
                "title": title,
                "released_on": released_on,
                "platforms": platforms,
                "summary": row.get("summary"),
                "aliases": [],
                "igdb_id": row.get("id"),
                "category": row.get("category"),
            }
        )
    # One page per title. A version folded above shares the parent's row.
    deduped: dict[str, dict] = {}
    for game in games:
        deduped.setdefault(collapse_words(game["title"]), game)
    return list(deduped.values()), versions, None


def author_name(node: dict) -> str | None:
    author = node.get("author")
    if isinstance(author, list):
        author = author[0] if author else None
    if isinstance(author, dict):
        name = author.get("name")
        return str(name) if name else None
    if isinstance(author, str):
        return author
    return None


def reviewed_title(node: dict, html: str, url: str) -> str:
    item = node.get("itemReviewed") if isinstance(node.get("itemReviewed"), dict) else {}
    name = item.get("name") or ""
    if name and "review" not in str(name).lower():
        return str(name)
    title_match = re.search(r"<title>(.*?)</title>", html, flags=re.I | re.S)
    headline = re.sub(r"\s+", " ", title_match.group(1)).strip() if title_match else ""
    headline = re.split(r"\s+[|–—-]\s+", headline)[0]
    headline = re.sub(r"\breview\b.*$", "", headline, flags=re.I).strip(" :-")
    return headline or str(name) or url


def review_from_page(url: str, outlet: dict) -> dict | None:
    if "review-in-progress" in url.lower():
        return None
    try:
        html = decode_response(fetch(url))
    except Exception as error:  # noqa: BLE001
        print(f"  page skip {url}: {error}")
        return None
    html = repair_mojibake(html) or html
    scale = float(outlet.get("scale_max") or 100)
    for node in jsonld_reviews(html):
        title = reviewed_title(node, html, url)
        if "in progress" in title.lower():
            continue
        score = rating_to_100(node, scale)
        if score is None or not title:
            continue
        rating = node.get("reviewRating") if isinstance(node.get("reviewRating"), dict) else {}
        try:
            raw = float(rating.get("ratingValue"))
            best = float(rating.get("bestRating") or scale)
        except (TypeError, ValueError):
            continue
        return {
            "title": repair_mojibake(title) or title,
            "score": round(score, 2),
            "raw_score": raw,
            "scale_max": best,
            "excerpt": repair_mojibake(excerpt_of(str(node.get("reviewBody") or node.get("description") or ""))) or "",
            "author": author_name(node),
            "date": node.get("datePublished"),
            "url": url.split("#", 1)[0],
            "outlet_slug": outlet.get("slug"),
            "outlet_name": outlet.get("name"),
        }
    return None


def outlet_by_host(outlets: list[dict]) -> dict[str, dict]:
    index = {}
    for outlet in outlets:
        domain = (outlet.get("domain") or "").lower()
        if domain:
            index[domain] = outlet
    for slug, known in KNOWN_FEEDS.items():
        index.setdefault(known["domain"], {"slug": slug, "name": slug, "domain": known["domain"], "scale_max": float(known["scale_max"])})
    return index


def outlet_for(url: str, index: dict[str, dict]) -> dict | None:
    host = (urllib.parse.urlparse(url).hostname or "").lower().removeprefix("www.")
    if host in index:
        return index[host]
    parts = host.split(".")
    while len(parts) > 2:
        parts = parts[1:]
        candidate = ".".join(parts)
        if candidate in index:
            return index[candidate]
    return None


def rss_review_links(games: list[dict]) -> list[str]:
    """Review links whose feed title or URL already names a game in the window."""
    import xml.etree.ElementTree as ET

    links = []
    for slug, known in KNOWN_FEEDS.items():
        feed = known.get("feed")
        if not feed:
            continue
        try:
            root = ET.fromstring(fetch(feed))
        except Exception as error:  # noqa: BLE001
            print(f"  feed skip {slug}: {error}")
            continue
        current_title = ""
        for item in root.iter():
            tag = item.tag.rsplit("}", 1)[-1]
            if tag == "title":
                current_title = (item.text or "").strip()
            elif tag == "link":
                href = (item.attrib.get("href") or item.text or "").strip()
                if not href.startswith("http"):
                    continue
                href = href.split("#", 1)[0]
                if not re.search(r"review", href, re.I) and not re.search(r"review", current_title, re.I):
                    continue
                if any(slug_hit(href, game) or mentions(current_title, game["title"], game.get("aliases")) for game in games):
                    links.append(href)
    return links


def attach_reviews(games: list[dict], versions: dict[str, str], reviews: list[dict]) -> dict[str, dict]:
    grouped: dict[str, dict] = {}
    for game in games:
        key = collapse_words(game["title"])
        grouped[key] = {**game, "reviews": []}
    for review in reviews:
        if is_hub_url(review.get("url")):
            review["url"] = None
        if not review.get("url"):
            continue
        matched = None
        for key, game in grouped.items():
            if same_game(review.get("title") or "", game, versions) or slug_hit(review.get("url") or "", game):
                matched = key
                break
        if matched is None:
            continue
        bucket = grouped[matched]["reviews"]
        # Same outlet and the same URL is one review. A different URL is a second one.
        key = dedupe_key(review.get("outlet_slug") or "", review.get("url"), review.get("score"), review.get("author"))
        match = next(
            (
                index
                for index, item in enumerate(bucket)
                if dedupe_key(item.get("outlet_slug") or "", item.get("url"), item.get("score"), item.get("author")) == key
            ),
            None,
        )
        if match is None:
            bucket.append(review)
        else:
            bucket[match] = prefer_review(bucket[match], review)
    return grouped


def recent_enough(released_on: str | None, today: date, days: int = 30) -> bool:
    if not released_on:
        return False
    try:
        released = date.fromisoformat(released_on[:10])
    except ValueError:
        return False
    age = (today - released).days
    return 0 <= age <= days


def earns_page(game: dict, today: date) -> bool:
    reviews = game["reviews"]
    if not reviews:
        return False
    released_on = game.get("released_on")
    upcoming = False
    if released_on:
        try:
            upcoming = date.fromisoformat(released_on[:10]) > today
        except ValueError:
            upcoming = False
    # Last 30 days, and anything not yet released, need 3 critic reviews.
    if (recent_enough(released_on, today) or upcoming) and len(reviews) < 3:
        return False
    return True


def report(grouped: dict[str, dict], igdb_error: str | None, today: date, watched_missing: list[str]) -> list[dict]:
    """Print a line for each watched title and return games that earn a page."""
    publish = []
    for game in grouped.values():
        if earns_page(game, today):
            publish.append(game)
        elif game["title"] not in WATCH and game["reviews"] and recent_enough(game.get("released_on"), today):
            print(
                f"skipped: {game['title']} - matched {len(game['reviews'])} critic reviews, "
                "below the 3-review minimum"
            )
    present = {game["title"] for game in grouped.values()}
    for title in WATCH:
        game = next((item for item in grouped.values() if item["title"] == title), None)
        if title in watched_missing or game is None:
            if igdb_error:
                print(f"skipped: {title} - never discovered ({igdb_error})")
            else:
                print(f"skipped: {title} - never discovered (not in the IGDB release window)")
            continue
        count = len(game["reviews"])
        if earns_page(game, today):
            note = f"{count} critic reviews"
            if igdb_error:
                note += f"; IGDB window was not pulled ({igdb_error}); release date {game.get('released_on')}"
            print(f"included: {title} - {note}")
        elif count == 0:
            print(f"skipped: {title} - discovered but its reviews were not matched")
        else:
            print(f"skipped: {title} - matched {count} critic reviews, below the 3-review minimum")
    del present
    return publish


def ensure_column(con: sqlite3.Connection, table: str, column: str, decl: str) -> None:
    have = {row[1] for row in con.execute(f"PRAGMA table_info({table})")}
    if column not in have:
        con.execute(f"ALTER TABLE {table} ADD COLUMN {column} {decl}")


def repair_database(con: sqlite3.Connection) -> None:
    ensure_column(con, "games", "released_on", "TEXT")
    for table, columns in (("games", ("title", "synopsis")), ("reviews", ("excerpt",)), ("outlets", ("name",))):
        rows = con.execute(f"SELECT id, {', '.join(columns)} FROM {table}").fetchall()
        for row in rows:
            changed = False
            values = []
            for value in row[1:]:
                fixed = repair_mojibake(value) if isinstance(value, str) else value
                if fixed != value:
                    changed = True
                values.append(fixed)
            if changed:
                assignments = ", ".join(f"{column}=?" for column in columns)
                con.execute(f"UPDATE {table} SET {assignments} WHERE id=?", (*values, row[0]))
    cleared = 0
    for rid, url in con.execute("SELECT id, review_url FROM reviews WHERE review_url IS NOT NULL"):
        if is_hub_url(url):
            con.execute("UPDATE reviews SET review_url=NULL WHERE id=?", (rid,))
            cleared += 1
    # Re-release under the same name: the edition's first release, not the year-list year.
    con.execute(
        """UPDATE games
           SET release_year=?, released_on=?, platform_years=?
           WHERE slug=?""",
        (
            2021,
            "2021-06-10",
            json.dumps({"ps5": 2021, "pc": 2021, "switch-2": 2026}),
            "final-fantasy-vii-remake-intergrade",
        ),
    )
    con.commit()
    if cleared:
        print(f"cleared {cleared} homepage or hub review URLs")


def repair_seed_text(path: Path) -> None:
    if not path.exists():
        return
    text = path.read_text(encoding="utf-8")
    original = text

    def repl_url(match: re.Match[str]) -> str:
        url = match.group(1)
        if is_hub_url(url):
            return '"url": null'
        return match.group(0)

    text = re.sub(r'"url": "(https?://[^"]+)"', repl_url, text)
    fixed = repair_mojibake(text)
    if fixed:
        text = fixed
    start = text.find('  "final-fantasy-vii-remake-intergrade":')
    if start != -1:
        nxt = text.find('\n  "', start + 10)
        window_end = nxt if nxt != -1 else len(text)
        year_at = text.find('"release_year": 2026', start, window_end)
        if year_at != -1:
            text = text[:year_at] + '"release_year": 2021' + text[year_at + len('"release_year": 2026') :]
    if text != original:
        path.write_text(text, encoding="utf-8")
        print(f"repaired {path.name}")


def store_games(con: sqlite3.Connection, games: list[dict]) -> None:
    relax_review_uniqueness(con)
    for game in games:
        slug = slugify(game["title"])
        year = None
        if game.get("released_on"):
            year = int(game["released_on"][:4])
        platforms = json.dumps(game.get("platforms") or [])
        synopsis = game.get("summary")
        con.execute(
            """INSERT INTO games (slug, title, release_year, platforms, synopsis, review_count, released_on)
               VALUES (?, ?, ?, ?, ?, 0, ?)
               ON CONFLICT(slug) DO UPDATE SET
                 title=excluded.title,
                 release_year=COALESCE(excluded.release_year, games.release_year),
                 platforms=COALESCE(excluded.platforms, games.platforms),
                 synopsis=COALESCE(excluded.synopsis, games.synopsis),
                 released_on=COALESCE(excluded.released_on, games.released_on)""",
            (slug, game["title"], year, platforms, synopsis, game.get("released_on")),
        )
        for review in game["reviews"]:
            upsert_review(con, slug, review)
    con.commit()


def remember_catalog(games: list[dict]) -> None:
    if not CATALOG_PATH.exists():
        return
    catalog = json.loads(CATALOG_PATH.read_text(encoding="utf-8"))
    rows = catalog.setdefault("games", {})
    changed = False
    for game in games:
        slug = slugify(game["title"])
        if slug in rows:
            continue
        rows[slug] = {
            "slug": slug,
            "title": game["title"],
            "aliases": game.get("aliases") or [],
            "external": {},
            "discovered_via": ["igdb-window"],
            "status": "active",
            "release_year": int(game["released_on"][:4]) if game.get("released_on") else None,
        }
        changed = True
    if changed:
        catalog["updated_at"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        CATALOG_PATH.write_text(
            json.dumps(catalog, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )


def collect_reviews(outlets: list[dict], games: list[dict]) -> list[dict]:
    index = outlet_by_host(outlets)
    urls = []
    seen = set()
    for url in rss_review_links(games) + SUPPLEMENTAL_URLS:
        if url in seen or is_hub_url(url):
            continue
        seen.add(url)
        urls.append(url)
    reviews = []
    for url in urls:
        outlet = outlet_for(url, index)
        if outlet is None:
            host = (urllib.parse.urlparse(url).hostname or "").lower().removeprefix("www.")
            outlet = {"slug": slugify(host), "name": host, "scale_max": 100, "domain": host}
        found = review_from_page(url, outlet)
        if found:
            reviews.append(found)
    return reviews


def run(db_path: Path) -> None:
    today = today_utc()
    start = today - timedelta(days=45)
    end = today + timedelta(days=30)
    print(f"discovery window {start.isoformat()} .. {end.isoformat()}")
    games, versions, igdb_error = fetch_igdb_window(start, end)
    watched_missing: list[str] = []
    if igdb_error:
        print(f"IGDB: {igdb_error}; using the known recent-release list")
        games = [dict(game) for game in FALLBACK_GAMES]
    else:
        print(f"IGDB: {len(games)} games in the window")
        have = {collapse_words(game["title"]) for game in games}
        for fallback in FALLBACK_GAMES:
            if collapse_words(fallback["title"]) not in have:
                watched_missing.append(fallback["title"])
    outlets = []
    if OUTLETS_PATH.exists():
        outlets = json.loads(OUTLETS_PATH.read_text(encoding="utf-8")).get("outlets") or []
    reviews = collect_reviews(outlets, games)
    print(f"read {len(reviews)} scored critic reviews from outlet pages")
    grouped = attach_reviews(games, versions, reviews)
    publish = report(grouped, igdb_error, today, watched_missing)
    # Never store a game that has no critic reviews.
    publish = [game for game in publish if game["reviews"]]
    con = sqlite3.connect(db_path)
    try:
        dedupe_database(con)
        repair_database(con)
        store_games(con, publish)
        dedupe_database(con)
    finally:
        con.close()
    repair_seed_text(SEED_PATH)
    remember_catalog(publish)
    leftover = []
    if db_path.exists():
        con = sqlite3.connect(db_path)
        try:
            for (excerpt,) in con.execute("SELECT excerpt FROM reviews WHERE excerpt IS NOT NULL"):
                if any(marker in excerpt for marker in MOJIBAKE_SEQUENCES):
                    leftover.append(excerpt[:80])
                    break
        finally:
            con.close()
    if leftover:
        raise SystemExit(f"mojibake remains in the database: {leftover[0]}")


def self_check() -> None:
    assert repair_mojibake("not clichÃ©d") == "not clichéd"
    assert repair_mojibake("clichï¿½") == "cliché"
    assert repair_mojibake("already fine") == "already fine"
    assert is_hub_url("https://easyallies.com")
    assert is_hub_url("https://easyallies.com/")
    assert is_hub_url("https://www.ign.com/xbox")
    assert is_hub_url("https://www.ign.com/playstation")
    assert not is_hub_url("https://www.ign.com/articles/bioshock-review")
    assert not is_hub_url("https://ztgd.com/the-legend-of-zelda-breath-of-the-wild/review")
    assert is_hub_url("https://www.cubed3.com/games/reviews")
    assert not is_hub_url("https://gamefocus.ca?nav=reviewCard&fid=9463")
    game = {"title": "Hades II", "aliases": []}
    versions = {collapse_words("Hades II Deluxe"): collapse_words("Hades II")}
    assert same_game("Hades II Deluxe", game, versions)
    assert not same_game("Hades II Remastered", game, {})
    assert mentions("Sega Football Manager 27", "Football Manager 27", ["football manager 2027"])
    fm = {"title": "Football Manager 27", "aliases": ["football manager 2027"]}
    assert slug_hit("https://www.cubed3.com/games/reviews/pc/football-manager-2027", fm)
    assert slug_hit("https://www.pcgamer.com/games/sports/football-manager-27-review/", fm)
    assert not slug_hit("https://example.com/hades-ii-deluxe-review", {"title": "Hades II", "aliases": []})
    assert recent_enough("2026-11-10", date(2026, 11, 26))
    assert not recent_enough("2026-10-01", date(2026, 11, 26))
    print("discover self-check ok")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, default=DB_PATH)
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    if args.self_check:
        self_check()
        return
    run(args.db)


if __name__ == "__main__":
    main()
