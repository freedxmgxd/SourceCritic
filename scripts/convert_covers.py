#!/usr/bin/env python3
"""Write 112px and 400px WebP/AVIF covers next to the source art.

Thumbnails are re-encoded until they are under 20 KB. Sources are the
original jpg/png/webp files whose names do not already end in -112/-400.
"""

from __future__ import annotations

import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
COVERS = ROOT / "static" / "covers"
SIZES = (112, 400)
THUMB_BUDGET = 20_000


def sources() -> list[Path]:
    found: list[Path] = []
    if not COVERS.is_dir():
        return found
    for path in sorted(COVERS.iterdir()):
        if not path.is_file():
            continue
        if path.suffix.lower() not in {".jpg", ".jpeg", ".png", ".webp"}:
            continue
        stem = path.stem
        if stem.endswith("-112") or stem.endswith("-400"):
            continue
        found.append(path)
    return found


def encode(path: Path) -> str:
    with Image.open(path) as image:
        image = image.convert("RGB")
        width, height = image.size
        for size in SIZES:
            if width <= 0 or height <= 0:
                continue
            target_h = max(1, round(height * (size / width)))
            resized = image.resize((size, target_h), Image.Resampling.LANCZOS)
            webp = COVERS / f"{path.stem}-{size}.webp"
            avif = COVERS / f"{path.stem}-{size}.avif"
            quality = 62 if size == 112 else 74
            resized.save(webp, "WEBP", quality=quality, method=6)
            if size == 112:
                while webp.stat().st_size > THUMB_BUDGET and quality > 28:
                    quality -= 8
                    resized.save(webp, "WEBP", quality=quality, method=6)
            avif_q = 36 if size == 112 else 48
            try:
                resized.save(avif, "AVIF", quality=avif_q)
                if size == 112:
                    while avif.stat().st_size > THUMB_BUDGET and avif_q > 18:
                        avif_q -= 6
                        resized.save(avif, "AVIF", quality=avif_q)
            except Exception as error:
                print(f"avif skip {path.name}: {error}")
    return path.name


def main() -> int:
    jobs = sources()
    if not jobs:
        print("no cover sources")
        return 0
    done = 0
    with ThreadPoolExecutor(max_workers=8) as pool:
        futures = [pool.submit(encode, path) for path in jobs]
        for future in as_completed(futures):
            future.result()
            done += 1
            if done % 100 == 0 or done == len(jobs):
                print(f"{done}/{len(jobs)}", flush=True)
    over = [
        path
        for path in COVERS.glob("*-112.webp")
        if path.stat().st_size > THUMB_BUDGET
    ]
    print(f"thumbs over 20KB: {len(over)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
