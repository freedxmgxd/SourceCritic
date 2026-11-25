#!/usr/bin/env python3
"""Collect game metadata and critic reviews without OpenCritic or Metacritic.

Metadata: IGDB (Twitch) when TWITCH_CLIENT_ID and TWITCH_CLIENT_SECRET are set,
otherwise RAWG when RAWG_API_KEY is set. Covers and summaries come from those
APIs only.

Reviews: each outlet in data/outlets.json, via its RSS/Atom feed or sitemap,
then the schema.org Review JSON-LD on the article. robots.txt is respected.
One request every few seconds per domain, with an on-disk cache.

Stored fields: score, an excerpt of at most two sentences, date, author, and
the canonical URL. Ambiguous title matches go to data/review_queue.json.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import time
import unicodedata
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data"
OUTLETS_PATH = DATA / "outlets.json"
CATALOG_OUTLETS = DATA / "catalog_outlets.json"
CATALOG_GAMES = DATA / "catalog_games.json"
CACHE = DATA / "http_cache"
QUEUE_PATH = DATA / "review_queue.json"
COLLECTED_PATH = DATA / "collected_reviews.json"

USER_AGENT = "SourceCriticBot/1.0 (+https://sourcecritic.freedxm.com/about/)"
MIN_INTERVAL_SECONDS = 3.0
BLOCKED_SUFFIXES = ("opencritic.com", "metacritic.com")

# Feeds we know. The rest of outlets.json is the catalog, with feed left null.
KNOWN_FEEDS: dict[str, dict[str, str]] = {
    "ign": {"domain": "ign.com", "homepage": "https://www.ign.com/", "feed": "https://feeds.feedburner.com/ign/games-reviews", "scale_max": "10", "language": "en"},
    "pc-gamer": {"domain": "pcgamer.com", "homepage": "https://www.pcgamer.com/", "feed": "https://www.pcgamer.com/rss/", "scale_max": "100", "language": "en"},
    "eurogamer": {"domain": "eurogamer.net", "homepage": "https://www.eurogamer.net/", "feed": "https://www.eurogamer.net/feed", "scale_max": "5", "language": "en"},
    "gamespot": {"domain": "gamespot.com", "homepage": "https://www.gamespot.com/", "feed": "https://www.gamespot.com/feeds/reviews/", "scale_max": "10", "language": "en"},
    "gamesradar": {"domain": "gamesradar.com", "homepage": "https://www.gamesradar.com/", "feed": "https://www.gamesradar.com/rss/", "scale_max": "5", "language": "en"},
    "rock-paper-shotgun": {"domain": "rockpapershotgun.com", "homepage": "https://www.rockpapershotgun.com/", "feed": "https://www.rockpapershotgun.com/feed", "scale_max": "100", "language": "en"},
    "polygon": {"domain": "polygon.com", "homepage": "https://www.polygon.com/", "feed": "https://www.polygon.com/rss/index.xml", "scale_max": "10", "language": "en"},
    "destructoid": {"domain": "destructoid.com", "homepage": "https://www.destructoid.com/", "feed": "https://www.destructoid.com/feed/", "scale_max": "10", "language": "en"},
    "vg247": {"domain": "vg247.com", "homepage": "https://www.vg247.com/", "feed": "https://www.vg247.com/feed", "scale_max": "5", "language": "en"},
    "push-square": {"domain": "pushsquare.com", "homepage": "https://www.pushsquare.com/", "feed": "https://www.pushsquare.com/feeds/latest", "scale_max": "10", "language": "en"},
    "nintendo-life": {"domain": "nintendolife.com", "homepage": "https://www.nintendolife.com/", "feed": "https://www.nintendolife.com/feeds/latest", "scale_max": "10", "language": "en"},
    "kotaku": {"domain": "kotaku.com", "homepage": "https://kotaku.com/", "feed": "https://kotaku.com/rss", "scale_max": "10", "language": "en"},
}

_last_hit: dict[str, float] = {}


def assert_allowed(url: str) -> None:
    host = (urllib.parse.urlparse(url).hostname or "").lower()
    if any(host == suffix or host.endswith("." + suffix) for suffix in BLOCKED_SUFFIXES):
        raise RuntimeError(f"refusing {host}: OpenCritic and Metacritic are not collection sources")


def norm_title(value: str) -> str:
    text = unicodedata.normalize("NFD", value)
    text = "".join(ch for ch in text if unicodedata.category(ch) != "Mn")
    text = text.replace("'", "").replace("’", "").lower()
    text = re.sub(r"[^a-z0-9]+", " ", text)
    return re.sub(r"\s+", " ", text).strip()


def excerpt_of(text: str) -> str:
    cleaned = re.sub(r"\s+", " ", text).strip()
    parts = re.split(r"(?<=[.!?])\s+", cleaned)
    short = " ".join(parts[:2]).strip()
    return short[:400]


def cache_path(url: str) -> Path:
    digest = hashlib.sha256(url.encode()).hexdigest()
    return CACHE / digest[:2] / digest


def fetch(url: str) -> bytes:
    assert_allowed(url)
    path = cache_path(url)
    if path.exists():
        return path.read_bytes()
    host = urllib.parse.urlparse(url).hostname or ""
    elapsed = time.time() - _last_hit.get(host, 0.0)
    if elapsed < MIN_INTERVAL_SECONDS:
        time.sleep(MIN_INTERVAL_SECONDS - elapsed)
    if not robots_allows(url):
        raise RuntimeError(f"robots.txt disallows {url}")
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    with urllib.request.urlopen(request, timeout=30) as response:
        body = response.read()
    _last_hit[host] = time.time()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body)
    return body


def robots_allows(url: str) -> bool:
    parsed = urllib.parse.urlparse(url)
    robots_url = f"{parsed.scheme}://{parsed.netloc}/robots.txt"
    assert_allowed(robots_url)
    path = cache_path(robots_url)
    if not path.exists():
        request = urllib.request.Request(robots_url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(request, timeout=20) as response:
                body = response.read()
        except Exception:
            return True
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(body)
        _last_hit[parsed.hostname or ""] = time.time()
    rules = path.read_text(encoding="utf-8", errors="replace").splitlines()
    agent = None
    disallows: list[str] = []
    for line in rules:
        line = line.split("#", 1)[0].strip()
        if not line or ":" not in line:
            continue
        key, value = [part.strip() for part in line.split(":", 1)]
        if key.lower() == "user-agent":
            agent = value
        elif key.lower() == "disallow" and agent in {"*", "SourceCriticBot", "SourceCriticBot/1.0"}:
            disallows.append(value)
    target = parsed.path or "/"
    return not any(prefix and target.startswith(prefix) for prefix in disallows)


def domain_of(url: str | None) -> str | None:
    if not url:
        return None
    from repair_integrity import registrable

    host = urllib.parse.urlparse(url).hostname or ""
    return registrable(host) or None


def article_url(declared: str | None, fetched: str | None) -> str | None:
    """Keep the article address. A JSON-LD homepage must not replace the feed link."""

    def usable(url: str | None) -> str | None:
        if not url or not str(url).startswith("http"):
            return None
        parsed = urllib.parse.urlparse(str(url).strip())
        if parsed.path in ("", "/"):
            return None
        return str(url).strip().split("#", 1)[0]

    declared_ok = usable(declared)
    fetched_ok = usable(fetched)
    if declared_ok and fetched_ok:
        same_site = domain_of(declared_ok) == domain_of(fetched_ok)
        same_path = (
            urllib.parse.urlparse(declared_ok).path.rstrip("/")
            == urllib.parse.urlparse(fetched_ok).path.rstrip("/")
        )
        if same_site and same_path:
            return declared_ok
        return fetched_ok
    return fetched_ok or declared_ok


def build_outlets() -> None:
    catalog = json.loads(CATALOG_OUTLETS.read_text(encoding="utf-8")) if CATALOG_OUTLETS.exists() else {"outlets": {}}
    outlets = []
    for slug, entry in sorted(catalog.get("outlets", {}).items()):
        if slug in {"metacritic-users", "opencritic-recommend", "metacritic", "opencritic"}:
            continue
        known = KNOWN_FEEDS.get(slug, {})
        homepage = known.get("homepage") or entry.get("homepage")
        domain = known.get("domain") or domain_of(homepage)
        sitemap = known.get("sitemap")
        for item in entry.get("review_index_urls") or []:
            if item and "sitemap" in str(item):
                sitemap = item
                break
        outlets.append(
            {
                "slug": slug,
                "name": entry.get("name") or slug,
                "domain": domain,
                "homepage": homepage,
                "feed": known.get("feed"),
                "sitemap": sitemap,
                "scale_max": float(known.get("scale_max", "100")),
                "language": known.get("language", "en"),
            }
        )
    OUTLETS_PATH.write_text(
        json.dumps(
            {
                "user_agent": USER_AGENT,
                "min_interval_seconds": MIN_INTERVAL_SECONDS,
                "note": "Fill feed or sitemap. Placeholders stay null until a feed is known.",
                "outlets": outlets,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"wrote {len(outlets)} outlets to {OUTLETS_PATH}")


def jsonld_reviews(html: str) -> list[dict]:
    found = []
    for match in re.finditer(
        r'<script[^>]+type=["\']application/ld\+json["\'][^>]*>(.*?)</script>',
        html,
        flags=re.I | re.S,
    ):
        try:
            payload = json.loads(match.group(1).strip())
        except json.JSONDecodeError:
            continue
        nodes = payload if isinstance(payload, list) else [payload]
        for node in nodes:
            if isinstance(node, dict) and "@graph" in node:
                nodes.extend(item for item in node["@graph"] if isinstance(item, dict))
            if isinstance(node, dict) and node.get("@type") in {"Review", "CriticReview"}:
                found.append(node)
    return found


def rating_to_100(node: dict, scale_max: float) -> float | None:
    rating = node.get("reviewRating") or {}
    raw = rating.get("ratingValue")
    if raw is None:
        return None
    try:
        value = float(raw)
    except (TypeError, ValueError):
        return None
    best = rating.get("bestRating")
    try:
        ceiling = float(best) if best is not None else scale_max
    except (TypeError, ValueError):
        ceiling = scale_max
    if ceiling <= 0:
        return None
    return max(0.0, min(100.0, value / ceiling * 100.0))


def load_catalog_index() -> dict[str, list[dict]]:
    index: dict[str, list[dict]] = {}
    if not CATALOG_GAMES.exists():
        return index
    catalog = json.loads(CATALOG_GAMES.read_text(encoding="utf-8"))
    games = catalog.get("games") or catalog
    if isinstance(games, dict):
        rows = games.values()
    else:
        rows = games
    for game in rows:
        if not isinstance(game, dict) or game.get("merged_into"):
            continue
        title = game.get("title") or ""
        key = norm_title(title)
        if not key:
            continue
        index.setdefault(key, []).append(game)
    return index


def attach(index: dict[str, list[dict]], title: str, igdb_id: int | None) -> tuple[dict | None, str]:
    if igdb_id is not None:
        for games in index.values():
            hits = [game for game in games if game.get("igdb_id") == igdb_id]
            if len(hits) == 1:
                return hits[0], "igdb"
            if len(hits) > 1:
                return None, "ambiguous-igdb"
    hits = index.get(norm_title(title), [])
    if len(hits) == 1:
        return hits[0], "title"
    if len(hits) > 1:
        return None, "ambiguous-title"
    return None, "unmatched"


def collect_feed(outlet: dict, index: dict[str, list[dict]], queue: list[dict], collected: dict) -> None:
    feed = outlet.get("feed")
    if not feed:
        return
    try:
        body = fetch(feed)
    except Exception as error:
        print(f"  feed skip {outlet.get('slug')}: {error}")
        return
    try:
        root = ET.fromstring(body)
    except ET.ParseError as error:
        print(f"  feed xml {outlet.get('slug')}: {error}")
        return
    links = []
    for item in root.iter():
        tag = item.tag.rsplit("}", 1)[-1]
        if tag == "link":
            href = item.attrib.get("href") or (item.text or "")
            if href.startswith("http"):
                links.append(href.strip())
    for link in links[:40]:
        if not re.search(r"review", link, re.I):
            continue
        try:
            page = fetch(link).decode("utf-8", errors="replace")
        except Exception as error:
            print(f"  page skip {link}: {error}")
            continue
        for node in jsonld_reviews(page):
            item = node.get("itemReviewed") if isinstance(node.get("itemReviewed"), dict) else {}
            title = item.get("name") or node.get("name") or ""
            # Kotaku pages do not carry a grade. A yes/no flag must not become 100.
            unscored = outlet.get("slug") == "kotaku"
            rating = node.get("reviewRating") if isinstance(node.get("reviewRating"), dict) else {}
            scale = float(outlet.get("scale_max") or 100)
            best = rating.get("bestRating")
            try:
                if best is not None:
                    scale = float(best)
            except (TypeError, ValueError):
                pass
            raw_value = rating.get("ratingValue")
            score = None if unscored else rating_to_100(node, float(outlet.get("scale_max") or 100))
            if not title or (score is None and not unscored):
                continue
            game, reason = attach(index, str(title), None)
            record = {
                "outlet_slug": outlet.get("slug"),
                "outlet_name": outlet.get("name"),
                "title": title,
                "score": None if score is None else round(score, 2),
                "scale_max": scale,
                "excerpt": excerpt_of(str(node.get("reviewBody") or node.get("description") or "")),
                "author": (node.get("author") or {}).get("name") if isinstance(node.get("author"), dict) else node.get("author"),
                "date": node.get("datePublished"),
                "url": article_url(node.get("url") if isinstance(node.get("url"), str) else None, link),
            }
            try:
                if raw_value is not None and not unscored:
                    record["raw_score"] = float(raw_value)
            except (TypeError, ValueError):
                pass
            if game is None:
                record["reason"] = reason
                queue.append(record)
            else:
                slug = game.get("slug")
                collected.setdefault(slug, {"title": game.get("title"), "reviews": []})
                collected[slug]["reviews"].append(record)


def igdb_token() -> str | None:
    import os

    client_id = os.environ.get("TWITCH_CLIENT_ID")
    secret = os.environ.get("TWITCH_CLIENT_SECRET")
    if not client_id or not secret:
        return None
    body = urllib.parse.urlencode(
        {"client_id": client_id, "client_secret": secret, "grant_type": "client_credentials"}
    ).encode()
    request = urllib.request.Request("https://id.twitch.tv/oauth2/token", data=body)
    with urllib.request.urlopen(request, timeout=30) as response:
        payload = json.loads(response.read().decode())
    return payload.get("access_token")


def fetch_metadata(name: str) -> dict | None:
    """IGDB first, RAWG second. Never Metacritic or OpenCritic."""
    import os

    token = igdb_token()
    client_id = os.environ.get("TWITCH_CLIENT_ID")
    if token and client_id:
        query = f'search "{name.replace(chr(34), "")}"; fields name,summary,first_release_date,cover.url,platforms.name; limit 5;'
        request = urllib.request.Request(
            "https://api.igdb.com/v4/games",
            data=query.encode(),
            headers={"Client-ID": client_id, "Authorization": f"Bearer {token}", "Accept": "application/json"},
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            rows = json.loads(response.read().decode())
        if rows:
            row = rows[0]
            cover = (row.get("cover") or {}).get("url")
            if isinstance(cover, str) and cover.startswith("//"):
                cover = "https:" + cover
            return {
                "source": "igdb",
                "igdb_id": row.get("id"),
                "name": row.get("name"),
                "summary": row.get("summary"),
                "cover_url": cover,
                "platforms": [item.get("name") for item in row.get("platforms") or [] if isinstance(item, dict)],
            }
    rawg = os.environ.get("RAWG_API_KEY")
    if rawg:
        url = "https://api.rawg.io/api/games?" + urllib.parse.urlencode({"search": name, "key": rawg, "page_size": 1})
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = json.loads(response.read().decode())
        results = payload.get("results") or []
        if results:
            row = results[0]
            return {
                "source": "rawg",
                "name": row.get("name"),
                "summary": None,
                "cover_url": row.get("background_image"),
                "platforms": [
                    (item.get("platform") or {}).get("name")
                    for item in row.get("platforms") or []
                    if isinstance(item, dict)
                ],
            }
    return None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--init-outlets", action="store_true")
    parser.add_argument("--metadata", action="store_true", help="Look up one title from IGDB or RAWG")
    parser.add_argument("--title", default="")
    parser.add_argument("--reviews", action="store_true", help="Read outlet feeds and write collected_reviews.json")
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    if args.self_check:
        try:
            assert_allowed("https://opencritic.com/game/1")
        except RuntimeError:
            pass
        else:
            raise SystemExit("blocklist failed")
        try:
            assert_allowed("https://www.metacritic.com/game/")
        except RuntimeError:
            pass
        else:
            raise SystemExit("blocklist failed")
        assert norm_title("Pokémon") == "pokemon"
        assert excerpt_of("One. Two. Three.") == "One. Two."
        assert article_url("https://kotaku.com/", "https://kotaku.com/hades-review") == "https://kotaku.com/hades-review"
        assert article_url("https://www.eurogamer.net/hades-review", "https://eurogamer.net/hades-review") == "https://www.eurogamer.net/hades-review"
        print("collect self-check ok")
        return
    if args.init_outlets or not OUTLETS_PATH.exists():
        build_outlets()
    if args.metadata:
        if not args.title:
            raise SystemExit("--metadata needs --title")
        found = fetch_metadata(args.title)
        print(json.dumps(found, indent=2))
        return
    if args.reviews:
        payload = json.loads(OUTLETS_PATH.read_text(encoding="utf-8"))
        index = load_catalog_index()
        queue: list[dict] = []
        collected: dict = {}
        for outlet in payload.get("outlets", []):
            if outlet.get("feed"):
                print(f"feed {outlet['slug']}")
                collect_feed(outlet, index, queue, collected)
        COLLECTED_PATH.write_text(json.dumps(collected, indent=2) + "\n", encoding="utf-8")
        QUEUE_PATH.write_text(json.dumps({"items": queue}, indent=2) + "\n", encoding="utf-8")
        print(f"collected {len(collected)} games, queued {len(queue)}")
        return
    if not args.init_outlets:
        parser.error("pass --init-outlets, --metadata, --reviews, or --self-check")


if __name__ == "__main__":
    main()
