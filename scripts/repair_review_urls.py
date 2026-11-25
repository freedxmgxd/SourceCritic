#!/usr/bin/env python3
"""Follow review links and store the address that actually opens.

Same-site redirects are saved. A link that leaves the outlet's site is left
alone. A 404 is retried with and without www.
"""

from __future__ import annotations

import sqlite3
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse, urlunparse
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


def fetch(url: str) -> tuple[int, str]:
    request = Request(url, headers={"User-Agent": UA, "Accept": "text/html"})
    with urlopen(request, timeout=12) as response:
        return response.status, response.geturl()


def resolve(url: str) -> str | None:
    """Return a better URL when the stored link redirects or 404s onto one."""
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
