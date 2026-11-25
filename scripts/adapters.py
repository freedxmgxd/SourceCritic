#!/usr/bin/env python3
"""Outlet merge helpers. Metacritic and OpenCritic fetches are disabled."""

from __future__ import annotations

import json
import re
import time
import unicodedata
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
    """Metacritic search is disabled (compliance)."""
    del title
    return None


def mc_details(slug: str) -> dict | None:
    """Metacritic game details are disabled (compliance)."""
    del slug
    return None


def _mc_platform_name(it: dict) -> str | None:
    plat = it.get("platform")
    if isinstance(plat, str) and plat.strip():
        return plat.strip()
    if isinstance(plat, dict):
        name = (plat.get("name") or "").strip()
        if name:
            return name
    product = it.get("reviewedProduct") or {}
    nested = product.get("platform") or {}
    if isinstance(nested, dict):
        name = (nested.get("name") or "").strip()
        if name:
            return name
    return None


def mc_reviews(slug: str) -> list[dict]:
    """Metacritic critic reviews are disabled (compliance)."""
    del slug
    return []


def is_mc_edition_slug(slug: str | None) -> bool:
    if not slug:
        return False
    return bool(_MC_EDITION_SLUG_RE.search(slug.strip().lower()))


def prefer_base_mc_slug(slug: str | None, title: str | None = None) -> str | None:
    """If catalog/search resolved to a Gold/Complete/port SKU, prefer the base game."""
    if not slug or not is_mc_edition_slug(slug):
        return slug
    # Strip one edition suffix and re-check details exist via caller.
    base = _MC_EDITION_SLUG_RE.sub("", slug.strip().lower()).strip("-")
    return base or slug


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
    """OpenCritic search is disabled (compliance)."""
    del title, fuzzy
    return None


def oc_parse_state(html: str) -> dict:
    match = re.search(
        r'<script[^>]*type="application/json"[^>]*>(.*?)</script>', html, re.S
    )
    if not match:
        return {}
    # OpenCritic embeds a custom-escaped JSON blob (&q; &s; &a; …).
    raw = (
        unescape(match.group(1))
        .replace("&q;", '"')
        .replace("&s;", "'")
        .replace("&a;", "&")
    )
    return json.loads(raw)


_BAD_URL_HOSTS = (
    "savethechildren.org",
    "support.savethechildren.org",
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
)


def sanitize_review_url(url: str | None) -> str | None:
    """Normalize aggregator URLs; drop donation / junk / non-http links."""
    if not url:
        return None
    link = unescape(str(url)).strip().replace("&a;", "&").replace("&amp;", "&")
    if not (link.startswith("http://") or link.startswith("https://")):
        return None
    host = urllib.parse.urlparse(link).hostname or ""
    host = host.lower().removeprefix("www.")
    if any(host == bad or host.endswith("." + bad) for bad in _BAD_URL_HOSTS):
        return None
    # Trailer / non-review media hosts occasionally leak into OC externalUrl.
    if host in {"imdb.com", "m.imdb.com"}:
        return None
    return link


def oc_reviews(game_id: int, slug: str) -> list[dict]:
    """OpenCritic reviews are disabled (compliance)."""
    del game_id, slug
    return []


