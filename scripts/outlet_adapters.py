#!/usr/bin/env python3
"""
Direct outlet-site review adapters.

Fetch scores from the publication's own pages (JSON-LD / schema.org Rating)
so the catalog is not solely dependent on Metacritic/OpenCritic aggregators.
"""

from __future__ import annotations

import json
import re
import time
import urllib.parse
import urllib.request
from html import unescape
from typing import Any
from urllib.parse import urljoin

UA = {
    "User-Agent": (
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
    ),
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "en-US,en;q=0.9",
}

# Host → catalog outlet slug / display name.
OUTLET_BY_HOST: dict[str, tuple[str, str]] = {
    "ign.com": ("ign", "IGN"),
    "www.ign.com": ("ign", "IGN"),
    "pcgamer.com": ("pc-gamer", "PC Gamer"),
    "www.pcgamer.com": ("pc-gamer", "PC Gamer"),
    "eurogamer.net": ("eurogamer", "Eurogamer"),
    "www.eurogamer.net": ("eurogamer", "Eurogamer"),
    "gamesradar.com": ("gamesradar", "GamesRadar+"),
    "www.gamesradar.com": ("gamesradar", "GamesRadar+"),
    "gamespot.com": ("gamespot", "GameSpot"),
    "www.gamespot.com": ("gamespot", "GameSpot"),
}

ADAPTERS: dict[str, dict[str, Any]] = {
    "ign": {
        "name": "IGN",
        "homepage": "https://www.ign.com/",
        "hosts": ("ign.com",),
        "search": "https://www.ign.com/search?q={q}",
        "link_must": ("/articles/",),
        "link_review": ("review",),
    },
    "pc-gamer": {
        "name": "PC Gamer",
        "homepage": "https://www.pcgamer.com/",
        "hosts": ("pcgamer.com",),
        "search": "https://www.pcgamer.com/search/?searchTerm={q}",
        "link_must": (),
        "link_review": ("review",),
        "guess": "https://www.pcgamer.com/{slug}-review/",
    },
    "eurogamer": {
        "name": "Eurogamer",
        "homepage": "https://www.eurogamer.net/",
        "hosts": ("eurogamer.net",),
        "search": "https://www.eurogamer.net/search?q={q}",
        "link_must": (),
        "link_review": ("review",),
        "guess": "https://www.eurogamer.net/{slug}-review",
    },
    "gamesradar": {
        "name": "GamesRadar+",
        "homepage": "https://www.gamesradar.com/",
        "hosts": ("gamesradar.com",),
        "search": "https://www.gamesradar.com/search/?searchTerm={q}",
        "link_must": (),
        "link_review": ("review",),
    },
    # GameSpot often 403s anonymous clients; keep adapter for known URLs.
    "gamespot": {
        "name": "GameSpot",
        "homepage": "https://www.gamespot.com/",
        "hosts": ("gamespot.com",),
        "search": None,
        "link_must": ("/reviews/",),
        "link_review": ("review",),
    },
}


