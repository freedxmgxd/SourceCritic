#!/usr/bin/env python3
"""Fill missing review links from each outlet's own site.

The catalog stored most publications as aggregator-only, with no homepage and
no sitemap, so the collector never opened the article. This learns the site
from the review links that publication already has, then matches games that
are still unlinked against that site's sitemap.

A match has to contain every significant word of the game title and sit in a
review section. News, previews and trailers are left alone.
"""

from __future__ import annotations

import argparse
import gzip
import json
import re
import socket
import sqlite3
import unicodedata
import urllib.request
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
import sys

sys.path.insert(0, str(ROOT / "scripts"))
from repair_integrity import registrable  # noqa: E402

DB = ROOT / "sourcecritic.db"
SEED = ROOT / "data" / "seed_reviews.json"
OUTLETS = ROOT / "data" / "outlets.json"
CATALOG = ROOT / "data" / "catalog_outlets.json"

UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
)
SKIP_HOSTS = {
    "youtube.com",
    "youtu.be",
    "twitter.com",
    "x.com",
    "facebook.com",
    "instagram.com",
    "twitch.tv",
    "reddit.com",
}
GENERIC_SEGMENTS = {
    "news",
    "blog",
    "tag",
    "tags",
    "category",
    "author",
    "feature",
    "features",
    "video",
    "videos",
    "sony",
    "nintendo",
    "microsoft",
    "sega",
    "pc",
    "ps2",
    "ps3",
    "ps4",
    "ps5",
    "xbox",
    "xbox360",
    "wii",
    "switch",
    "playstation-4",
    "playstation-5",
}
REVIEW_MARKERS = (
    "review",
    "reviews",
    "analisis",
    "analise",
    "recenz",
    "critique",
    "critica",
    "inceleme",
    "teste",
)
STOP = {
    "the",
    "a",
    "an",
    "of",
    "and",
    "for",
    "to",
    "on",
    "in",
    "de",
    "la",
    "el",
    "da",
    "do",
    "edition",
    "remastered",
    "remake",
    "hd",
    "dx",
    "goty",
    "review",
    "game",
}
SLUG_NOISE = STOP | {
    "reviews",
    "review",
    "recenzja",
    "recenzje",
    "analisis",
    "analise",
    "analyse",
    "critique",
    "critica",
    "teste",
    "test",
    "pc",
    "ps2",
    "ps3",
    "ps4",
    "ps5",
    "switch",
    "xbox",
    "x360",
    "pro",
    "html",
    "shtml",
    "php",
    "article",
    "articles",
    "magazine",
    "cdaction",
    "pl",
    "www",
    "com",
}
MAX_SITEMAPS = 40
MAX_BYTES = 5_000_000
socket.setdefaulttimeout(20)


def fold(text: str) -> str:
    normalized = unicodedata.normalize("NFKD", text)
    stripped = "".join(ch for ch in normalized if not unicodedata.combining(ch))
    return stripped.lower()


ROMAN = {"i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii"}


def tokens(title: str) -> list[str]:
    words = re.findall(r"[a-z0-9]+", fold(title))
    kept: list[str] = []
    for index, word in enumerate(words):
        last = index == len(words) - 1
        if word in STOP:
            continue
        if len(word) > 1 or word.isdigit() or word in ROMAN and (last or len(word) > 1):
            kept.append(word)
    return kept


def path_tokens(url: str) -> set[str]:
    path = urlparse(url).path
    return set(re.findall(r"[a-z0-9]+", fold(path)))


def is_homepage(url: str | None) -> bool:
    if not url:
        return True
    parsed = urlparse(url)
    if parsed.path not in ("", "/"):
        return False
    return not parsed.query


def reviewish(url: str, section: str | None) -> bool:
    path = fold(urlparse(url).path)
    segments = [part for part in path.split("/") if part]
    news = ("news", "newsy", "noticias", "noticia", "blog", "blogs", "actualidad", "guia", "guide", "guides")
    if any(part in news or part.startswith("news") for part in segments):
        return False
    bad = {"trailer", "trailers", "video", "videos", "podcast", "interview", "tag", "tags", "category", "author"}
    has_marker = any(marker in path for marker in REVIEW_MARKERS)
    if set(segments) & bad and not has_marker:
        return False
    if any(word in path for word in ("trailer", "preview", "hands-on", "impressions")) and not has_marker:
        return False
    if has_marker:
        return True
    if section and section in segments:
        return True
    return False