def oc_details(game_id: int, slug: str) -> dict | None:
    """OpenCritic details are disabled (compliance)."""
    del game_id, slug
    return None


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
    # TLD / punctuation slug variants (MC vs OC / API drift).
    "gameblogfr": "gameblog-fr",
    "gameblog-fr": "gameblog-fr",
    "4pde": "4p-de",
    "4p-de": "4p-de",
    "gamescz": "games-cz",
    "games-cz": "games-cz",
    "jeuxvideocom": "jeuxvideo-com",
    "jeuxvideo-com": "jeuxvideo-com",
    "gameovergr": "gameover-gr",
    "gameover-gr": "gameover-gr",
    "gamernl": "gamer-nl",
    "gamer-nl": "gamer-nl",
    "insidegamernl": "insidegamer-nl",
    "insidegamer-nl": "insidegamer-nl",
    "totalgamesnet": "totalgames-net",
    "totalgames-net": "totalgames-net",
    "uagnait": "uagna-it",
    "uagna-it": "uagna-it",
    "level7nu": "level7-nu",
    "level7-nu": "level7-nu",
    "playtm": "play-tm",
    "play-tm": "play-tm",
    "playgroundru": "playground-ru",
    "playground-ru": "playground-ru",
    "gamer-20": "gamer-2-0",
    "gamer-2-0": "gamer-2-0",
    "official-us-playstation-magazine": "official-u-s-playstation-magazine",
    "official-u-s-playstation-magazine": "official-u-s-playstation-magazine",
    "whats-it-like": "what-s-it-like",
    "what-s-it-like": "what-s-it-like",
    "gamers-hell": "gamer-s-hell",
    "gamer-s-hell": "gamer-s-hell",
    "gamers-pulse": "gamer-s-pulse",
    "gamer-s-pulse": "gamer-s-pulse",
    "gamersat": "gamers-at",
    "gamers-at": "gamers-at",
    "gamertv": "gamer-tv",
    "gamer-tv": "gamer-tv",
    "videogameszonede": "videogameszone-de",
    "videogameszone-de": "videogameszone-de",
    "rogerebertcom": "rogerebert-com",
    "rogerebert-com": "rogerebert-com",
    "spilstrom": "spilstr-m",
    "spilstr-m": "spilstr-m",
    "julien-chieze": "julien-chi-ze",
    "julien-chi-ze": "julien-chi-ze",
    "dreamcastnet": "dreamcast-net",
    "dreamcast-net": "dreamcast-net",
    "gamesvillageit": "gamesvillage-it",
    "gamesvillage-it": "gamesvillage-it",
    "lets-play-video-games": "let-s-play-video-games",
    "let-s-play-video-games": "let-s-play-video-games",
    "ps3bloggense": "ps3bloggen-se",
    "ps3bloggen-se": "ps3bloggen-se",
    "applenapps": "apple-n-apps",
    "apple-n-apps": "apple-n-apps",
    "yahoo!": "yahoo",
    "yahoo": "yahoo",
    "gamezilla!": "gamezilla",
    "gamezilla": "gamezilla",
    "spank!": "spank",
    "spank": "spank",
    "d+pad-magazine": "d-pad-magazine",
    "d-pad-magazine": "d-pad-magazine",
    "wham!-gaming": "wham-gaming",
    "wham-gaming": "wham-gaming",
    "antagonist-antcom": "antagonist-ant-com",
    "antagonist----antcom": "antagonist-ant-com",
    "antagonist-ant-com": "antagonist-ant-com",
    # Same outlet, MC country suffix vs OC bare name.
    "merlin-in-kazan": "merlin-in-kazan",
    "merlin-in-kazan-turkey": "merlin-in-kazan",
    "merlinin-kazan-turkey": "merlin-in-kazan",
    "pc-games-russia": "pc-games",
    "pc-games": "pc-games",
    # IGN regional sites — one vote per locale brand.
    "ign-italia": "ign-italia",
    "ign-italy": "ign-italia",
    "ign-espana": "ign-espana",
    "ign-espa-a": "ign-espana",
    "ign-spain": "ign-espana",
    "ign-deutschland": "ign-deutschland",
    "ign-germany": "ign-deutschland",
    "ign-nordic": "ign-nordic",
    "ign-sweden": "ign-nordic",
    "ign-ads": "ign",
    # Direct adapters: keep stable ids even when aggregator uses display names.
    "ign": "ign",
    "pc-gamer": "pc-gamer",
    "eurogamer": "eurogamer",
    "gamespot": "gamespot",
}