def slugify(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "item"


def http_text(url: str, timeout: int = 40) -> tuple[str, str]:
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return response.geturl(), response.read().decode("utf-8", "ignore")


def host_of(url: str) -> str:
    try:
        return urllib.parse.urlparse(url).netloc.lower()
    except Exception:
        return ""


def outlet_for_url(url: str) -> tuple[str, str] | None:
    host = host_of(url)
    if host in OUTLET_BY_HOST:
        return OUTLET_BY_HOST[host]
    # regional IGN (br.ign.com, etc.)
    if host.endswith(".ign.com") or host == "ign.com":
        return ("ign", "IGN")
    if host.endswith(".eurogamer.net") or "eurogamer." in host:
        return ("eurogamer", "Eurogamer")
    return None


def _walk_json(obj: Any):
    stack = [obj]
    while stack:
        cur = stack.pop()
        if isinstance(cur, dict):
            yield cur
            stack.extend(cur.values())
        elif isinstance(cur, list):
            stack.extend(cur)


def ratings_from_html(html: str) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for match in re.finditer(
        r'<script[^>]*type="application/ld\+json"[^>]*>(.*?)</script>',
        html,
        re.S | re.I,
    ):
        raw = unescape(match.group(1).strip())
        try:
            data = json.loads(raw)
        except Exception:
            continue
        for node in _walk_json(data):
            if "ratingValue" not in node:
                continue
            out.append(
                {
                    "type": node.get("@type"),
                    "ratingValue": node.get("ratingValue"),
                    "bestRating": node.get("bestRating"),
                    "worstRating": node.get("worstRating"),
                }
            )
    return out


# Default bestRating when JSON-LD omits it (per outlet convention).
DEFAULT_BEST: dict[str, float] = {
    "ign": 10.0,
    "pc-gamer": 100.0,
    "eurogamer": 5.0,
    "gamesradar": 5.0,
    "gamespot": 10.0,
}
DEFAULT_WORST: dict[str, float] = {
    "eurogamer": 1.0,  # Eurogamer historically uses 1–5
}


def normalize_score(
    value: Any,
    best: Any = None,
    worst: Any = None,
    *,
    outlet_slug: str | None = None,
) -> float | None:
    try:
        score = float(value)
    except (TypeError, ValueError):
        return None
    try:
        best_f = float(best) if best is not None else None
    except (TypeError, ValueError):
        best_f = None
    try:
        worst_f = float(worst) if worst is not None else None
    except (TypeError, ValueError):
        worst_f = None
    if best_f is None and outlet_slug in DEFAULT_BEST:
        best_f = DEFAULT_BEST[outlet_slug]
    if worst_f is None and outlet_slug in DEFAULT_WORST:
        worst_f = DEFAULT_WORST[outlet_slug]
    if worst_f is None:
        worst_f = 0.0
    if best_f is None:
        # Generic heuristic: classic 0–10 critic scale vs already 0–100.
        if score <= 10:
            best_f = 10.0
        else:
            best_f = 100.0
    if best_f <= worst_f:
        return None
    norm = (score - worst_f) / (best_f - worst_f) * 100.0
    return max(0.0, min(100.0, norm))


def excerpt_from_html(html: str) -> str | None:
    for pat in [
        r'property="og:description" content="([^"]+)"',
        r'name="description" content="([^"]+)"',
    ]:
        match = re.search(pat, html, re.I)
        if match:
            text = unescape(match.group(1)).strip()
            if text:
                return text[:600]
    return None


def fetch_review_url(url: str) -> dict | None:
    """Fetch an outlet review URL and return a normalized review dict."""
    info = outlet_for_url(url)
    if not info:
        return None
    slug, name = info
    try:
        final, html = http_text(url)
    except Exception:
        return None
    ratings = ratings_from_html(html)
    if not ratings:
        return None
    # Prefer explicit Rating nodes; else first with ratingValue.
    # Prefer a Rating node that declares bestRating (avoids ambiguous bare numbers).
    rating = next(
        (
            r
            for r in ratings
            if str(r.get("type")).lower() == "rating" and r.get("bestRating") is not None
        ),
        None,
    )
    if rating is None:
        rating = next(
            (r for r in ratings if str(r.get("type")).lower() == "rating"),
            ratings[0],
        )
    score = normalize_score(
        rating.get("ratingValue"),
        rating.get("bestRating"),
        rating.get("worstRating"),
        outlet_slug=slug,
    )
    if score is None:
        return None
    row: dict[str, Any] = {
        "publication": name,
        "slug": slug,
        "score": float(score),
        "source": "outlet",
        "url": final,
    }
    excerpt = excerpt_from_html(html)
    if excerpt:
        row["excerpt"] = excerpt
    return row


def _title_tokens(title: str) -> set[str]:
    stop = {"the", "a", "an", "of", "and", "review", "pc", "edition"}
    words = re.findall(r"[a-z0-9]+", title.lower())
    return {w for w in words if w not in stop and len(w) > 1}


def _score_link(title: str, href: str) -> float:
    tokens = _title_tokens(title)
    path = urllib.parse.urlparse(href).path.lower()
    if "review" not in path:
        return -1.0
    # Prefer exact-ish slug presence.
    hit = 0
    for tok in tokens:
        if tok in path:
            hit += 1
    if not tokens:
        return 0.0
    return hit / len(tokens)


def search_review_url(adapter_id: str, title: str) -> str | None:
    cfg = ADAPTERS.get(adapter_id)
    if not cfg:
        return None
    slug = slugify(title)
    # Direct guess first (fast path for PC Gamer / Eurogamer).
    guess_tmpl = cfg.get("guess")
    if guess_tmpl:
        guess = guess_tmpl.format(slug=slug)
        try:
            final, html = http_text(guess)
            if ratings_from_html(html):
                return final
        except Exception:
            pass

    search_tmpl = cfg.get("search")
    if not search_tmpl:
        return None
    url = search_tmpl.format(q=urllib.parse.quote(title))
    try:
        final, html = http_text(url)
    except Exception:
        return None

    hosts = cfg.get("hosts") or ()
    must = cfg.get("link_must") or ()
    candidates: list[tuple[float, str]] = []
    for href in re.findall(r'href="([^"]+)"', html):
        abs_url = urljoin(final, href)
        host = host_of(abs_url)
        if hosts and not any(host == h or host.endswith("." + h) for h in hosts):
            continue
        path = urllib.parse.urlparse(abs_url).path.lower()
        if must and not any(m in path for m in must):
            continue
        if "review" not in path:
            continue
        # Skip section indexes.
        if path.rstrip("/").endswith("reviews") or path.rstrip("/").endswith("review"):
            if path.count("/") <= 2:
                continue
        score = _score_link(title, abs_url)
        if score >= 0.4:
            candidates.append((score, abs_url.split("?")[0]))

    if not candidates:
        return None
    candidates.sort(key=lambda x: x[0], reverse=True)
    # De-dupe
    seen: set[str] = set()
    for _score, href in candidates:
        if href in seen:
            continue
        seen.add(href)
        return href
    return None


def existing_outlet_url(reviews: list[dict], adapter_id: str) -> str | None:
    """Prefer canonical English host URLs over regional mirrors."""
    cfg = ADAPTERS.get(adapter_id) or {}
    hosts = cfg.get("hosts") or ()
    preferred_host = None
    homepage = cfg.get("homepage") or ""
    if homepage:
        preferred_host = host_of(homepage)

    candidates: list[tuple[int, str]] = []
    for review in reviews:
        url = (review.get("url") or "").strip()
        if not url.startswith("http"):
            continue
        host = host_of(url)
        info = outlet_for_url(url)
        matched = (info and info[0] == adapter_id) or (
            hosts and any(host == h or host.endswith("." + h) for h in hosts)
        )
        if not matched:
            continue
        rank = 0
        if preferred_host and host == preferred_host:
            rank = 2
        elif hosts and host in hosts:
            rank = 1
        candidates.append((rank, url))
    if not candidates:
        return None
    candidates.sort(key=lambda x: x[0], reverse=True)
    return candidates[0][1]


def scrape_outlet_for_game(
    adapter_id: str,
    title: str,
    existing_reviews: list[dict] | None = None,
) -> dict | None:
    """Find + scrape one outlet review for a game title."""
    reviews = existing_reviews or []
    url = existing_outlet_url(reviews, adapter_id)
    if not url:
        url = search_review_url(adapter_id, title)
        time.sleep(0.15)
    if not url:
        return None
    row = fetch_review_url(url)
    time.sleep(0.1)
    return row


def scrape_configured_outlets(
    title: str,
    existing_reviews: list[dict] | None = None,
    adapters: list[str] | None = None,
) -> list[dict]:
    ids = adapters or ["ign", "pc-gamer", "eurogamer", "gamesradar"]
    out: list[dict] = []
    for adapter_id in ids:
        try:
            row = scrape_outlet_for_game(adapter_id, title, existing_reviews)
        except Exception:
            row = None
        if row:
            out.append(row)
    return out


def catalog_outlet_bootstrap() -> list[dict]:
    """Candidates to upsert into catalog_outlets.json for configured adapters."""
    rows = []
    for adapter_id, cfg in ADAPTERS.items():
        rows.append(
            {
                "slug": adapter_id,
                "name": cfg["name"],
                "homepage": cfg.get("homepage"),
                "scrape_adapter": f"custom:{adapter_id}",
                "discovered_via": "outlet-adapters",
                "scale": "0-100",
            }
        )
    return rows
