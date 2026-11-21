#!/usr/bin/env python3
"""
Periodic SourceCritic ops pipeline (catalog is already the source of truth):

  scrape (outlet sites → MC → OC) → outlet fill pass → dedupe → reseed live DB

Does not discover or grow catalog_games / catalog_outlets — add games there
separately when needed.
"""

from __future__ import annotations

import argparse
import fcntl
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "scripts"
DATA = ROOT / "data"
DEFAULT_LOCK = Path("/tmp/sourcecritic_pipeline.lock")
DEFAULT_LOG = Path("/tmp/sc_pipeline.log")


def log(message: str, log_path: Path | None = None) -> None:
    line = message.rstrip()
    print(line, flush=True)
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        with log_path.open("a", encoding="utf-8") as handle:
            handle.write(line + "\n")


def run(args: list[str], *, log_path: Path) -> int:
    log(f"$ {' '.join(args)}", log_path)
    env = os.environ.copy()
    env.setdefault("PYTHONUNBUFFERED", "1")
    env["SOURCECRITIC_PIPELINE_CHILD"] = "1"
    with log_path.open("a", encoding="utf-8") as handle:
        proc = subprocess.run(
            args,
            cwd=str(ROOT),
            env=env,
            stdout=handle,
            stderr=subprocess.STDOUT,
            check=False,
        )
    log(f"exit={proc.returncode}", log_path)
    return proc.returncode


def acquire_lock(lock_path: Path, block: bool):
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    handle = lock_path.open("w")
    flags = fcntl.LOCK_EX
    if not block:
        flags |= fcntl.LOCK_NB
    try:
        fcntl.flock(handle.fileno(), flags)
    except BlockingIOError as error:
        handle.close()
        raise SystemExit(
            f"pipeline already running (lock={lock_path}); use --wait to block"
        ) from error
    handle.write(f"pid={os.getpid()}\nstarted={time.strftime('%Y-%m-%dT%H:%M:%S')}\n")
    handle.flush()
    return handle


def phase_scrape(
    log_path: Path,
    *,
    only_missing: bool,
    with_outlets: bool,
    stale_days: float,
) -> int:
    cmd = [
        sys.executable,
        str(SCRIPTS / "scrape_catalog.py"),
        "--stale-days",
        str(stale_days),
    ]
    if only_missing:
        cmd.append("--only-missing")
    if with_outlets:
        cmd.append("--with-outlets")
    else:
        cmd.append("--skip-outlets")
    return run(cmd, log_path=log_path)


def phase_outlets(log_path: Path, *, stale_days: float) -> int:
    return run(
        [
            sys.executable,
            str(SCRIPTS / "scrape_catalog.py"),
            "--outlets-only",
            "--stale-days",
            str(stale_days),
        ],
        log_path=log_path,
    )


def phase_dedupe(log_path: Path) -> int:
    return run(
        [
            sys.executable,
            str(SCRIPTS / "scrape_catalog.py"),
            "--dedupe-only",
        ],
        log_path=log_path,
    )


def phase_reseed(log_path: Path, url: str) -> int:
    seed = DATA / "seed_reviews.json"
    if not seed.exists():
        log(f"reseed skip: missing {seed}", log_path)
        return 0
    live = Path("/tmp/seed_live.json")
    live.write_bytes(seed.read_bytes())
    return run(
        [
            sys.executable,
            str(SCRIPTS / "seed_examples.py"),
            "--seed",
            str(live),
            "--url",
            url,
        ],
        log_path=log_path,
    )


def install_cron(interval_hours: int, python: str) -> None:
    line = (
        f"0 */{interval_hours} * * * cd {ROOT} && "
        f"{python} scripts/run_pipeline.py --all "
        f">>/tmp/sc_pipeline.log 2>&1"
    )
    marker = "scripts/run_pipeline.py --all"
    if not any(
        (Path(p) / "crontab").exists() for p in os.environ.get("PATH", "").split(":")
    ):
        raise SystemExit(
            "crontab not found on PATH; use --install-systemd HOURS instead"
        )
    try:
        current = subprocess.check_output(["crontab", "-l"], text=True)
    except subprocess.CalledProcessError:
        current = ""
    lines = [ln for ln in current.splitlines() if marker not in ln and ln.strip()]
    lines.append(line)
    body = "\n".join(lines).rstrip() + "\n"
    proc = subprocess.run(["crontab", "-"], input=body, text=True, check=False)
    if proc.returncode != 0:
        raise SystemExit("failed to install crontab")
    print(f"installed cron:\n{line}")