# Aggregate user scores / recommend % — never treat as critic outlets.
PSEUDO_OUTLET_SLUGS = frozenset({"metacritic-users", "opencritic-recommend"})
# Kotaku does not publish a number. A stored 0/100 is an old yes/no flag, not a grade.
UNSCORED_SLUGS = frozenset({"kotaku"})


def decode_text(value: str | None) -> str | None:
    """Undo OpenCritic's custom entities (&a; &q; &s;) and a broken &amp;a;."""
    if value is None:
        return None
    text = str(value)
    for _ in range(4):
        nxt = (
            text.replace("&amp;a;", "&")
            .replace("&amp;", "&")
            .replace("&a;", "&")
            .replace("&q;", '"')
            .replace("&s;", "'")
        )
        nxt = unescape(nxt)
        if nxt == text:
            break
        text = nxt
    return text


def fold_name(value: str | None) -> str:
    """Identity key: same publication, ignoring punctuation, case, and accents."""
    text = decode_text(value) or ""
    text = unicodedata.normalize("NFKD", text)
    text = "".join(ch for ch in text if not unicodedata.combining(ch))
    return re.sub(r"[^a-z0-9]+", "", text.lower())


def canonicalize_outlet_slug(slug: str) -> str:
    key = (slug or "").lower().strip()
    if not key:
        return "item"
    # Strip residual punctuation the aggregator sometimes leaves in.
    key = re.sub(r"[^a-z0-9]+", "-", key).strip("-") or "item"
    return OUTLET_ALIASES.get(key, key)


# MC often appends "(Country)" to the same outlet OC lists without it.
_OUTLET_COUNTRY_SUFFIX = re.compile(
    r"\s*\((?:Turkey|Russia|Spain|Italy|France|Germany|Brazil|Poland|Japan|UK|US|USA|"
    r"Netherlands|Sweden|Norway|Denmark|Finland|Portugal|Mexico|Argentina|China|"
    r"Korea|Australia|Canada|Belgium|Austria|Switzerland|Greece|"
    r"Czech(?:ia| Republic)?|Hungary|Romania|Ukraine|Israel|India|Taiwan|"
    r"Hong Kong|Singapore|New Zealand|Ireland|Scotland|Wales|England)\)\s*$",
    re.I,
)

# Only strip country when the bare name is a known alias target (avoids
# collapsing unrelated "Play (Poland)" into catalog outlet "play").
_OUTLET_STRIP_COUNTRY_BASES = frozenset(
    {
        "merlin-in-kazan",
        "pc-games",
    }
)


def publication_merge_name(name: str) -> str:
    """Display-oriented name; strip MC regional suffix for known shared brands."""
    bare = _OUTLET_COUNTRY_SUFFIX.sub("", name).strip() or name.strip()
    if bare == name.strip():
        return name.strip()
    bare_key = canonicalize_outlet_slug(slugify(bare))
    full_key = canonicalize_outlet_slug(slugify(name))
    if bare_key in _OUTLET_STRIP_COUNTRY_BASES or full_key in _OUTLET_STRIP_COUNTRY_BASES:
        return bare
    # Also strip when alias map already folds the country slug into the bare one.
    if full_key == bare_key or OUTLET_ALIASES.get(slugify(name)) == bare_key:
        return bare
    return name.strip()