def slug_extra(title: str, url: str) -> list[str] | None:
    """Words in the article slug that are not the game. None when the title is not fully there."""
    needed = tokens(title)
    if not needed:
        return None
    have = path_tokens(url)
    if any(token not in have for token in needed):
        return None
    slug = urlparse(url).path.rstrip("/").split("/")[-1]
    extra = []
    for word in re.findall(r"[a-z0-9]+", fold(slug)):
        if word in needed or word in SLUG_NOISE:
            continue
        if word.isdigit() and len(word) == 4:
            continue
        extra.append(word)
    return extra


def match_url(title: str, urls: list[str], section: str | None) -> str | None:
    ranked: list[tuple[int, int, str]] = []
    for url in urls:
        if not reviewish(url, section):
            continue
        extra = slug_extra(title, url)
        if extra is None or extra:
            continue
        ranked.append((len(urlparse(url).path), url.split("?")[0]))
    if not ranked:
        return None
    ranked.sort()
    return ranked[0][1]


def learn_sites(db: sqlite3.Connection) -> dict[str, dict]:
    rows = db.execute(
        """
        SELECT o.slug, r.review_url
        FROM reviews r JOIN outlets o ON o.id = r.outlet_id
        WHERE r.review_url IS NOT NULL AND r.review_url != ''
        """
    )
    hosts: dict[str, Counter] = defaultdict(Counter)
    domains: dict[str, Counter] = defaultdict(Counter)
    segments: dict[str, Counter] = defaultdict(Counter)
    bare_hosts: dict[str, Counter] = defaultdict(Counter)
    for slug, url in rows:
        parsed = urlparse(url)
        if is_homepage(url):
            bare = (parsed.hostname or "").lower()
            if bare and registrable(bare) not in SKIP_HOSTS:
                bare_hosts[slug][bare] += 1
            continue
        host = (parsed.hostname or "").lower()
        if not host:
            continue
        domain = registrable(host)
        if domain in SKIP_HOSTS:
            continue
        hosts[slug][host] += 1
        domains[slug][domain] += 1
        segment = parsed.path.strip("/").split("/")[0].lower() if parsed.path.strip("/") else ""
        if segment:
            segments[slug][segment] += 1
    sites = {}
    for slug, counted in domains.items():
        domain, domain_n = counted.most_common(1)[0]
        known = sum(counted.values())
        if domain_n < 4 or domain_n / known < 0.6:
            continue
        host, _host_n = hosts[slug].most_common(1)[0]
        section = None
        if segments[slug]:
            segment, segment_n = segments[slug].most_common(1)[0]
            useful = (
                segment_n / known >= 0.5
                and not segment.isdigit()
                and segment not in GENERIC_SEGMENTS
                and len(segment) > 2
            )
            if useful and not any(marker in segment for marker in REVIEW_MARKERS):
                homepage = f"https://{host}/{segment}/"
                section = None
            else:
                homepage = f"https://{host}/"
                if useful or (segment_n / known >= 0.5 and any(marker in segment for marker in REVIEW_MARKERS)):
                    section = segment
        else:
            homepage = f"https://{host}/"
        sites[slug] = {"domain": domain, "host": host, "homepage": homepage, "section": section}
    for slug, counted in bare_hosts.items():
        if slug in sites:
            continue
        host, host_n = counted.most_common(1)[0]
        known = sum(counted.values())
        if host_n < 4 or host_n / known < 0.6:
            continue
        sites[slug] = {
            "domain": registrable(host),
            "host": host,
            "homepage": f"https://{host}/",
            "section": None,
        }
    return sites