def install_systemd(interval_hours: int, python: str) -> None:
    unit_dir = Path.home() / ".config" / "systemd" / "user"
    unit_dir.mkdir(parents=True, exist_ok=True)
    service = unit_dir / "sourcecritic-pipeline.service"
    timer = unit_dir / "sourcecritic-pipeline.timer"
    service.write_text(
        "\n".join(
            [
                "[Unit]",
                "Description=SourceCritic scrape/outlets/reseed pipeline",
                "",
                "[Service]",
                "Type=oneshot",
                f"WorkingDirectory={ROOT}",
                f"ExecStart={python} {SCRIPTS / 'run_pipeline.py'} --all",
                "Nice=10",
                "",
            ]
        )
        + "\n"
    )
    timer.write_text(
        "\n".join(
            [
                "[Unit]",
                "Description=Run SourceCritic pipeline on a schedule",
                "",
                "[Timer]",
                "OnBootSec=5m",
                f"OnUnitActiveSec={interval_hours}h",
                "Persistent=true",
                "Unit=sourcecritic-pipeline.service",
                "",
                "[Install]",
                "WantedBy=timers.target",
                "",
            ]
        )
        + "\n"
    )
    cmds = [
        ["systemctl", "--user", "daemon-reload"],
        ["systemctl", "--user", "enable", "--now", "sourcecritic-pipeline.timer"],
    ]
    for cmd in cmds:
        proc = subprocess.run(cmd, check=False, capture_output=True, text=True)
        if proc.returncode != 0:
            raise SystemExit(f"failed: {' '.join(cmd)}\n{proc.stderr or proc.stdout}")
    status = subprocess.check_output(
        ["systemctl", "--user", "list-timers", "sourcecritic-pipeline.timer", "--no-pager"],
        text=True,
    )
    print(f"installed systemd user timer ({interval_hours}h):\n{service}\n{timer}\n{status}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--all",
        action="store_true",
        help="scrape (outlet→MC→OC) + outlet fill + dedupe + reseed",
    )
    parser.add_argument(
        "--scrape",
        action="store_true",
        help="scrape_catalog (outlet sites first, then MC/OC)",
    )
    parser.add_argument(
        "--skip-outlets-during-scrape",
        action="store_true",
        help="Aggregator-only scrape (skip direct outlet sites)",
    )
    parser.add_argument(
        "--with-outlets-during-scrape",
        action="store_true",
        help="Deprecated: outlets are on by default during scrape",
    )
    parser.add_argument("--outlets", action="store_true", help="scrape_catalog --outlets-only")
    parser.add_argument(
        "--dedupe",
        action="store_true",
        help="Collapse duplicate critic rows in seed (no network)",
    )
    parser.add_argument("--reseed", action="store_true", help="POST seed into live server")
    parser.add_argument("--only-missing", action="store_true", help="Scrape only catalog∉seed")
    parser.add_argument(
        "--stale-days",
        type=float,
        default=7.0,
        help="Re-scrape games not scraped within N days (0 disables; default 7)",
    )
    parser.add_argument("--reseed-url", default="http://127.0.0.1:3000/reviews")
    parser.add_argument("--lock", type=Path, default=DEFAULT_LOCK)
    parser.add_argument("--log", type=Path, default=DEFAULT_LOG)
    parser.add_argument("--wait", action="store_true", help="Block on lock instead of exiting")
    parser.add_argument(
        "--install-cron",
        type=int,
        metavar="HOURS",
        help="Install/replace user crontab entry every N hours and exit",
    )
    parser.add_argument(
        "--install-systemd",
        type=int,
        metavar="HOURS",
        help="Install systemd --user timer every N hours and exit",
    )
    args = parser.parse_args()

    if args.install_cron is not None:
        if args.install_cron < 1:
            raise SystemExit("--install-cron HOURS must be >= 1")
        install_cron(args.install_cron, sys.executable)
        return

    if args.install_systemd is not None:
        if args.install_systemd < 1:
            raise SystemExit("--install-systemd HOURS must be >= 1")
        install_systemd(args.install_systemd, sys.executable)
        return

    if args.all:
        args.scrape = True
        args.outlets = True
        args.dedupe = True
        args.reseed = True

    if not any([args.scrape, args.outlets, args.dedupe, args.reseed]):
        parser.error("pass --all or one of --scrape/--outlets/--dedupe/--reseed")

    lock_handle = acquire_lock(args.lock, block=args.wait)
    started = time.strftime("%Y-%m-%dT%H:%M:%S")
    log(f"===== pipeline start {started} =====", args.log)
    try:
        if args.scrape:
            # Outlets on by default (priority: outlet → MC → OC). Opt out explicitly.
            with_outlets = not args.skip_outlets_during_scrape
            log(
                f"--- scrape (outlet→MC→OC, outlets={'on' if with_outlets else 'off'}) ---",
                args.log,
            )
            code = phase_scrape(
                args.log,
                only_missing=args.only_missing,
                with_outlets=with_outlets,
                stale_days=args.stale_days,
            )
            if code != 0:
                raise SystemExit(code)
            if args.reseed:
                phase_reseed(args.log, args.reseed_url)

        if args.outlets:
            log("--- outlet fill pass ---", args.log)
            code = phase_outlets(args.log, stale_days=args.stale_days)
            if code != 0:
                raise SystemExit(code)

        if args.dedupe:
            log("--- dedupe critic rows ---", args.log)
            code = phase_dedupe(args.log)
            if code != 0:
                raise SystemExit(code)

        if args.reseed:
            log("--- reseed ---", args.log)
            code = phase_reseed(args.log, args.reseed_url)
            if code != 0:
                raise SystemExit(code)

        log(f"===== pipeline done {time.strftime('%Y-%m-%dT%H:%M:%S')} =====", args.log)
    finally:
        try:
            fcntl.flock(lock_handle.fileno(), fcntl.LOCK_UN)
        finally:
            lock_handle.close()


if __name__ == "__main__":
    main()