def outlet_merge_key(review: dict) -> str:
    """Stable outlet identity across Metacritic / OpenCritic / outlet scrapes.

    Prefer the publication display name (slugified) over API-provided slugs —
    MC and OC often disagree on slugs (multiplayer-it vs multiplayerit) for the
    same outlet, which used to create duplicate critic rows.
    """
    name = (review.get("publication") or review.get("outlet") or "").strip()
    if name:
        return canonicalize_outlet_slug(slugify(publication_merge_name(name)))
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
            chosen = dict(review)
        elif new_rank < prev_rank:
            chosen = dict(prev)
        elif not prev.get("excerpt") and review.get("excerpt"):
            chosen = dict(review)
        elif prev.get("excerpt") and not review.get("excerpt"):
            chosen = dict(prev)
        else:
            # Same rank: prefer the one with a cleaner URL / keep existing.
            chosen = dict(prev)
        # Prefer display name without MC regional "(Country)" suffix.
        prev_pub = (prev.get("publication") or "").strip()
        new_pub = (review.get("publication") or "").strip()
        if prev_pub and new_pub:
            prev_bare = publication_merge_name(prev_pub)
            new_bare = publication_merge_name(new_pub)
            if prev_bare == new_bare or outlet_merge_key(prev) == outlet_merge_key(review):
                if "(" in chosen.get("publication", "") and "(" not in prev_bare:
                    chosen["publication"] = prev_bare
                elif "(" in chosen.get("publication", "") and "(" not in new_bare:
                    chosen["publication"] = new_bare
        # Union platforms so per-port UI keeps every SKU an outlet covered.
        plats: list[str] = []
        for src in (prev, review, chosen):
            for p in src.get("platforms") or []:
                if isinstance(p, str) and p.strip() and p not in plats:
                    plats.append(p.strip())
        if plats:
            chosen["platforms"] = plats
        # Prefer a sanitized URL over a junk/missing one.
        prev_url = sanitize_review_url(prev.get("url"))
        new_url = sanitize_review_url(review.get("url"))
        if new_url and (not prev_url or "savethechildren" in (chosen.get("url") or "")):
            chosen["url"] = new_url
        elif prev_url:
            chosen["url"] = prev_url
        else:
            chosen.pop("url", None)
        return chosen

    for review in existing + new_reviews:
        review = dict(review)
        raw_slug = (review.get("slug") or "").lower().strip()
        if raw_slug in PSEUDO_OUTLET_SLUGS:
            continue
        if review.get("excerpt"):
            review["excerpt"] = decode_text(review.get("excerpt"))
        if review.get("publication"):
            review["publication"] = decode_text(review.get("publication"))
        key = outlet_merge_key(review)
        if key in PSEUDO_OUTLET_SLUGS:
            continue
        if key in UNSCORED_SLUGS or canonicalize_outlet_slug(raw_slug) in UNSCORED_SLUGS:
            # Keep the excerpt and the link. The number is not a grade.
            review["score"] = None
        else:
            # Drop non-scores (null/missing). Keep a real 0.0 only when present.
            if review.get("score") is None:
                continue
            try:
                float(review["score"])
            except (TypeError, ValueError):
                continue
        url_key = _norm_url(sanitize_review_url(review.get("url")))
        # Same clean URL + same outlet key → collapse. Never merge distinct
        # outlets that share a broken/redirect URL.
        if url_key and url_key in url_to_key and url_to_key[url_key] != key:
            url_key = ""
        elif url_key and url_key in url_to_key:
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
        pub = (chosen.get("publication") or "").strip()
        if pub:
            chosen["publication"] = publication_merge_name(pub)
        clean = sanitize_review_url(chosen.get("url"))
        if clean:
            chosen["url"] = clean
        else:
            chosen.pop("url", None)
    return list(by_key.values())


def dedupe_seed_reviews(seed: dict) -> tuple[dict, int]:
    """Collapse MC/OC/outlet duplicates in-place across the whole seed.

    Rewrites every review to the canonical outlet slug / display name so
    seed_examples and the live DB never see split rows like
    merlin-in-kazan vs merlin-in-kazan-turkey.

    Returns (seed, total_rows_removed).
    """
    removed = 0
    for entry in seed.values():
        reviews = entry.get("reviews")
        if not isinstance(reviews, list) or not reviews:
            continue
        before = len(reviews)
        normalized: list[dict] = []
        for review in reviews:
            row = dict(review)
            pub = (row.get("publication") or row.get("outlet") or "").strip()
            if pub:
                row["publication"] = publication_merge_name(pub)
            row["slug"] = outlet_merge_key(row)
            normalized.append(row)
        entry["reviews"] = merge_reviews(normalized, [])
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
