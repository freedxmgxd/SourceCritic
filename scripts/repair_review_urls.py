#!/usr/bin/env python3
"""Follow review links and store the address that actually opens.

Same-site redirects are saved. A link that leaves the outlet's site is left
alone. A 404 is retried with and without www.

Adrenaline Vault (avault.com) now redirects to an unrelated site. Those
reviews are replaced with the Wayback snapshot of the original article.
"""

from __future__ import annotations

import json
import sqlite3
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlparse, urlunparse
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from repair_integrity import registrable  # noqa: E402

UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
)
DB = ROOT / "sourcecritic.db"


def with_www(url: str, has_www: bool) -> str | None:
    parsed = urlparse(url)
    host = parsed.hostname or ""
    if not host:
        return None
    if has_www:
        if not host.startswith("www."):
            return None
        host = host[4:]
    else:
        if host.startswith("www."):
            return None
        host = "www." + host
    return urlunparse(parsed._replace(netloc=host))


# The domain was sold. A live 200 is the new site, not the review.
DEAD_HOSTS = {"avault.com"}
WAYBACK_TIMESTAMP = "20070601"


def snapshot_map() -> dict[str, str]:
    """Prefer curated AVault URLs from data/avault_snapshots.json when present."""
    path = ROOT / "data" / "avault_snapshots.json"
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    out: dict[str, str] = {}
    for entry in (payload.get("reviews") or {}).values():
        if not entry.get("keep"):
            continue
        url = entry.get("url") or ""
        code = ""
        if "game=" in url:
            code = url.split("game=")[-1].split("&")[0].split("#")[0].lower()
        if code and "web.archive.org" in url:
            out[code] = url
    return out


def wayback_snapshot(url: str) -> str | None:
    """Snapshot of the original article, closest to when Adrenaline Vault was alive."""
    parsed = urlparse(url)
    host = (parsed.hostname or "").lower().removeprefix("www.")
    if host not in DEAD_HOSTS:
        return None
    bare = (parsed.path or "") + (("?" + parsed.query) if parsed.query else "")
    code = ""
    if "game=" in bare:
        code = bare.split("game=")[-1].split("&")[0].split("#")[0].lower()
    mapped = snapshot_map().get(code) if code else None
    if mapped:
        return mapped
    candidates = [f"http://www.{host}{bare}", f"http://{host}{bare}", url]
    for candidate in candidates:
        api = (
            "https://archive.org/wayback/available?timestamp="
            + WAYBACK_TIMESTAMP
            + "&url="
            + quote(candidate, safe="")
        )
        request = Request(api, headers={"User-Agent": UA, "Accept": "application/json"})
        try:
            with urlopen(request, timeout=20) as response:
                data = json.loads(response.read().decode())
        except (HTTPError, URLError, TimeoutError, OSError, json.JSONDecodeError):
            continue
        snap = (data.get("archived_snapshots") or {}).get("closest") or {}
        found = snap.get("url") or ""
        timestamp = str(snap.get("timestamp") or "")
        if not snap.get("available") or str(snap.get("status")) != "200":
            continue
        if not timestamp or int(timestamp[:4]) > 2012:
            continue
        return found.replace("http://web.archive.org/", "https://web.archive.org/", 1)
    return None


def fetch(url: str) -> tuple[int, str]:
    request = Request(url, headers={"User-Agent": UA, "Accept": "text/html"})
    with urlopen(request, timeout=12) as response:
        return response.status, response.geturl()


def resolve(url: str) -> str | None:
    """Return a better URL when the stored link redirects or 404s onto one."""
    host = (urlparse(url).hostname or "").lower().removeprefix("www.")
    if host in DEAD_HOSTS:
        archived = wayback_snapshot(url)
        if archived and archived.rstrip("/") != url.rstrip("/"):
            return archived
        return None
    status = 0
    final = url
    try:
        status, final = fetch(url)
    except HTTPError as error:
        if error.code not in {404, 410}:
            return None
        host = urlparse(url).hostname or ""
        alternate = with_www(url, host.startswith("www."))
        if not alternate:
            return None
        try:
            status, final = fetch(alternate)
        except (HTTPError, URLError, TimeoutError, OSError):
            return None
    except (URLError, TimeoutError, OSError):
        return None
    if status >= 400:
        return None
    original = urlparse(url)
    landed = urlparse(final)
    if registrable(original.hostname or "") != registrable(landed.hostname or ""):
        return None
    if landed.path in ("", "/"):
        return None
    cleaned = final.split("#", 1)[0]
    if cleaned.rstrip("/") == url.rstrip("/"):
        return None
    return cleaned


def main() -> None:
    db = sqlite3.connect(DB)
    rows = db.execute(
        "SELECT id, review_url FROM reviews WHERE review_url IS NOT NULL AND review_url != ''"
    ).fetchall()
    print(f"checking {len(rows)} links", flush=True)
    updates: list[tuple[str, int]] = []
    rewritten = 0
    done = 0
    with ThreadPoolExecutor(max_workers=16) as pool:
        futures = {pool.submit(resolve, url): rid for rid, url in rows}
        for future in as_completed(futures):
            done += 1
            rid = futures[future]
            try:
                found = future.result()
            except Exception:
                found = None
            if found:
                updates.append((found, rid))
                rewritten += 1
            if done % 200 == 0 and updates:
                db.executemany("UPDATE reviews SET review_url=? WHERE id=?", updates)
                db.commit()
                print(f"checked {done} rewritten {rewritten}", flush=True)
                updates.clear()
    if updates:
        db.executemany("UPDATE reviews SET review_url=? WHERE id=?", updates)
        db.commit()
    print(f"checked {done} done", flush=True)


if __name__ == "__main__":
    main()