def fetch(url: str) -> bytes | None:
    request = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/xml,text/xml,*/*"})
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            data = response.read(MAX_BYTES + 1)
    except Exception:
        return None
    if len(data) > MAX_BYTES:
        return None
    if data[:2] == b"\x1f\x8b":
        try:
            data = gzip.decompress(data)
        except Exception:
            return None
    return data


def locs(xml: bytes) -> tuple[list[str], list[str]]:
    try:
        root = ET.fromstring(xml)
    except ET.ParseError:
        return [], []
    maps: list[str] = []
    pages: list[str] = []
    for node in root.iter():
        tag = node.tag.rsplit("}", 1)[-1]
        if tag != "loc" or not node.text:
            continue
        loc = node.text.strip()
        if not loc.startswith("http"):
            continue
        parent = ""
        # ElementTree has no parent pointer; classify by the document root.
        root_tag = root.tag.rsplit("}", 1)[-1]
        if root_tag == "sitemapindex":
            maps.append(loc)
        else:
            pages.append(loc)
    return maps, pages


def sitemap_priority(url: str) -> int:
    lowered = url.lower()
    if any(part in lowered for part in ("news", "image", "video", "author", "category", "tag")):
        return 0
    if any(part in lowered for part in ("review", "analisis", "recenz", "post-sitemap", "article")):
        return 2
    return 1


def site_urls(homepage: str) -> tuple[str | None, list[str]]:
    host = urlparse(homepage).hostname or ""
    robots = fetch(f"https://{host}/robots.txt")
    found: list[str] = []
    if robots:
        for line in robots.decode("utf-8", "replace").splitlines():
            if line.lower().startswith("sitemap:"):
                found.append(line.split(":", 1)[1].strip())
    if not found:
        for suffix in ("/sitemap.xml", "/sitemap_index.xml", "/wp-sitemap.xml"):
            found.append(f"https://{host}{suffix}")
    pages: list[str] = []
    seen: set[str] = set()
    queue = found[:]
    pulled = 0
    sitemap_used = None
    while queue and pulled < MAX_SITEMAPS:
        url = queue.pop(0)
        if url in seen:
            continue
        seen.add(url)
        body = fetch(url)
        pulled += 1
        if not body:
            continue
        maps, page_locs = locs(body)
        if maps or page_locs:
            sitemap_used = sitemap_used or url
        pages.extend(page_locs)
        ranked = sorted(maps, key=sitemap_priority, reverse=True)
        for child in ranked:
            if sitemap_priority(child) == 0:
                continue
            if child not in seen:
                queue.append(child)
    # Keep only pages on this registrable domain.
    prefix = urlparse(homepage).path.rstrip("/")
    kept = []
    for url in pages:
        parsed = urlparse(url)
        page_host = (parsed.hostname or "").lower().removeprefix("www.")
        if page_host != host.removeprefix("www."):
            continue
        if prefix and not parsed.path.startswith(prefix):
            continue
        kept.append(url.split("#", 1)[0])
    return sitemap_used, kept


def missing_reviews(db: sqlite3.Connection, slug: str) -> list[tuple[int, str, str, str | None]]:
    rows = db.execute(
        """
        SELECT r.id, g.slug, g.title, r.review_url
        FROM reviews r
        JOIN outlets o ON o.id = r.outlet_id
        JOIN games g ON g.id = r.game_id
        WHERE o.slug = ?
        """,
        (slug,),
    ).fetchall()
    return [(rid, game, title, url) for rid, game, title, url in rows if is_homepage(url)]


def remember_sites(sites: dict[str, dict], sitemaps: dict[str, str]) -> None:
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    outlets = json.loads(OUTLETS.read_text(encoding="utf-8"))
    by_slug = {row["slug"]: row for row in outlets.get("outlets", [])}
    for slug, site in sites.items():
        entry = catalog.get("outlets", {}).get(slug)
        if entry and not entry.get("homepage"):
            entry["homepage"] = site["homepage"]
        sitemap = sitemaps.get(slug)
        if entry and sitemap:
            indexes = entry.setdefault("review_index_urls", [])
            if sitemap not in indexes:
                indexes.append(sitemap)
        row = by_slug.get(slug)
        if not row:
            continue
        if not row.get("domain"):
            row["domain"] = site["domain"]
        if not row.get("homepage"):
            row["homepage"] = site["homepage"]
        if sitemap and not row.get("sitemap"):
            row["sitemap"] = sitemap
    CATALOG.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    OUTLETS.write_text(json.dumps(outlets, indent=2) + "\n", encoding="utf-8")


def remember_seed(found: list[tuple[str, str, str]]) -> int:
    if not found or not SEED.exists():
        return 0
    data = json.loads(SEED.read_text(encoding="utf-8"))
    written = 0
    for game_slug, outlet_slug, url in found:
        game = data.get(game_slug)
        if not game:
            continue
        for review in game.get("reviews") or []:
            if (review.get("slug") or "") != outlet_slug:
                continue
            if review.get("url") and not is_homepage(review.get("url")):
                continue
            review["url"] = url
            written += 1
            break
    if written:
        SEED.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return written


def self_check() -> None:
    section = "reviews"
    urls = [
        "https://gamemag.ru/reviews/red-dead-redemption-2-pc-review",
        "https://gamemag.ru/news/red-dead-redemption-2-trailer",
        "https://gamemag.ru/reviews/red-dead-redemption",
        "https://gamemag.ru/reviews/super-mario-odyssey",
        "https://gamemag.ru/reviews/super-mario-galaxy",
    ]
    assert match_url("Red Dead Redemption 2", urls, section) == urls[0]
    assert match_url("Super Mario Odyssey", urls, section) == urls[3]
    assert match_url("Super Mario Galaxy", urls, section) == urls[4]
    gta = [
        "https://example.com/reviews/grand-theft-auto-v",
        "https://example.com/reviews/grand-theft-auto-iv",
    ]
    assert match_url("Grand Theft Auto V", gta, "reviews") == gta[0]
    assert match_url("Grand Theft Auto IV", gta, "reviews") == gta[1]
    news = ["https://cdaction.pl/newsy/the-legend-of-zelda-breath-of-the-wild-strona-z-krytyczna-recenzja-padla-ofiara-ataku-ddos/"]
    assert match_url("The Legend of Zelda: Breath of the Wild", news, "recenzje") is None
    assert match_url("God of War", ["https://gamemag.ru/reviews/god-of-war-ascension"], "reviews") is None
    assert match_url("God of War", ["https://gamemag.ru/reviews/god-of-war-ascension", "https://gamemag.ru/reviews/god-of-war-ps4"], "reviews") == "https://gamemag.ru/reviews/god-of-war-ps4"
    assert match_url("Persona 5", ["https://gamemag.ru/reviews/persona-5-royal-ps4-pro"], "reviews") is None
    assert match_url("The Last of Us", ["https://cdaction.pl/recenzje/the-last-of-us-part-ii-recenzja"], "recenzje") is None
    assert is_homepage("https://easyallies.com")
    assert not is_homepage("https://hardcoregamer.com?post_type=post&p=428568")
    print("backfill self-check ok")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--self-check", action="store_true")
    parser.add_argument("--limit-outlets", type=int, default=0)
    parser.add_argument("--slugs", default="")
    args = parser.parse_args()
    if args.self_check:
        self_check()
        return
    db = sqlite3.connect(DB, timeout=60)
    sites = learn_sites(db)
    slugs = sorted(sites, key=lambda slug: len(missing_reviews(db, slug)), reverse=True)
    if args.slugs:
        wanted = {part.strip() for part in args.slugs.split(",") if part.strip()}
        slugs = [slug for slug in slugs if slug in wanted]
    if args.limit_outlets:
        slugs = slugs[: args.limit_outlets]
    print(f"outlets with a known site: {len(slugs)}", flush=True)
    if args.write:
        remember_sites(sites, {})
    updates: list[tuple[str, int]] = []
    seed_rows: list[tuple[str, str, str]] = []
    sitemaps: dict[str, str] = {}
    for slug in slugs:
        pending = missing_reviews(db, slug)
        if not pending:
            continue
        site = sites[slug]
        print(f"fetch {slug} missing={len(pending)}", flush=True)
        sitemap, pages = site_urls(site["homepage"])
        if sitemap:
            sitemaps[slug] = sitemap
        matched = 0
        for rid, game_slug, title, _url in pending:
            found = match_url(title, pages, site["section"])
            if not found:
                continue
            matched += 1
            updates.append((found, rid))
            seed_rows.append((game_slug, slug, found))
            if not args.write:
                print(f"  {slug}: {title} -> {found}", flush=True)
        print(f"{slug} missing={len(pending)} pages={len(pages)} matched={matched}", flush=True)
        if args.write and matched:
            batch = updates[-matched:]
            db.executemany("UPDATE reviews SET review_url=? WHERE id=?", batch)
            db.commit()
    print(f"matched {len(updates)} reviews", flush=True)
    if not args.write:
        return
    seeded = remember_seed(seed_rows)
    remember_sites(sites, sitemaps)
    print(f"wrote {len(updates)} links, seed {seeded}", flush=True)


if __name__ == "__main__":
    main()
