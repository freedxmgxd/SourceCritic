//! HTML pages. Each URL returns a finished document.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use serde::Deserialize;

use super::{AppState, lock};
use crate::db::{GameView, OutletView, ReviewView};

const CSS: &str = r#"
:root {
  --bg: #f0ebe8;
  --bg-deep: #e2d8d4;
  --ink: #1a1218;
  --muted: #6a5963;
  --line: #d0c3c8;
  --panel: #fdfbfa;
  --accent: #8b1e4a;
  --accent-soft: #f3e0e8;
  --accent-mid: #e4c4d0;
  --danger: #c2410c;
  --cmp-high: #7a3d96;
  --cmp-low: #c24747;
  --code-bg: #1a1218;
  --code-ink: #eddfe6;
  --surface-hover: #f4ecef;
  --surface-muted: #f6f0f2;
  --text-body: #2a1f26;
}

* { box-sizing: border-box; }

body {
  margin: 0;
  min-height: 100vh;
  color: var(--ink);
  font: 16px/1.55 "IBM Plex Sans", "Segoe UI", sans-serif;
  background:
    radial-gradient(circle at 12% 0%, #faf6f5 0%, transparent 42%),
    radial-gradient(circle at 88% 8%, var(--accent-soft) 0%, transparent 36%),
    linear-gradient(180deg, var(--bg) 0%, var(--bg-deep) 100%);
}

main {
  width: min(860px, calc(100% - 32px));
  margin: 28px auto 64px;
  padding: 28px 32px 40px;
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 18px;
  box-shadow: 0 18px 40px rgba(26, 18, 24, 0.08);
}

header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  padding-bottom: 18px;
  border-bottom: 1px solid var(--line);
}

.brand {
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.45rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  text-decoration: none;
  color: var(--ink);
}

nav {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

nav a {
  padding: 7px 12px;
  border-radius: 999px;
  color: var(--muted);
  text-decoration: none;
  font-size: 0.92rem;
}

nav a:hover { color: var(--ink); background: var(--surface-hover); }

nav a[aria-current="page"] {
  color: var(--accent);
  background: var(--accent-soft);
  font-weight: 600;
}

.page {
  margin: 28px 0 10px;
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.85rem;
  font-weight: 600;
  letter-spacing: -0.03em;
  line-height: 1.15;
}

.lead, .muted {
  margin: 0 0 18px;
  color: var(--muted);
  max-width: 40rem;
}

.row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 28px;
  padding: 14px 0;
  border-bottom: 1px solid var(--line);
  text-decoration: none;
  color: inherit;
}

/* `display: flex` above would otherwise override the hidden attribute. */
.row[hidden],
#game-empty[hidden] {
  display: none !important;
}

.row:first-of-type { border-top: 1px solid var(--line); }
.row:hover { background: linear-gradient(90deg, transparent, var(--surface-hover) 40%, transparent); }
.row .meta { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; }
.row .title-block { flex: 1 1 auto; min-width: 0; }
.row .meta > span { min-width: 0; }
.row strong {
  display: block;
  font-size: 1.05rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row small { display: block; margin-top: 2px; color: var(--muted); font-size: 0.9rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.cover {
  width: 56px;
  height: 78px;
  object-fit: cover;
  border-radius: 6px;
  background: var(--accent-mid);
  flex: none;
  box-shadow: 0 4px 12px rgba(26, 18, 24, 0.12);
}

.cover-lg {
  width: 140px;
  height: 198px;
  object-fit: cover;
  border-radius: 10px;
  background: var(--accent-mid);
  box-shadow: 0 10px 24px rgba(26, 18, 24, 0.14);
}

.game-hero {
  display: flex;
  gap: 22px;
  align-items: flex-start;
  margin: 22px 0 8px;
}

.game-hero .text { min-width: 0; flex: 1; }
.game-hero .page { margin: 0 0 12px; }

.score-hero {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 10px 18px;
  margin: 0 0 16px;
  padding: 16px 18px;
  border: 1px solid color-mix(in srgb, var(--accent) 22%, var(--line));
  border-radius: 14px;
  background:
    linear-gradient(135deg, #faf0f4 0%, #f3e0e8 55%, #faf6f5 100%);
}

.score-hero-value {
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: clamp(2.6rem, 6vw, 3.4rem);
  font-weight: 600;
  line-height: 0.95;
  letter-spacing: -0.04em;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
}

.score-hero-copy {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  padding-bottom: 4px;
}

.score-hero-label {
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 650;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.score-hero-meta {
  color: var(--muted);
  font-size: 0.92rem;
}

.synopsis {
  margin: 0 0 14px;
  max-width: 42rem;
  color: var(--text-body);
  font-size: 1.02rem;
  line-height: 1.55;
}

.reviews { margin-top: 22px; }
.reviews h2 {
  margin: 0 0 10px;
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.15rem;
}
.port-filter {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin: 0 0 12px;
}
.port-filter button {
  appearance: none;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  border: 1px solid var(--line);
  background: transparent;
  color: var(--muted);
  border-radius: 999px;
  padding: 4px 10px 4px 12px;
  font: inherit;
  font-size: 0.88rem;
  cursor: pointer;
}
.port-filter .port-score {
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--accent);
}
.port-filter button[aria-pressed="true"] {
  border-color: var(--text);
  color: var(--text);
  background: color-mix(in srgb, var(--text) 8%, transparent);
}
.port-filter-meta {
  margin: 0 0 10px;
  color: var(--muted);
  font-size: 0.92rem;
}
.review {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px 16px;
  padding: 14px 0;
  border-bottom: 1px solid var(--line);
}
.review:first-of-type { border-top: 1px solid var(--line); }
.review strong { display: block; font-size: 1.02rem; }
.review small {
  display: block;
  margin-top: 3px;
  color: var(--muted);
  font-size: 0.9rem;
}
.review .z,
.review .mapped {
  color: var(--accent);
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.15rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  text-align: right;
}
.review .excerpt {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--text-body);
  font-size: 0.95rem;
  line-height: 1.5;
  max-width: 42rem;
}
.review .source {
  grid-column: 1 / -1;
  font-size: 0.88rem;
}
.review strong a { color: inherit; text-decoration: none; }
.review strong a:hover { color: var(--accent); text-decoration: underline; }
.review .source a { color: var(--accent); }

@media (max-width: 640px) {
  .game-hero { flex-direction: column; }
  .cover-lg { width: 110px; height: 156px; }
}

.score {
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.8rem;
  font-weight: 600;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  text-align: right;
  flex: none;
  margin-left: 12px;
  padding-left: 8px;
}

.score-compact {
  display: inline-flex;
  justify-content: flex-end;
  min-width: 7.5ch;
}

.score-compact .frac {
  display: inline-block;
  max-width: 0;
  overflow: hidden;
  opacity: 0;
  vertical-align: bottom;
  transition: max-width 0.22s ease, opacity 0.18s ease;
}

.score-compact:hover .frac,
.score-compact:focus-visible .frac {
  max-width: 5ch;
  opacity: 1;
}

.walk {
  margin-top: 8px;
  border: 1px solid var(--line);
  border-radius: 16px;
  overflow: hidden;
  background: #fff;
}

.step {
  padding: 22px 22px 20px;
  border-top: 1px solid var(--line);
}

.step:first-child { border-top: 0; }

.step-label {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 8px;
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 650;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.step h2 {
  margin: 0 0 8px;
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.2rem;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.step p {
  margin: 0 0 14px;
  max-width: 42rem;
  color: var(--text-body);
}

.panel {
  margin-top: 4px;
  border: 1px solid var(--line);
  border-radius: 14px;
  overflow: hidden;
  background: var(--surface-muted);
}

.panel .block {
  padding: 14px 16px;
  overflow: auto;
}

.panel .block + .block {
  border-top: 1px solid var(--line);
}

.block .tag {
  display: block;
  margin-bottom: 8px;
  font-size: 0.72rem;
  font-weight: 650;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.math {
  color: #3d1528;
  background: linear-gradient(180deg, #faf0f4 0%, #f3e0e8 100%);
}

.math .tag { color: var(--accent); }

.math .tex {
  overflow-x: auto;
  font-size: 1.02rem;
}

.math .katex-display { margin: 0.55em 0; }

.math .katex-display:first-child { margin-top: 0.15em; }

.math .katex-display:last-child { margin-bottom: 0.15em; }

.code {
  background: var(--code-bg);
  color: var(--code-ink);
  font: 0.86rem/1.55 "IBM Plex Mono", ui-monospace, monospace;
}

.code .tag { color: #d4a0b8; }

.code code {
  display: block;
  white-space: pre;
  font: inherit;
}

.stats {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
  margin: 16px 0 22px;
}

.stat {
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}

.stat .label {
  display: block;
  margin-bottom: 4px;
  color: var(--muted);
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.stat .value {
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.25rem;
  font-variant-numeric: tabular-nums;
}

.empty {
  margin: 18px 0 0;
  padding: 22px;
  border: 1px dashed var(--line);
  border-radius: 14px;
  color: var(--muted);
  text-align: center;
}

.browse-controls {
  margin: 4px 0 14px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-muted);
}

.filters {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
}

.filters .filter-search {
  grid-column: 1 / -1;
}

.filters label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--muted);
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.filters select,
.filters input[type="search"] {
  font: inherit;
  text-transform: none;
  letter-spacing: 0;
  color: var(--ink);
  border: 1px solid var(--line);
  border-radius: 10px;
  background: white;
  padding: 8px 10px;
}

.filters select { cursor: pointer; }

.filters input[type="search"] {
  width: 100%;
}

.filters input[type="search"]::placeholder {
  color: color-mix(in srgb, var(--muted) 75%, white);
}

.filters select:focus-visible,
.filters input[type="search"]:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--accent) 45%, white);
  outline-offset: 2px;
}

.browse-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 16px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}

.sort {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin: 4px 0 16px;
  color: var(--muted);
  font-size: 0.9rem;
}

.browse-controls .sort { margin: 0; }

.sort .label {
  margin-right: 4px;
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.sort a,
nav.sort button[data-sort] {
  appearance: none;
  display: inline-flex;
  align-items: center;
  padding: 5px 11px;
  border-radius: 999px;
  color: var(--muted);
  text-decoration: none;
  border: 1px solid var(--line);
  background: white;
  font: inherit;
  cursor: pointer;
  line-height: 1.2;
}

.sort a:hover,
nav.sort button[data-sort]:hover {
  color: var(--ink);
  background: var(--surface-hover);
}

.sort a[aria-current="true"],
nav.sort button[data-sort][aria-current="true"] {
  color: var(--accent);
  background: var(--accent-soft);
  border-color: color-mix(in srgb, var(--accent) 22%, white);
  font-weight: 600;
}

.sort a:focus-visible,
nav.sort button[data-sort]:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--accent) 45%, white);
  outline-offset: 2px;
}

.filter-reset {
  appearance: none;
  border: 0;
  background: transparent;
  color: var(--accent);
  font: inherit;
  font-size: 0.88rem;
  font-weight: 600;
  cursor: pointer;
  padding: 4px 2px;
  text-decoration: underline;
  text-underline-offset: 3px;
}

.filter-reset:hover { color: var(--ink); }

.filter-reset[hidden] { display: none; }

.filter-meta {
  margin: -6px 0 14px;
  color: var(--muted);
  font-size: 0.9rem;
}

.row small .cmp-high,
.row small .cmp-low {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}

.row small .cmp-high { color: var(--cmp-high); }
.row small .cmp-low { color: var(--cmp-low); }

.about-block {
  margin: 22px 0 0;
  padding: 22px 22px 20px;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: #fff;
}

.about-block > h2 {
  margin: 0 0 8px;
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.25rem;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.about-block > .lead {
  margin-bottom: 14px;
}

.faq {
  border-top: 1px solid var(--line);
}

.faq details {
  border-bottom: 1px solid var(--line);
}

.faq details:last-child { border-bottom: 0; }

.faq summary {
  cursor: pointer;
  list-style: none;
  padding: 14px 0;
  font-weight: 600;
  color: var(--ink);
}

.faq summary::-webkit-details-marker { display: none; }

.faq summary::before {
  content: "+";
  display: inline-block;
  width: 1.1em;
  margin-right: 6px;
  color: var(--accent);
  font-weight: 650;
}

.faq details[open] summary::before { content: "–"; }

.faq details p {
  margin: 0 0 14px;
  max-width: 42rem;
  color: var(--text-body);
}

.about-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 4px;
}

.about-actions a {
  display: inline-flex;
  align-items: center;
  padding: 9px 14px;
  border-radius: 999px;
  text-decoration: none;
  font-weight: 600;
  font-size: 0.92rem;
  color: var(--accent);
  background: var(--accent-soft);
  border: 1px solid color-mix(in srgb, var(--accent) 22%, white);
}

.about-actions a:hover { background: var(--accent-mid); }

.about-actions a.secondary {
  color: var(--ink);
  background: var(--surface-hover);
  border-color: var(--line);
  font-weight: 500;
}

.about-actions a.secondary:hover { background: var(--accent-mid); }

/*
  Main column keeps a fixed max width. The ad uses leftover space on the right
  only — it never overlays main and never shrinks it.
*/
.page-shell {
  display: grid;
  grid-template-columns: 1fr min(860px, 100%) minmax(120px, 300px);
  column-gap: 20px;
  align-items: start;
  width: min(1180px, calc(100% - 32px));
  margin: 0 auto;
}

.page-shell > main {
  grid-column: 2;
  width: 100%;
  max-width: 860px;
  margin: 28px 0 64px;
}

.site-ad-rail {
  grid-column: 3;
  position: sticky;
  top: 96px;
  width: 100%;
  max-width: 300px;
  margin: 96px 0 48px;
  padding: 0;
  align-self: start;
  /* Stay in its own column — never cover main. */
  z-index: 0;
}

.site-ad-label {
  display: block;
  margin: 0 0 6px;
  color: var(--muted);
  font-size: 0.68rem;
  font-weight: 500;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  opacity: 0.55;
}

.site-ad-slot {
  display: block;
  width: 100%;
  min-height: 600px;
  overflow: hidden;
  border-radius: 10px;
  background: transparent;
  border: 0;
}

.ad-home-end { display: none; }
.ad-unit { margin: 16px 0; }
.ad-unit.rect .site-ad-slot { min-height: 280px; }
.ad-unit.is-empty,
.ad-unit[hidden] { display: none !important; }

.pager {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 12px;
  margin: 18px 0 0;
}
.pager a {
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid var(--line);
  color: var(--ink);
  text-decoration: none;
  background: white;
}
.pager a:hover { background: var(--surface-hover); }

.provisional {
  display: inline-block;
  margin-left: 6px;
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 0.75rem;
  font-weight: 650;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.buy-block {
  margin: 22px 0 8px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-muted);
}
.buy-block h2 {
  margin: 0 0 6px;
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.15rem;
}
.buy-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 10px; }
.buy-actions a {
  display: inline-flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: white;
  color: var(--ink);
  text-decoration: none;
  font-weight: 600;
}
.buy-actions a small { font-weight: 500; color: var(--muted); }
.buy-note { margin: 8px 0 0; color: var(--muted); font-size: 0.88rem; }

.newsletter {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin: 8px 0 0;
}
.newsletter input[type="email"] {
  font: inherit;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 10px;
  min-width: min(260px, 100%);
}
.newsletter button {
  appearance: none;
  border: 0;
  border-radius: 999px;
  padding: 8px 14px;
  background: var(--accent);
  color: white;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}

.site-footer {
  margin-top: 28px;
  padding-top: 16px;
  border-top: 1px solid var(--line);
  color: var(--muted);
  font-size: 0.9rem;
}
.site-footer nav { margin-bottom: 8px; }
.site-footer a { color: var(--accent); }

.cookie-banner {
  position: static;
  z-index: 1;
  display: flex;
  flex-wrap: wrap;
  gap: 10px 14px;
  align-items: center;
  justify-content: space-between;
  margin: 18px 0 0;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: #fff;
}
.cookie-banner[hidden] { display: none !important; }
.cookie-banner p { margin: 0; max-width: 40rem; color: var(--text-body); font-size: 0.92rem; }
.cookie-banner .cookie-actions { display: flex; gap: 8px; }
.cookie-banner button {
  appearance: none;
  border-radius: 999px;
  padding: 7px 12px;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.cookie-banner .accept { border: 0; background: var(--accent); color: white; }
.cookie-banner .reject { border: 1px solid var(--line); background: white; color: var(--ink); }

.site-ad-slot .adsbygoogle,
.site-ad-slot > * {
  width: 100% !important;
  max-width: 100%;
}

/* Not enough room for main + side → hide the ad, keep main centered. */
@media (max-width: 1099px) {
  .page-shell {
    display: block;
    width: min(860px, calc(100% - 32px));
  }
  .page-shell > main {
    margin: 28px auto 64px;
  }
  .site-ad-rail,
  .ad-inlist {
    display: none !important;
  }
  .ad-home-end { display: block !important; }
}

@media (max-width: 720px) {
  main { padding: 22px 18px 32px; }
  header { flex-direction: column; align-items: flex-start; }
  .stats { grid-template-columns: 1fr 1fr; }
  .filters { grid-template-columns: 1fr; }
  .browse-toolbar { align-items: flex-start; }
  .ad-inlist { display: none !important; }
  .ad-home-end { display: block !important; }
}

@media (min-width: 1100px) {
  .ad-home-end { display: none !important; }
}

@media (max-width: 600px) {
  .row { gap: 10px; }
  .row-secondary { display: none !important; }
  .score { font-size: 1.4rem; margin-left: 6px; padding-left: 0; }
  .cover { width: 44px; height: 62px; }
}

.crypto-list { display: grid; gap: 12px; margin: 16px 0 0; }
.crypto-row {
  display: grid;
  gap: 6px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.crypto-row .coin { font-weight: 600; color: var(--ink); }
.crypto-row code {
  display: block;
  overflow-wrap: anywhere;
  font: 13px/1.45 "IBM Plex Mono", ui-monospace, monospace;
  color: var(--text-body);
}
.crypto-row .crypto-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.crypto-row button, .about-actions button {
  appearance: none;
  border: 1px solid var(--line);
  background: var(--panel);
  color: var(--ink);
  border-radius: 999px;
  padding: 6px 12px;
  font: 500 13px/1.2 "IBM Plex Sans", sans-serif;
  cursor: pointer;
}
.crypto-row button:hover, .about-actions button:hover { background: var(--surface-hover); }
"#;

/// Public contact address shown on /about. `None` until an inbox exists.
const CONTACT_EMAIL: Option<&str> = Some("joeltondbrandao@freedxm.com");
/// Public GitHub profile linked from /about.
const GITHUB_URL: Option<&str> = Some("https://github.com/freedxmgxd");
/// Donation URL (Ko-fi, PayPal, Pix page, etc.). `None` until a page exists.
const DONATE_URL: Option<&str> = None;
/// Optional crypto donation addresses shown on /about. Fill in when wallets are ready.
const DONATE_BTC: Option<&str> = None;
const DONATE_ETH: Option<&str> = None;
const DONATE_XMR: Option<&str> = None;
/// AdSense publisher client (`ca-pub-…`). Loaded only after consent. Auto ads stay off.
const ADSENSE_CLIENT: Option<&str> = Some("ca-pub-5490722539415533");
const GA_MEASUREMENT_ID: Option<&str> = None;
const PAGE_SIZE: usize = 50;
const DEFAULT_MIN_REVIEWS: i64 = 10;
/// Side-rail display unit only. Fills the right column; main column width is unchanged.
/// Env `SOURCECRITIC_AD_HTML` overrides. At most once per browser hour.
#[allow(dead_code)]
const AD_HTML: Option<&str> = Some(
    r#"<ins class="adsbygoogle"
     style="display:block;width:100%;min-height:600px"
     data-ad-client="ca-pub-5490722539415533"
     data-ad-slot="8380670310"
     data-ad-format="vertical"
     data-full-width-responsive="false"></ins>"#,
);
/// Canonical public origin for absolute links in the static export.
const SITE_ORIGIN: &str = "https://sourcecritic.freedxm.com";

const SITE_JS: &str = r#"
(function () {
  function num(el, key) {
    var v = el.getAttribute(key);
    return v == null || v === "" ? 0 : Number(v);
  }
  function text(el, key) {
    return (el.getAttribute(key) || "").toLowerCase();
  }
  function setCurrent(nav, key) {
    if (!nav) return;
    nav.querySelectorAll("[data-sort]").forEach(function (btn) {
      if (btn.getAttribute("data-sort") === key) btn.setAttribute("aria-current", "true");
      else btn.removeAttribute("aria-current");
    });
  }
  function sortChildren(list, key, mode) {
    var rows = Array.prototype.slice.call(list.children);
    rows.sort(function (a, b) {
      if (mode === "title" || mode === "name") {
        return text(a, "data-title").localeCompare(text(b, "data-title"));
      }
      if (mode === "reviews") return num(b, "data-reviews") - num(a, "data-reviews");
      if (mode === "mean") return num(b, "data-mean") - num(a, "data-mean");
      if (mode === "mu") return num(b, "data-mu") - num(a, "data-mu");
      if (mode === "mu_prime") return num(b, "data-mu-prime") - num(a, "data-mu-prime");
      if (mode === "z") return num(b, "data-z") - num(a, "data-z");
      if (mode === "score") return num(b, "data-score") - num(a, "data-score");
      return 0;
    });
    rows.forEach(function (row) { list.appendChild(row); });
    setCurrent(list.parentElement.querySelector(".sort"), key);
    if (list.id === "game-list") {
      list.querySelectorAll(".row").forEach(function (row) {
        var scoreEl = row.querySelector("[data-score-display]");
        var meanEl = row.querySelector("[data-mean-display]");
        var meta = row.querySelector("[data-score-meta]");
        if (!scoreEl || !meanEl || !meta) return;
        var showMean = key === "mean";
        scoreEl.hidden = showMean;
        meanEl.hidden = !showMean;
        var scoreLabel = row.getAttribute("data-score-label") || "";
        var meanLabel = row.getAttribute("data-mean-label") || "";
        // Color encodes score vs mean: red if score > mean, purple if score < mean.
        meta.innerHTML = showMean
          ? ("score " + cmpNumHtml(scoreLabel, num(row, "data-score"), num(row, "data-mean")))
          : ("simple mean " + cmpNumHtml(meanLabel, num(row, "data-score"), num(row, "data-mean")));
      });
    }
    if (list.id === "game-reviews" || list.id === "outlet-reviews") {
      var showZ = key === "z";
      list.querySelectorAll(".review").forEach(function (row) {
        var scoreEl = row.querySelector("[data-score-display]");
        var zEl = row.querySelector("[data-z-display]");
        var zMeta = row.querySelector("[data-z-meta]");
        var scoreMeta = row.querySelector("[data-score-meta]");
        if (!scoreEl || !zEl || !zMeta || !scoreMeta) return;
        scoreEl.hidden = showZ;
        zEl.hidden = !showZ;
        zMeta.hidden = showZ;
        scoreMeta.hidden = !showZ;
      });
    }
  }
  function cmpNumHtml(label, score, mean) {
    if (!isFinite(score) || !isFinite(mean) || score === mean) {
      return label;
    }
    var cls = score > mean ? "cmp-low" : "cmp-high";
    return '<span class="' + cls + '">' + label + "</span>";
  }
  function fold(value) {
    return (value || "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  }
  function formState(form) {
    return {
      platform: (form.platform && form.platform.value) || "all",
      year: (form.year && form.year.value) || "all",
      min: String((form.min_reviews && form.min_reviews.value) || "10"),
      q: ((form.q && form.q.value) || "").trim(),
      sort: (form.getAttribute("data-sort") || "score")
    };
  }
  function stateIsDefault(state) {
    return state.platform === "all" && state.year === "all" && state.min === "10" && !state.q && state.sort === "score";
  }
  function filtersActive(form) {
    return !stateIsDefault(formState(form));
  }
  function writeQuery(state) {
    var params = new URLSearchParams();
    if (state.platform !== "all") params.set("platform", state.platform);
    if (state.year !== "all") params.set("year", state.year);
    if (state.min !== "10") params.set("min", state.min);
    if (state.sort && state.sort !== "score") params.set("sort", state.sort);
    if (state.q) params.set("q", state.q);
    var qs = params.toString();
    var path = stateIsDefault(state) ? (location.pathname || "/") : "/";
    history.replaceState(null, "", path + (qs ? "?" + qs : ""));
  }
  function readQuery(form) {
    var params = new URLSearchParams(location.search);
    if (form.platform && params.get("platform")) form.platform.value = params.get("platform");
    if (form.year && params.get("year")) form.year.value = params.get("year");
    var min = params.get("min") || params.get("min_reviews");
    if (form.min_reviews && min) form.min_reviews.value = min;
    if (params.get("q") && form.q) form.q.value = params.get("q");
    var sort = params.get("sort");
    if (sort) form.setAttribute("data-sort", sort);
  }
  function renderClient(games) {
    var list = document.getElementById("game-list");
    var form = document.getElementById("game-filters");
    var meta = document.getElementById("filter-meta");
    var reset = document.getElementById("filter-reset");
    var empty = document.getElementById("game-empty");
    var pager = document.getElementById("pager");
    if (!list || !form) return;
    var state = formState(form);
    var q = fold(state.q);
    var minReviews = Number(state.min || 0);
    var matched = games.filter(function (game) {
      if (state.platform !== "all" && (game.platforms || []).indexOf(state.platform) === -1) return false;
      if (state.year !== "all" && String(game.year || "") !== state.year) return false;
      if (minReviews > 0 && Number(game.reviews || 0) < minReviews) return false;
      if (q && fold(game.title).indexOf(q) === -1) return false;
      return true;
    });
    var sort = state.sort || "score";
    matched.sort(function (a, b) {
      if (sort === "title") return fold(a.title).localeCompare(fold(b.title));
      if (sort === "reviews") return Number(b.reviews) - Number(a.reviews);
      if (sort === "mean") return Number(b.mean) - Number(a.mean);
      return Number(b.score) - Number(a.score);
    });
    var pageSize = 50;
    var params = new URLSearchParams(location.search);
    var page = Math.max(1, Number(params.get("page") || 1));
    var pages = Math.max(1, Math.ceil(matched.length / pageSize));
    if (page > pages) page = pages;
    var slice = matched.slice((page - 1) * pageSize, page * pageSize);
    var html = "";
    slice.forEach(function (game, index) {
      var score = Math.round(Number(game.score) || 0);
      var mean = Math.round(Number(game.mean) || 0);
      var provisional = Number(game.reviews) < 5 ? " <span class=\"provisional\">nota provisória</span>" : "";
      var bits = "";
      if (game.year) bits += game.year;
      if (game.platform_labels) bits += (bits ? " · " : "") + game.platform_labels;
      html += '<a class="row" href="/game/' + game.slug + '/"><span class="meta">' +
        (game.cover ? '<img class="cover" src="' + game.cover + '" alt="Capa de ' + game.title.replace(/"/g, "&quot;") + '" width="112" height="157" loading="lazy">' : "") +
        '<span class="title-block"><strong>' + game.title + '</strong><small><span class="row-count">' +
        game.reviews + ' reviews</span>' + (bits ? '<span class="row-secondary"> · ' + bits + '</span>' : '') +
        '</small></span></span><span class="score">' + score + provisional + '</span></a>';
      if ((index + 1) % 25 === 0) html += window.__scAdInlist || "";
    });
    list.innerHTML = html;
    list.setAttribute("data-client", "1");
    list.setAttribute("data-total", String(matched.length));
    if (meta) {
      var start = matched.length ? (page - 1) * pageSize + 1 : 0;
      var end = Math.min(page * pageSize, matched.length);
      meta.textContent = "Showing " + start + "–" + end + " of " + matched.length + " games.";
    }
    if (empty) empty.hidden = matched.length !== 0;
    if (reset) reset.hidden = !filtersActive(form);
    if (pager) pager.hidden = pages < 2;
  }
  function ensureIndex() {
    if (window.__scGames) return Promise.resolve(window.__scGames);
    if (window.__scGamesLoading) return window.__scGamesLoading;
    window.__scGamesLoading = fetch("/games-index.json")
      .then(function (response) { return response.json(); })
      .then(function (games) {
        window.__scGames = games;
        return games;
      })
      .catch(function () { return []; });
    return window.__scGamesLoading;
  }
  function syncBrowse() {
    var form = document.getElementById("game-filters");
    if (!form) return;
    var state = formState(form);
    writeQuery(state);
    if (stateIsDefault(state) && location.pathname.indexOf("/page/") !== 0) {
      if (document.getElementById("game-list") && document.getElementById("game-list").getAttribute("data-client") === "1") {
        location.href = "/";
      }
      var reset = document.getElementById("filter-reset");
      if (reset) reset.hidden = true;
      return;
    }
    ensureIndex().then(renderClient);
  }
  function resetGameFilters() {
    var form = document.getElementById("game-filters");
    if (!form) return;
    if (form.platform) form.platform.value = "all";
    if (form.year) form.year.value = "all";
    if (form.min_reviews) form.min_reviews.value = "10";
    if (form.q) form.q.value = "";
    form.setAttribute("data-sort", "score");
    syncBrowse();
  }
  function bindSort(listId, defaultKey) {
    var list = document.getElementById(listId);
    if (!list) return;
    var root = list.closest("section, main, body") || document;
    var nav = list.parentElement && list.parentElement.querySelector(".sort");
    if (!nav) nav = root.querySelector(".sort");
    if (!nav) return;
    nav.addEventListener("click", function (ev) {
      var btn = ev.target.closest("[data-sort]");
      if (!btn || !nav.contains(btn)) return;
      ev.preventDefault();
      if (listId === "game-list") {
        var form = document.getElementById("game-filters");
        if (form) form.setAttribute("data-sort", btn.getAttribute("data-sort") || "score");
        setCurrent(nav, btn.getAttribute("data-sort"));
        syncBrowse();
        return;
      }
      sortChildren(list, btn.getAttribute("data-sort"), btn.getAttribute("data-sort"));
    });
    if (listId !== "game-list") sortChildren(list, defaultKey, defaultKey);
  }
  document.addEventListener("DOMContentLoaded", function () {
    var form = document.getElementById("game-filters");
    if (form) {
      readQuery(form);
      form.addEventListener("submit", function (ev) { ev.preventDefault(); syncBrowse(); });
      form.addEventListener("change", syncBrowse);
      form.addEventListener("input", syncBrowse);
    }
    var reset = document.getElementById("filter-reset");
    if (reset) reset.addEventListener("click", resetGameFilters);
    bindSort("game-list", "score");
    bindSort("outlet-list", "reviews");
    bindSort("game-reviews", "name");
    bindSort("outlet-reviews", "name");
    bindPortFilter();
    if (form && !stateIsDefault(formState(form))) syncBrowse();
    document.querySelectorAll("[data-copy]").forEach(function (btn) {
      btn.addEventListener("click", function () {
        var value = btn.getAttribute("data-copy") || "";
        if (!value || !navigator.clipboard) return;
        navigator.clipboard.writeText(value).then(function () {
          var prev = btn.textContent;
          btn.textContent = "Copied";
          setTimeout(function () { btn.textContent = prev; }, 1200);
        });
      });
    });
    initConsent();
  });
  function bindPortFilter() {
    var list = document.getElementById("game-reviews");
    var bar = document.getElementById("port-filter");
    var meta = document.getElementById("port-filter-meta");
    if (!list || !bar) return;
    function apply(port) {
      var shown = 0;
      var rawSum = 0;
      var rawN = 0;
      var mappedSum = 0;
      var mappedN = 0;
      Array.prototype.forEach.call(list.querySelectorAll(".review"), function (row) {
        var plats = (row.getAttribute("data-platforms") || "").split(/\s+/).filter(Boolean);
        var ok = port === "all" || plats.indexOf(port) !== -1;
        row.hidden = !ok;
        if (!ok) return;
        shown += 1;
        var raw = num(row, "data-raw-norm");
        var mapped = num(row, "data-mapped");
        if (isFinite(raw)) { rawSum += raw; rawN += 1; }
        if (isFinite(mapped)) { mappedSum += mapped; mappedN += 1; }
      });
      Array.prototype.forEach.call(bar.querySelectorAll("[data-port]"), function (btn) {
        btn.setAttribute("aria-pressed", btn.getAttribute("data-port") === port ? "true" : "false");
      });
      var hero = document.getElementById("score-hero-value");
      var heroMeta = document.getElementById("score-hero-meta");
      var label = btnLabel(port) || port;
      if (port === "all") {
        if (hero) hero.textContent = hero.getAttribute("data-base") || hero.textContent;
        if (heroMeta) heroMeta.innerHTML = heroMeta.getAttribute("data-base") || heroMeta.innerHTML;
      } else {
        var score = mappedN ? Math.round(mappedSum / mappedN) : 0;
        var mean = rawN ? Math.round(rawSum / rawN) : 0;
        if (hero) hero.textContent = String(score);
        if (heroMeta) {
          heroMeta.textContent = label + " · simple mean " + mean + " · " + shown + (shown === 1 ? " review" : " reviews");
        }
      }
      if (meta) {
        meta.textContent = port === "all"
          ? ("All platforms · " + shown + " reviews")
          : (label + " · " + shown + " reviews");
      }
    }
    function btnLabel(port) {
      var btn = bar.querySelector('[data-port="' + port + '"]');
      if (!btn) return port;
      var name = btn.querySelector(".port-name");
      return name ? name.textContent : btn.textContent;
    }
    bar.addEventListener("click", function (ev) {
      var btn = ev.target.closest("[data-port]");
      if (!btn || !bar.contains(btn)) return;
      apply(btn.getAttribute("data-port") || "all");
    });
    apply("all");
  }
  function isEeaUk() {
    var tz = "";
    try { tz = Intl.DateTimeFormat().resolvedOptions().timeZone || ""; } catch (e) {}
    if (tz.indexOf("Europe/") === 0) return true;
    return tz === "Atlantic/Reykjavik" || tz === "Atlantic/Faroe" || tz === "Atlantic/Canary" || tz === "Atlantic/Madeira" || tz === "Atlantic/Azores";
  }
  function consentUpdate(granted) {
    var value = granted ? "granted" : "denied";
    window.dataLayer = window.dataLayer || [];
    function gtag(){ window.dataLayer.push(arguments); }
    window.gtag = window.gtag || gtag;
    gtag("consent", "update", {
      ad_storage: value,
      ad_user_data: value,
      ad_personalization: value,
      analytics_storage: value
    });
  }
  function watchUnfilled(unit) {
    var ins = unit.querySelector("ins.adsbygoogle");
    if (!ins) return;
    var obs = new MutationObserver(function () {
      if (ins.getAttribute("data-ad-status") === "unfilled") {
        unit.classList.add("is-empty");
        unit.hidden = true;
      }
    });
    obs.observe(ins, { attributes: true, attributeFilter: ["data-ad-status"] });
  }
  function loadAds() {
    if (localStorage.getItem("sc_consent") !== "granted") return;
    var units = document.querySelectorAll(".ad-unit");
    if (!units.length) return;
    var client = units[0].getAttribute("data-ad-client") || "";
    function fill() {
      try {
        window.adsbygoogle = window.adsbygoogle || [];
        window.adsbygoogle.push({
          google_ad_client: client,
          enable_page_level_ads: false
        });
        Array.prototype.forEach.call(units, function (unit) {
          if (unit.querySelector(".adsbygoogle")) window.adsbygoogle.push({});
          watchUnfilled(unit);
        });
      } catch (e) {}
    }
    var ga = document.body && document.body.getAttribute("data-ga");
    if (ga && !window.__scGaLoaded) {
      window.__scGaLoaded = true;
      var g = document.createElement("script");
      g.async = true;
      g.src = "https://www.googletagmanager.com/gtag/js?id=" + encodeURIComponent(ga);
      document.head.appendChild(g);
      window.dataLayer = window.dataLayer || [];
      function gtag(){ window.dataLayer.push(arguments); }
      gtag("js", new Date());
      gtag("config", ga, { anonymize_ip: true });
    }
    if (!client) return;
    if (window.__scAdsScriptLoading) {
      window.__scAdsScriptLoading.then(fill);
      return;
    }
    window.__scAdsScriptLoading = new Promise(function (resolve) {
      var s = document.createElement("script");
      s.async = true;
      s.crossOrigin = "anonymous";
      s.src = "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client=" + encodeURIComponent(client);
      s.onload = function () { resolve(); };
      s.onerror = function () { resolve(); };
      document.head.appendChild(s);
    });
    window.__scAdsScriptLoading.then(fill);
  }
  function initConsent() {
    var banner = document.getElementById("cookie-banner");
    var saved = localStorage.getItem("sc_consent");
    if (saved === "granted") {
      if (banner) banner.hidden = true;
      consentUpdate(true);
      loadAds();
      return;
    }
    if (saved === "denied") {
      if (banner) banner.hidden = true;
      document.querySelectorAll(".ad-unit").forEach(function (unit) { unit.hidden = true; });
      return;
    }
    if (isEeaUk()) {
      if (banner) banner.hidden = true;
      window.googlefc = window.googlefc || {};
      window.googlefc.callbackQueue = window.googlefc.callbackQueue || [];
      window.googlefc.callbackQueue.push({
        CONSENT_DATA_READY: function () {
          try {
            var values = window.googlefc.getGoogleConsentModeValues ? window.googlefc.getGoogleConsentModeValues() : null;
            var granted = values && (values.ad_storage === "granted" || values.analytics_storage === "granted");
            if (granted) {
              localStorage.setItem("sc_consent", "granted");
              consentUpdate(true);
              loadAds();
            }
          } catch (e) {}
        }
      });
      var fc = document.createElement("script");
      fc.async = true;
      fc.src = "https://fundingchoicesmessages.google.com/i/pub-5490722539415533?ers=1";
      fc.onerror = function () { if (banner) banner.hidden = false; };
      document.head.appendChild(fc);
      return;
    }
    if (!banner) return;
    banner.hidden = false;
    var accept = document.getElementById("cookie-accept");
    var reject = document.getElementById("cookie-reject");
    if (accept) accept.addEventListener("click", function () {
      localStorage.setItem("sc_consent", "granted");
      banner.hidden = true;
      consentUpdate(true);
      loadAds();
    });
    if (reject) reject.addEventListener("click", function () {
      localStorage.setItem("sc_consent", "denied");
      banner.hidden = true;
      consentUpdate(false);
      document.querySelectorAll(".ad-unit").forEach(function (unit) { unit.hidden = true; });
    });
  }
})();
"#;

enum Section {
    Games,
    Outlets,
    Score,
    About,
    Other,
}

#[derive(Debug, Deserialize)]
pub(super) struct SortQuery {
    #[serde(default)]
    sort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GameListQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    year: Option<String>,
    #[serde(default, alias = "min")]
    min_reviews: Option<i64>,
    #[serde(default)]
    page: Option<u32>,
}

#[derive(Debug, Clone)]
struct GameFilters {
    platform: String,
    year: String,
    min_reviews: i64,
}

pub(super) async fn home(
    State(state): State<AppState>,
    Query(query): Query<GameListQuery>,
) -> impl IntoResponse {
    let games = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()),
        };
        match db.list_games() {
            Ok(games) => games,
            Err(error) => return internal(error.to_string()),
        }
    };
    let sort = normalize_game_sort(query.sort.as_deref());
    let filters = normalize_game_filters(&query);
    let page = query.page.unwrap_or(1).max(1) as usize;
    render_games_page(games, sort, &filters, page, None)
}

pub(super) async fn games_index(State(state): State<AppState>) -> impl IntoResponse {
    let games = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()).into_response(),
        };
        match db.list_games() {
            Ok(games) => games,
            Err(error) => return internal(error.to_string()).into_response(),
        }
    };
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        games_index_json(&games),
    )
        .into_response()
}

pub(super) async fn privacy() -> impl IntoResponse {
    ok_seo(
        Section::Other,
        "Privacy policy | SourceCritic",
        &legal_page(true),
        "/privacy/",
        "What SourceCritic collects, how AdSense and Analytics cookies work, and your LGPD rights.",
    )
}

pub(super) async fn terms() -> impl IntoResponse {
    ok_seo(
        Section::Other,
        "Terms of use | SourceCritic",
        &legal_page(false),
        "/terms/",
        "Terms for using SourceCritic. Reviews belong to the outlets that published them.",
    )
}

pub(super) async fn most_divisive(State(state): State<AppState>) -> impl IntoResponse {
    discovery_response(state, "most-divisive/index.html").await
}

pub(super) async fn disagreements(State(state): State<AppState>) -> impl IntoResponse {
    discovery_response(state, "disagreements/index.html").await
}

pub(super) async fn outlet_ranking(State(state): State<AppState>) -> impl IntoResponse {
    discovery_response(state, "outlets/ranking/index.html").await
}

pub(super) async fn best_index(State(state): State<AppState>) -> impl IntoResponse {
    discovery_response(state, "best/index.html").await
}

pub(super) async fn best_of(
    State(state): State<AppState>,
    Path((platform, year)): Path<(String, String)>,
) -> impl IntoResponse {
    discovery_response(state, &format!("best/{platform}/{year}/index.html")).await
}

async fn discovery_response(state: AppState, file: &str) -> axum::response::Response {
    let (games, outlets) = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()).into_response(),
        };
        match (db.list_games(), db.list_outlets()) {
            (Ok(games), Ok(mut outlets)) => {
                outlets.retain(|outlet| is_public_outlet(&outlet.slug));
                (games, outlets)
            }
            (Err(error), _) | (_, Err(error)) => return internal(error.to_string()).into_response(),
        }
    };
    match discovery_pages(&games, &outlets)
        .into_iter()
        .find(|(path, _)| path == file)
    {
        Some((_, html)) => (StatusCode::OK, Html(html)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            document(
                Section::Other,
                "Not found",
                "<h1 class=\"page\">Page not found.</h1>",
                None,
                &Seo::default(),
            ),
        )
            .into_response(),
    }
}

fn ok_seo(
    section: Section,
    title: &str,
    body: &str,
    path: &str,
    description: &str,
) -> (StatusCode, Html<String>) {
    (
        StatusCode::OK,
        document(
            section,
            title,
            body,
            Some(path),
            &Seo {
                description: description.to_string(),
                ..Seo::default()
            },
        ),
    )
}

pub(super) async fn home_page(
    State(state): State<AppState>,
    Path(page): Path<u32>,
    Query(query): Query<GameListQuery>,
) -> impl IntoResponse {
    let games = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()),
        };
        match db.list_games() {
            Ok(games) => games,
            Err(error) => return internal(error.to_string()),
        }
    };
    let sort = normalize_game_sort(query.sort.as_deref());
    let filters = normalize_game_filters(&query);
    render_games_page(games, sort, &filters, page.max(1) as usize, None)
}

fn render_games_page(
    mut games: Vec<GameView>,
    sort: &str,
    filters: &GameFilters,
    page: usize,
    path_override: Option<&str>,
) -> (StatusCode, Html<String>) {
    let platforms = collect_platforms(&games);
    let years = collect_years(&games);
    let catalog_total = games.len();
    sort_games(&mut games, sort);
    let filtered: Vec<GameView> = games
        .into_iter()
        .filter(|game| game_matches(game, filters))
        .collect();
    let filtered_total = filtered.len();
    let page_count = filtered_total.div_ceil(PAGE_SIZE).max(1);
    if page > page_count && filtered_total > 0 {
        return (
            StatusCode::NOT_FOUND,
            document(
                Section::Other,
                "Not found",
                "<h1 class=\"page\">Page not found.</h1>",
                None,
                &Seo::default(),
            ),
        );
    }
    let start = (page - 1).saturating_mul(PAGE_SIZE).min(filtered_total);
    let end = (start + PAGE_SIZE).min(filtered_total);
    let slice = if filtered_total == 0 {
        &filtered[..]
    } else {
        &filtered[start..end]
    };
    let path = path_override
        .map(|value| value.to_string())
        .unwrap_or_else(|| page_href(page));
    let title = if page <= 1 {
        "SourceCritic: notas e reviews de jogos da crítica"
    } else {
        "SourceCritic: notas e reviews de jogos da crítica"
    };
    let description = format!(
        "Notas da crítica para {filtered_total} jogos, ajustadas pelo hábito de cada veículo. Página {page} de {page_count}."
    );
    (
        StatusCode::OK,
        document(
            Section::Games,
            title,
            &game_list(
                slice,
                sort,
                filters,
                &platforms,
                &years,
                catalog_total,
                filtered_total,
                page,
                page_count,
            ),
            Some(&path),
            &Seo {
                description,
                image: Some(format!("{SITE_ORIGIN}/og-default.png")),
                json_ld: item_list_json("SourceCritic games", slice),
                extra_head: pager_head(page, page_count),
            },
        ),
    )
}

pub(super) async fn game(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<SortQuery>,
) -> impl IntoResponse {
    let found = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()),
        };
        match db.get_game(&slug) {
            Ok(game) => game.or_else(|| {
                db.list_games().ok().and_then(|games| {
                    games.into_iter().find(|game| public_slug(&game.slug, &game.title) == slug)
                })
            }),
            Err(error) => return internal(error.to_string()),
        }
    };
    let Some(mut game) = found else {
        return (
            StatusCode::NOT_FOUND,
            document(
                Section::Other,
                "Not found",
                "<h1 class=\"page\">Game not found.</h1>",
                None,
                &Seo::default(),
            ),
        );
    };
    let sort = normalize_review_sort(query.sort.as_deref());
    sort_game_reviews(&mut game.reviews, sort, game.mu_global);
    ok(Section::Other, &game.title, &game_page(&game, sort))
}

pub(super) async fn method() -> impl IntoResponse {
    ok(Section::Score, "The score", &method_page())
}

pub(super) async fn about() -> impl IntoResponse {
    ok(Section::About, "About", &about_page())
}

pub(super) async fn outlets(
    State(state): State<AppState>,
    Query(query): Query<SortQuery>,
) -> impl IntoResponse {
    let mut outlets = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()),
        };
        match db.list_outlets() {
            Ok(outlets) => outlets,
            Err(error) => return internal(error.to_string()),
        }
    };
    let sort = normalize_outlet_sort(query.sort.as_deref());
    sort_outlets(&mut outlets, sort);
    ok(Section::Outlets, "Outlets", &outlet_list(&outlets, sort))
}

pub(super) async fn outlet(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<SortQuery>,
) -> impl IntoResponse {
    let found = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()),
        };
        match db.get_outlet(&slug) {
            Ok(outlet) => outlet,
            Err(error) => return internal(error.to_string()),
        }
    };
    let Some(mut outlet) = found else {
        return (
            StatusCode::NOT_FOUND,
            document(
                Section::Other,
                "Not found",
                "<h1 class=\"page\">Outlet not found.</h1>",
                None,
                &Seo::default(),
            ),
        );
    };
    let sort = normalize_review_sort(query.sort.as_deref());
    sort_outlet_reviews(&mut outlet.reviews, sort, outlet.mu_global);
    ok(Section::Outlets, &outlet.name, &outlet_page(&outlet, sort))
}

fn normalize_game_sort(sort: Option<&str>) -> &'static str {
    match sort {
        Some("mean") => "mean",
        Some("title") => "title",
        Some("reviews") => "reviews",
        _ => "score",
    }
}

fn normalize_game_filters(query: &GameListQuery) -> GameFilters {
    let platform = query
        .platform
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("all")
        .to_string();
    let year = query
        .year
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("all")
        .to_string();
    let min_reviews = query.min_reviews.unwrap_or(DEFAULT_MIN_REVIEWS).max(0);
    GameFilters {
        platform,
        year,
        min_reviews,
    }
}

fn collect_platforms(games: &[GameView]) -> Vec<(String, String)> {
    let mut map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for game in games {
        for platform in &game.platforms {
            let key = platform_key(platform);
            map.entry(key.clone())
                .or_insert_with(|| platform_label(&key));
        }
    }
    map.into_iter().collect()
}

fn collect_years(games: &[GameView]) -> Vec<i32> {
    let mut years: Vec<i32> = games.iter().filter_map(|game| game.release_year).collect();
    years.sort_unstable();
    years.dedup();
    years.reverse();
    years
}

fn platform_key(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    match lower.as_str() {
        "playstation 5" | "ps5" => "ps5".into(),
        "playstation 4" | "ps4" => "ps4".into(),
        "playstation 3" | "ps3" => "ps3".into(),
        "playstation 2" | "ps2" => "ps2".into(),
        "playstation" | "ps1" | "psx" => "ps1".into(),
        "playstation vita" | "ps vita" | "psvita" | "vita" => "vita".into(),
        "psp" | "playstation portable" => "psp".into(),
        "xbox series x" | "xbox series x/s" | "xbox series s" | "xbox-series-x" | "xbxs" => {
            "xbox-series".into()
        }
        "xbox one" | "xbox-one" => "xbox-one".into(),
        "nintendo switch 2" | "switch 2" | "nintendo-switch-2" => "switch-2".into(),
        "nintendo switch" | "switch" | "nintendo-switch" => "switch".into(),
        "nintendo 3ds" | "3ds" => "3ds".into(),
        "nintendo ds" | "nds" | "ds" => "ds".into(),
        "nintendo gamecube" | "gamecube" | "game cube" | "gc" => "gamecube".into(),
        "wii u" | "wiiu" => "wii-u".into(),
        "wii" => "wii".into(),
        "nintendo 64" | "n64" => "n64".into(),
        "super nintendo" | "snes" | "super nes" => "snes".into(),
        "nes" | "nintendo entertainment system" => "nes".into(),
        "game boy advance" | "gba" => "gba".into(),
        "game boy color" | "gbc" => "gbc".into(),
        "game boy" | "gb" => "gb".into(),
        "xbox 360" | "x360" => "xbox-360".into(),
        "xbox" => "xbox".into(),
        "pc" | "windows pc" | "windows" => "pc".into(),
        "mac" | "macos" | "os x" => "mac".into(),
        "linux" => "linux".into(),
        "ios" | "iphone/ipad" | "iphone" | "ipad" | "ios iphone ipad"
        | "ios (iphone/ipad)" | "iphone / ipad" | "ios-iphone-ipad" => "ios".into(),
        "android" => "android".into(),
        "stadia" => "stadia".into(),
        other => other
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string(),
    }
}

fn platform_label(key: &str) -> String {
    match key {
        "ps5" => "PlayStation 5".into(),
        "ps4" => "PlayStation 4".into(),
        "ps3" => "PlayStation 3".into(),
        "ps2" => "PlayStation 2".into(),
        "ps1" => "PlayStation".into(),
        "vita" => "PlayStation Vita".into(),
        "psp" => "PSP".into(),
        "xbox-series" => "Xbox Series X/S".into(),
        "xbox-one" => "Xbox One".into(),
        "xbox-360" => "Xbox 360".into(),
        "xbox" => "Xbox".into(),
        "switch-2" => "Nintendo Switch 2".into(),
        "switch" => "Nintendo Switch".into(),
        "3ds" => "Nintendo 3DS".into(),
        "ds" => "Nintendo DS".into(),
        "gamecube" => "GameCube".into(),
        "wii-u" => "Wii U".into(),
        "wii" => "Wii".into(),
        "n64" => "Nintendo 64".into(),
        "snes" => "SNES".into(),
        "nes" => "NES".into(),
        "gba" => "Game Boy Advance".into(),
        "gbc" => "Game Boy Color".into(),
        "gb" => "Game Boy".into(),
        "pc" => "PC".into(),
        "mac" => "Mac".into(),
        "linux" => "Linux".into(),
        "ios" => "iOS".into(),
        "android" => "Android".into(),
        "stadia" => "Stadia".into(),
        other => other
            .split('-')
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

#[allow(dead_code)]
fn game_matches(game: &GameView, filters: &GameFilters) -> bool {
    if game.review_count < filters.min_reviews {
        return false;
    }
    if filters.year != "all" {
        let Ok(year) = filters.year.parse::<i32>() else {
            return false;
        };
        if game.release_year != Some(year) {
            return false;
        }
    }
    if filters.platform != "all" {
        let wanted = platform_key(&filters.platform);
        if !game
            .platforms
            .iter()
            .any(|platform| platform_key(platform) == wanted)
        {
            return false;
        }
    }
    true
}

#[allow(dead_code)]
fn game_list_query(sort: &str, filters: &GameFilters) -> String {
    let mut parts: Vec<String> = Vec::new();
    if sort != "score" {
        parts.push(format!("sort={}", esc(sort)));
    }
    if filters.platform != "all" {
        parts.push(format!("platform={}", esc(&filters.platform)));
    }
    if filters.year != "all" {
        parts.push(format!("year={}", esc(&filters.year)));
    }
    if filters.min_reviews != 0 {
        parts.push(format!("min_reviews={}", filters.min_reviews));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

fn normalize_outlet_sort(sort: Option<&str>) -> &'static str {
    match sort {
        Some("name") => "name",
        Some("mu") => "mu",
        Some("mu_prime") => "mu_prime",
        _ => "reviews",
    }
}

fn normalize_review_sort(sort: Option<&str>) -> &'static str {
    match sort {
        Some("score") => "score",
        Some("z") => "z",
        _ => "name",
    }
}

fn sort_games(games: &mut [GameView], sort: &str) {
    match sort {
        "mean" => games.sort_by(|a, b| {
            b.simple_mean
                .partial_cmp(&a.simple_mean)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.title.cmp(&b.title))
        }),
        "title" => games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
        "reviews" => games.sort_by(|a, b| {
            b.review_count
                .cmp(&a.review_count)
                .then_with(|| a.title.cmp(&b.title))
        }),
        _ => games.sort_by(|a, b| {
            b.agg_score
                .partial_cmp(&a.agg_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.title.cmp(&b.title))
        }),
    }
}

fn sort_outlets(outlets: &mut [OutletView], sort: &str) {
    match sort {
        "name" => outlets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        "mu" => outlets.sort_by(|a, b| {
            b.mu
                .partial_cmp(&a.mu)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
        "mu_prime" => outlets.sort_by(|a, b| {
            b.mu_prime
                .partial_cmp(&a.mu_prime)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
        _ => outlets.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.name.cmp(&b.name))),
    }
}

fn sort_game_reviews(reviews: &mut [ReviewView], sort: &str, mu_global: f64) {
    match sort {
        "score" => reviews.sort_by(|a, b| {
            let a_mapped = crate::engine::map_z(a.z_score, mu_global);
            let b_mapped = crate::engine::map_z(b.z_score, mu_global);
            b_mapped
                .partial_cmp(&a_mapped)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.outlet_name.cmp(&b.outlet_name))
        }),
        "z" => reviews.sort_by(|a, b| {
            b.z_score
                .partial_cmp(&a.z_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.outlet_name.cmp(&b.outlet_name))
        }),
        _ => reviews.sort_by(|a, b| {
            a.outlet_name
                .to_lowercase()
                .cmp(&b.outlet_name.to_lowercase())
        }),
    }
}

fn sort_outlet_reviews(
    reviews: &mut [crate::db::OutletReviewView],
    sort: &str,
    mu_global: f64,
) {
    match sort {
        "score" => reviews.sort_by(|a, b| {
            let a_mapped = crate::engine::map_z(a.z_score, mu_global);
            let b_mapped = crate::engine::map_z(b.z_score, mu_global);
            b_mapped
                .partial_cmp(&a_mapped)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.game_title.cmp(&b.game_title))
        }),
        "z" => reviews.sort_by(|a, b| {
            b.z_score
                .partial_cmp(&a.z_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.game_title.cmp(&b.game_title))
        }),
        _ => reviews.sort_by(|a, b| {
            a.game_title
                .to_lowercase()
                .cmp(&b.game_title.to_lowercase())
        }),
    }
}

fn sort_bar(_base: &str, current: &str, _default: &str, options: &[(&str, &str)]) -> String {
    let mut html = String::from(
        "<nav class=\"sort\" aria-label=\"Sort by\"><span class=\"label\">Sort by</span>",
    );
    for (key, label) in options {
        let current_attr = if current == *key {
            " aria-current=\"true\""
        } else {
            ""
        };
        html.push_str(&format!(
            "<button type=\"button\" data-sort=\"{key}\"{current_attr}>{label}</button>",
            key = esc(key),
            current_attr = current_attr,
            label = esc(label),
        ));
    }
    html.push_str("</nav>");
    html
}

fn page_href(page: usize) -> String {
    if page <= 1 {
        "/".to_string()
    } else {
        format!("/page/{page}/")
    }
}

fn pager_head(page: usize, page_count: usize) -> String {
    let mut html = String::new();
    if page > 1 {
        html.push_str(&format!(
            "<link rel=\"prev\" href=\"{SITE_ORIGIN}{href}\">",
            href = page_href(page - 1)
        ));
    }
    if page < page_count {
        html.push_str(&format!(
            "<link rel=\"next\" href=\"{SITE_ORIGIN}{href}\">",
            href = page_href(page + 1)
        ));
    }
    html
}

fn pager_nav(page: usize, page_count: usize) -> String {
    if page_count <= 1 {
        return String::new();
    }
    let prev = if page > 1 {
        format!("<a href=\"{}\" rel=\"prev\">Previous</a>", page_href(page - 1))
    } else {
        String::new()
    };
    let next = if page < page_count {
        format!("<a href=\"{}\" rel=\"next\">Next</a>", page_href(page + 1))
    } else {
        String::new()
    };
    format!(
        "<nav class=\"pager\" id=\"pager\" aria-label=\"Pages\">{prev}<span>Page {page} of {page_count}</span>{next}</nav>"
    )
}

fn site_footer() -> String {
    format!(
        "<footer class=\"site-footer\">\
           <nav>\
             <a href=\"/privacy/\">Privacy</a>\
             <a href=\"/terms/\">Terms</a>\
             <a href=\"/about/\">About</a>\
             <a href=\"/best/\">Best of</a>\
             <a href=\"/most-divisive/\">Most divisive</a>\
             <a href=\"/disagreements/\">Disagreements</a>\
             <a href=\"/outlets/ranking/\">Outlet habits</a>\
           </nav>\
           <p>Game metadata and cover art come from IGDB, powered by Twitch. Reviews remain the property of each publication: SourceCritic shows the score, a short excerpt, and a link.</p>\
         </footer>"
    )
}

fn cookie_banner() -> String {
    String::from(
        "<div class=\"cookie-banner\" id=\"cookie-banner\">\
           <p>Cookies de anúncio (AdSense) e medição (Analytics) ficam desligados até você aceitar. \
           Ad and analytics cookies stay off until you accept. \
           <a href=\"/privacy/\">Privacy</a></p>\
           <div class=\"cookie-actions\">\
             <button type=\"button\" class=\"reject\" id=\"cookie-reject\">Reject</button>\
             <button type=\"button\" class=\"accept\" id=\"cookie-accept\">Accept</button>\
           </div>\
         </div>",
    )
}

fn newsletter_form() -> String {
    let config = include_str!("../../data/newsletter.json");
    let endpoint = serde_json::from_str::<serde_json::Value>(config)
        .ok()
        .and_then(|value| value.get("endpoint").and_then(|item| item.as_str()).map(str::to_string))
        .filter(|value| !value.is_empty() && !value.contains("YOUR_"))
        .unwrap_or_else(|| "https://buttondown.com/api/emails/embed-subscribe/YOUR_BUTTONDOWN_USERNAME".to_string());
    format!(
        "<form class=\"newsletter\" action=\"{endpoint}\" method=\"post\" target=\"_blank\">\
           <label>Email <input type=\"email\" name=\"email\" required autocomplete=\"email\" placeholder=\"you@example.com\"></label>\
           <button type=\"submit\">Subscribe</button>\
         </form>",
        endpoint = esc(&endpoint),
    )
}

fn item_list_json(name: &str, games: &[GameView]) -> String {
    let items: Vec<String> = games
        .iter()
        .enumerate()
        .map(|(index, game)| {
            format!(
                "{{\"@type\":\"ListItem\",\"position\":{},\"name\":{},\"url\":\"{SITE_ORIGIN}/game/{}/\"}}",
                index + 1,
                serde_json::to_string(&game.title).unwrap_or_else(|_| "\"\"".into()),
                public_slug(&game.slug, &game.title),
            )
        })
        .collect();
    format!(
        "{{\"@context\":\"https://schema.org\",\"@type\":\"ItemList\",\"name\":{},\"itemListElement\":[{}]}}",
        serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into()),
        items.join(",")
    )
}

fn game_list(
    games: &[GameView],
    sort: &str,
    filters: &GameFilters,
    platforms: &[(String, String)],
    years: &[i32],
    catalog_total: usize,
    filtered_total: usize,
    page: usize,
    page_count: usize,
) -> String {
    let mut html = String::from(
        "<h1 class=\"page\">Games</h1>\
         <p class=\"lead\">Outlet scores, adjusted so generous critics do not inflate a game and harsh critics do not drag it down.</p>",
    );
    html.push_str(&browse_controls(filters, platforms, years, sort));
    let start = if filtered_total == 0 {
        0
    } else {
        (page - 1) * PAGE_SIZE + 1
    };
    let end = start.saturating_add(games.len().saturating_sub(if start == 0 { 0 } else { 1 }));
    html.push_str(&format!(
        "<p class=\"filter-meta\" id=\"filter-meta\">Showing {start}–{end} of {filtered_total} games.</p>",
    ));
    if catalog_total == 0 {
        html.push_str("<p class=\"empty\">No games yet.</p>");
        return html;
    }
    let open = format!(
        "<p class=\"empty\" id=\"game-empty\" hidden>No games match these filters.</p>\
         <div id=\"game-list\" data-total=\"{filtered_total}\">"
    );
    html.push_str(&open);
    for (index, game) in games.iter().enumerate() {
        let count = if game.review_count == 1 {
            "1 review".to_string()
        } else {
            format!("{} reviews", game.review_count)
        };
        let cover = cover_img(game.cover_path.as_deref(), false, &game.title);
        let score_value = game.agg_score.unwrap_or(0.0);
        let mean_value = game.simple_mean.unwrap_or(0.0);
        let score_label = fmt_score(score_value);
        let mean_label = fmt_score(mean_value);
        let platform_keys: Vec<String> = game
            .platforms
            .iter()
            .map(|platform| platform_key(platform))
            .collect();
        let platform_labels = platform_keys
            .iter()
            .map(|key| platform_label(key))
            .collect::<Vec<_>>()
            .join(", ");
        let year_attr = game
            .release_year
            .map(|year| year.to_string())
            .unwrap_or_default();
        let mut bits_prefix = String::new();
        if let Some(year) = game.release_year {
            bits_prefix.push_str(&year.to_string());
        }
        if !platform_labels.is_empty() {
            if !bits_prefix.is_empty() {
                bits_prefix.push_str(" · ");
            }
            bits_prefix.push_str(&platform_labels);
        }
        let bits_tail = if bits_prefix.is_empty() {
            String::new()
        } else {
            format!(" · {}", esc(&bits_prefix))
        };
        // Same rule either way: red if score > mean, purple if score < mean.
        let meta_html = if sort == "mean" {
            format!(
                "score {}",
                cmp_num_html(&score_label, score_value, mean_value)
            )
        } else {
            format!(
                "simple mean {}",
                cmp_num_html(&mean_label, score_value, mean_value)
            )
        };
        let (score_hidden, mean_hidden) = if sort == "mean" {
            (" hidden", "")
        } else {
            ("", " hidden")
        };
        let provisional = if game.review_count < 5 {
            " <span class=\"provisional\">nota provisória</span>"
        } else {
            ""
        };
        let secondary = if bits_tail.is_empty() {
            String::new()
        } else {
            format!("<span class=\"row-secondary\">{bits_tail}</span>")
        };
        html.push_str(&format!(
            "<a class=\"row\" href=\"/game/{slug}/\" \
                data-score=\"{score_n}\" data-mean=\"{mean_n}\" data-reviews=\"{reviews}\" \
                data-title=\"{title_attr}\" data-year=\"{year}\" data-platforms=\"{plats}\" \
                data-score-label=\"{score_label}\" data-mean-label=\"{mean_label}\">\
               <span class=\"meta\">{cover}<span class=\"title-block\"><strong>{title}</strong>\
                 <small><span class=\"row-secondary\" data-score-meta>{meta_html}</span> · <span class=\"row-count\">{count}</span>{secondary}</small></span></span>\
               <span data-score-display{score_hidden}>{score_html}{provisional}</span>\
               <span data-mean-display{mean_hidden}>{mean_html}</span>\
             </a>",
            slug = esc(&public_slug(&game.slug, &game.title)),
            provisional = provisional,
            secondary = secondary,
            title = esc(&game.title),
            title_attr = esc(&game.title),
            count = esc(&count),
            meta_html = meta_html,
            score_html = score_compact_html(score_value),
            mean_html = score_compact_html(mean_value),
            score_hidden = score_hidden,
            mean_hidden = mean_hidden,
            score_n = score_value,
            mean_n = mean_value,
            reviews = game.review_count,
            year = esc(&year_attr),
            plats = esc(&platform_keys.join(" ")),
            score_label = esc(&score_label),
            mean_label = esc(&mean_label),
            cover = cover,
        ));
        if (index + 1) % 25 == 0 {
            html.push_str(&ad_unit("ad-inlist rect", "280px"));
        }
    }
    html.push_str("</div>");
    html.push_str(&pager_nav(page, page_count));
    html.push_str(&ad_unit("ad-home-end rect", "280px"));
    html
}

fn filter_bar(
    filters: &GameFilters,
    platforms: &[(String, String)],
    years: &[i32],
    sort: &str,
) -> String {
    let mut html = format!(
        "<form class=\"filters\" id=\"game-filters\" data-sort=\"{sort}\" aria-label=\"Filters\" role=\"search\">",
        sort = esc(sort),
    );

    html.push_str(
        "<label class=\"filter-search\">Search\
           <input type=\"search\" name=\"q\" placeholder=\"Search games\" autocomplete=\"off\" spellcheck=\"false\">\
         </label>",
    );

    html.push_str("<label>Platform<select name=\"platform\">");
    html.push_str(&option_html("all", "All platforms", filters.platform == "all"));
    for (key, label) in platforms {
        html.push_str(&option_html(key, label, filters.platform == *key));
    }
    html.push_str("</select></label>");

    html.push_str("<label>Year<select name=\"year\">");
    html.push_str(&option_html("all", "All years", filters.year == "all"));
    for year in years {
        let value = year.to_string();
        html.push_str(&option_html(
            &value,
            &value,
            filters.year == value,
        ));
    }
    html.push_str("</select></label>");

    html.push_str("<label>Min reviews<select name=\"min_reviews\">");
    for (value, label) in [
        (0, "Any"),
        (10, "10+"),
        (30, "30+"),
        (50, "50+"),
        (100, "100+"),
    ] {
        html.push_str(&option_html(
            &value.to_string(),
            label,
            filters.min_reviews == value,
        ));
    }
    html.push_str("</select></label></form>");
    html
}

fn option_html(value: &str, label: &str, selected: bool) -> String {
    format!(
        "<option value=\"{value}\"{selected}>{label}</option>",
        value = esc(value),
        label = esc(label),
        selected = if selected { " selected" } else { "" },
    )
}

fn game_sort_bar(sort: &str) -> String {
    let options = [
        ("score", "Score"),
        ("mean", "Simple mean"),
        ("reviews", "Reviews"),
        ("title", "Title"),
    ];
    let mut html = String::from(
        "<nav class=\"sort\" aria-label=\"Sort by\"><span class=\"label\">Sort by</span>",
    );
    for (key, label) in options {
        let current_attr = if sort == key {
            " aria-current=\"true\""
        } else {
            ""
        };
        html.push_str(&format!(
            "<button type=\"button\" data-sort=\"{key}\"{current_attr}>{label}</button>",
            key = esc(key),
            current_attr = current_attr,
            label = esc(label),
        ));
    }
    html.push_str("</nav>");
    html
}

fn filters_are_active(filters: &GameFilters) -> bool {
    filters.platform != "all" || filters.year != "all" || filters.min_reviews != DEFAULT_MIN_REVIEWS
}

fn browse_controls(
    filters: &GameFilters,
    platforms: &[(String, String)],
    years: &[i32],
    sort: &str,
) -> String {
    let reset_hidden = if filters_are_active(filters) {
        ""
    } else {
        " hidden"
    };
    format!(
        "<div class=\"browse-controls\">\
           {filters}\
           <div class=\"browse-toolbar\">\
             {sort}\
             <button type=\"button\" class=\"filter-reset\" id=\"filter-reset\"{reset_hidden}>\
               Clear filters\
             </button>\
           </div>\
         </div>",
        filters = filter_bar(filters, platforms, years, sort),
        sort = game_sort_bar(sort),
        reset_hidden = reset_hidden,
    )
}

fn outlet_list(outlets: &[OutletView], sort: &str) -> String {
    if outlets.is_empty() {
        return String::from(
            "<h1 class=\"page\">Outlets</h1>\
             <p class=\"lead\">Each review site&apos;s scoring habit—mean, spread, and the reviews behind them.</p>\
             <p class=\"empty\">No outlets yet.</p>",
        );
    }
    let mut html = String::from(
        "<h1 class=\"page\">Outlets</h1>\
         <p class=\"lead\">Each review site has a raw habit ($\\mu_j$, $\\sigma_j$) and a shrunk habit ($\\mu'_j$, $\\sigma'_j$) pulled toward the catalog. Open one to see every review that built those numbers.</p>",
    );
    html.push_str(&sort_bar(
        "/outlets",
        sort,
        "reviews",
        &[
            ("reviews", "Reviews"),
            ("mu_prime", "μ′"),
            ("mu", "μ"),
            ("name", "Name"),
        ],
    ));
    html.push_str("<div id=\"outlet-list\">");
    for outlet in outlets {
        let count = if outlet.n == 1 {
            "1 review".to_string()
        } else {
            format!("{} reviews", outlet.n)
        };
        html.push_str(&format!(
            "<a class=\"row\" href=\"/outlet/{slug}/\" \
                data-title=\"{name_attr}\" data-reviews=\"{n}\" data-mu=\"{mu_n}\" data-mu-prime=\"{mu_p_n}\">\
               <span><strong>{name}</strong><small>{count} · μ {mu} · σ {sigma} · μ′ {mu_p} · σ′ {sigma_p}</small></span>\
               <span class=\"score\">{mu_p}</span>\
             </a>",
            slug = esc(&outlet.slug),
            name = esc(&outlet.name),
            name_attr = esc(&outlet.name),
            count = count,
            n = outlet.n,
            mu = fmt_num(outlet.mu),
            sigma = fmt_num(outlet.sigma),
            mu_p = fmt_num(outlet.mu_prime),
            sigma_p = fmt_num(outlet.sigma_prime),
            mu_n = outlet.mu,
            mu_p_n = outlet.mu_prime,
        ));
    }
    html.push_str("</div>");
    html
}

fn outlet_page(outlet: &OutletView, sort: &str) -> String {
    let count = if outlet.n == 1 {
        "1 review".to_string()
    } else {
        format!("{} reviews", outlet.n)
    };
    let mut reviews = String::from("<section class=\"reviews\"><h2>Reviews from this outlet</h2>");
    reviews.push_str(&sort_bar(
        &format!("/outlet/{}", outlet.slug),
        sort,
        "name",
        &[("name", "Game"), ("score", "Score"), ("z", "Z")],
    ));
    reviews.push_str("<div id=\"outlet-reviews\">");
    for review in &outlet.reviews {
        let raw = match (review.raw_score, review.scale_max) {
            (Some(score), Some(scale)) => format!("{score}/{scale}"),
            _ => format!("{}", fmt_score(review.raw_score_norm)),
        };
        let mapped = crate::engine::map_z(review.z_score, outlet.mu_global);
        let excerpt = review
            .excerpt
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(|text| format!("<p class=\"excerpt\">{}</p>", esc(text)))
            .unwrap_or_default();
        let source = match review
            .review_url
            .as_deref()
            .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        {
            Some(url) => format!(
                "<p class=\"source\"><a href=\"{url}\" rel=\"noopener noreferrer\" target=\"_blank\">Read original review</a></p>",
                url = esc(url),
            ),
            None => String::new(),
        };
        reviews.push_str(&format!(
            "<div class=\"review\" data-title=\"{game_attr}\" data-score=\"{score_n}\" data-z=\"{z_n}\">\
               <span><strong><a href=\"/game/{game_slug}/\">{game}</a></strong>\
                 <small>{raw} → {norm} · \
                   <span data-z-meta>Z {z}</span>\
                   <span data-score-meta hidden>Score {mapped}</span>\
                 </small>\
               </span>\
               <span data-score-display class=\"mapped\" title=\"This review&apos;s contribution to the game score\">Score {mapped}</span>\
               <span data-z-display class=\"z\" hidden>Z {z}</span>\
               {excerpt}{source}\
             </div>",
            game_slug = esc(&public_slug(&review.game_slug, &review.game_title)),
            game = esc(&review.game_title),
            game_attr = esc(&review.game_title),
            raw = esc(&raw),
            norm = fmt_score(review.raw_score_norm),
            z = fmt_z(review.z_score),
            mapped = fmt_score(mapped),
            score_n = mapped,
            z_n = review.z_score,
            excerpt = excerpt,
            source = source,
        ));
    }
    reviews.push_str("</div></section>");
    format!(
        "<h1 class=\"page\">{name}</h1>\
         <p class=\"muted\">{count}. Catalog baseline μ_global {mu_g}, σ_global {sigma_g}.</p>\
         <p class=\"lead\">μ<sub>j</sub> and σ<sub>j</sub> are this outlet&apos;s raw mean and spread. μ′<sub>j</sub> and σ′<sub>j</sub> are the shrunk values used when scoring a game—pulled toward the catalog so a thin history cannot dominate. See <a href=\"/score/\">how the score works</a>.</p>\
         <div class=\"stats\">\
           <div class=\"stat\"><span class=\"label\">Raw mean μ<sub>j</sub></span><span class=\"value\">{mu}</span></div>\
           <div class=\"stat\"><span class=\"label\">Raw spread σ<sub>j</sub></span><span class=\"value\">{sigma}</span></div>\
           <div class=\"stat\"><span class=\"label\">Shrunk mean μ′<sub>j</sub></span><span class=\"value\">{mu_p}</span></div>\
           <div class=\"stat\"><span class=\"label\">Shrunk spread σ′<sub>j</sub></span><span class=\"value\">{sigma_p}</span></div>\
         </div>\
         {reviews}",
        name = esc(&outlet.name),
        count = count,
        mu_g = fmt_num(outlet.mu_global),
        sigma_g = fmt_num(outlet.sigma_global),
        mu = fmt_num(outlet.mu),
        sigma = fmt_num(outlet.sigma),
        mu_p = fmt_num(outlet.mu_prime),
        sigma_p = fmt_num(outlet.sigma_prime),
        reviews = reviews,
    )
}

fn cover_img(path: Option<&str>, large: bool, title: &str) -> String {
    let Some(path) = path.filter(|p| p.starts_with("/covers/") && !p.contains("..")) else {
        return String::new();
    };
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit_once('.')
        .map(|(name, _)| name)
        .unwrap_or(path);
    let width = if large { 400 } else { 112 };
    let height = if large { 560 } else { 157 };
    let webp = format!("/covers/{stem}-{width}.webp");
    let avif = format!("/covers/{stem}-{width}.avif");
    let alt = format!("Capa de {title}");
    let loading = if large { "eager" } else { "lazy" };
    format!(
        "<picture>\
           <source type=\"image/avif\" srcset=\"{avif}\">\
           <source type=\"image/webp\" srcset=\"{webp}\">\
           <img class=\"{class}\" src=\"{webp}\" alt=\"{alt}\" width=\"{width}\" height=\"{height}\" loading=\"{loading}\">\
         </picture>",
        class = if large { "cover-lg" } else { "cover" },
        avif = esc(&avif),
        webp = esc(&webp),
        alt = esc(&alt),
    )
}

fn game_page(game: &GameView, sort: &str) -> String {
    let count = if game.review_count == 1 {
        "1 review".to_string()
    } else {
        format!("{} reviews", game.review_count)
    };
    let cover = cover_img(game.cover_path.as_deref(), true, &game.title);
    let synopsis = game
        .synopsis
        .as_deref()
        .map(clean_synopsis)
        .filter(|text| !text.is_empty())
        .map(|text| format!("<p class=\"synopsis\">{}</p>", esc(&text)))
        .unwrap_or_default();
    let mut meta_bits = Vec::new();
    if let Some(year) = game.release_year {
        meta_bits.push(year.to_string());
    }
    if !game.platforms.is_empty() {
        meta_bits.push(
            game.platforms
                .iter()
                .map(|platform| platform_label(&platform_key(platform)))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    let meta = if meta_bits.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=\"muted\">{}</p>",
            esc(&meta_bits.join(" · "))
        )
    };
    let mut reviews = String::from("<section class=\"reviews\"><h2>Outlet reviews</h2>");
    // Ports stay on this page. Chips recompute the score from reviews of that port.
    let mut port_keys: Vec<String> = Vec::new();
    for platform in &game.platforms {
        let key = platform_key(platform);
        if !key.is_empty() && !port_keys.iter().any(|k| k == &key) {
            port_keys.push(key);
        }
    }
    for review in game.reviews.iter().filter(|review| is_public_outlet(&review.outlet_slug)) {
        for platform in &review.platforms {
            let key = platform_key(platform);
            if !key.is_empty() && !port_keys.iter().any(|k| k == &key) {
                port_keys.push(key);
            }
        }
    }
    port_keys.sort();
    if port_keys.len() > 1 {
        reviews.push_str("<div class=\"port-filter\" id=\"port-filter\" role=\"group\" aria-label=\"Filter by platform\">");
        let (all_score, _, _) = port_stats(game, &port_keys, None);
        reviews.push_str(&port_chip("all", "All", all_score, true));
        for key in &port_keys {
            let (score, _, _) = port_stats(game, &port_keys, Some(key));
            reviews.push_str(&port_chip(key, &platform_label(key), score, false));
        }
        reviews.push_str("</div>");
        reviews.push_str("<p class=\"port-filter-meta\" id=\"port-filter-meta\"></p>");
    }
    reviews.push_str(&sort_bar(
        &format!("/game/{}", game.slug),
        sort,
        "name",
        &[("name", "Outlet"), ("score", "Score"), ("z", "Z")],
    ));
    reviews.push_str("<div id=\"game-reviews\">");
    for review in game.reviews.iter().filter(|review| is_public_outlet(&review.outlet_slug)) {
        let raw = match (review.raw_score, review.scale_max) {
            (Some(score), Some(scale)) => format!("{score}/{scale}"),
            _ => format!("{}", fmt_score(review.raw_score_norm)),
        };
        let mapped = crate::engine::map_z(review.z_score, game.mu_global);
        let excerpt = review
            .excerpt
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(|text| format!("<p class=\"excerpt\">{}</p>", esc(text)))
            .unwrap_or_default();
        let source = match review.review_url.as_deref().filter(|url| {
            (url.starts_with("https://") || url.starts_with("http://"))
                && !url.contains("savethechildren.org")
        }) {
            Some(url) => format!(
                "<p class=\"source\"><a href=\"{url}\" rel=\"noopener noreferrer\" target=\"_blank\">Read original review</a></p>",
                url = esc(url),
            ),
            None => String::new(),
        };
        let mut plat_keys: Vec<String> = review
            .platforms
            .iter()
            .map(|p| platform_key(p))
            .filter(|k| !k.is_empty())
            .collect();
        // No port tag: the review is of the release, so it counts on every port.
        if plat_keys.is_empty() {
            plat_keys = port_keys.clone();
        }
        let plats_attr = plat_keys.join(" ");
        reviews.push_str(&format!(
            "<div class=\"review\" data-title=\"{outlet_attr}\" data-score=\"{score_n}\" data-z=\"{z_n}\" \
               data-mapped=\"{mapped_n}\" data-raw-norm=\"{norm_n}\" data-platforms=\"{plats}\">\
               <span><strong><a href=\"/outlet/{outlet_slug}/\">{outlet}</a></strong>\
                 <small>{raw} → {norm} · \
                   <span data-z-meta>Z {z}</span>\
                   <span data-score-meta hidden>Score {mapped}</span>\
                 </small>\
               </span>\
               <span data-score-display class=\"mapped\" title=\"This outlet&apos;s contribution to the game score\">Score {mapped}</span>\
               <span data-z-display class=\"z\" hidden>Z {z}</span>\
               {excerpt}{source}\
             </div>",
            outlet_slug = esc(&review.outlet_slug),
            outlet = esc(&review.outlet_name),
            outlet_attr = esc(&review.outlet_name),
            raw = esc(&raw),
            norm = fmt_score(review.raw_score_norm),
            z = fmt_z(review.z_score),
            mapped = fmt_score(mapped),
            score_n = mapped,
            z_n = review.z_score,
            mapped_n = mapped,
            norm_n = review.raw_score_norm,
            plats = esc(&plats_attr),
            excerpt = excerpt,
            source = source,
        ));
    }
    reviews.push_str("</div></section>");
    format!(
        "<div class=\"game-hero\">{cover}<div class=\"text\">\
           <h1 class=\"page\">{title}</h1>\
           <div class=\"score-hero\" aria-label=\"SourceCritic score {score}\">\
             <span class=\"score-hero-value\" id=\"score-hero-value\" data-base=\"{score}\">{score}</span>\
             <span class=\"score-hero-copy\">\
               <span class=\"score-hero-label\">SourceCritic score</span>\
               <span class=\"score-hero-meta\" id=\"score-hero-meta\" data-base=\"{hero_meta}\">{hero_meta}</span>\
             </span>\
           </div>\
           {synopsis}\
           {meta}\
           <p class=\"lead\">The published score is adjusted by each outlet&apos;s history. See <a href=\"/score/\">how the score works</a>.</p>\
         </div></div>\
         {reviews}\
         {buy}\
         {ad}",
        title = esc(&game.title),
        hero_meta = format!(
            "Simple mean {mean} · {count}{provisional}",
            mean = fmt_score(game.simple_mean.unwrap_or(0.0)),
            count = count,
            provisional = if game.review_count < 5 { " · nota provisória" } else { "" },
        ),
        buy = buy_block(&game.title),
        ad = ad_unit("ad-game rect", "280px"),
        score = fmt_score(game.agg_score.unwrap_or(0.0)),
        synopsis = synopsis,
        meta = meta,
        cover = cover,
        reviews = reviews,
    )
}

fn step(n: u8, title: &str, prose: &str, math: &str, code: &str) -> String {
    format!(
        "<article class=\"step\">\
           <p class=\"step-label\">Step {n}</p>\
           <h2>{title}</h2>\
           <p>{prose}</p>\
           <div class=\"panel\">\
             <div class=\"block math\"><span class=\"tag\">Math</span><div class=\"tex\">{math}</div></div>\
             <div class=\"block code\"><span class=\"tag\">Code</span><code>{code}</code></div>\
           </div>\
         </article>",
        title = esc(title),
        prose = prose,
        math = math,
        code = esc(code),
    )
}

fn about_page() -> String {
    let contact_faq = match CONTACT_EMAIL {
        Some(_) => {
            "There is no public submit form yet. If a score looks wrong, an outlet is missing, or a link is broken, email us with the game, outlet, and what should change."
        }
        None => {
            "There is no public submit form yet. A contact address is coming soon—until then, corrections are not open to the public."
        }
    };

    let mut contact_block = String::from(
        "<p class=\"lead\">Bugs, missing games, methodology questions, or press—write and we will get back when we can.</p>",
    );
    let mut contact_actions = String::new();
    match CONTACT_EMAIL {
        Some(email) => {
            let mailto = format!("mailto:{email}");
            contact_block.push_str(&format!(
                "<p>Email <a href=\"{mailto}\">{email}</a>.</p>",
                mailto = esc(&mailto),
                email = esc(email),
            ));
            contact_actions.push_str(&format!(
                "<a href=\"{mailto}\">Send an email</a>",
                mailto = esc(&mailto),
            ));
        }
        None => contact_block.push_str(
            "<p class=\"muted\">No contact address yet. Check back here when one is ready.</p>",
        ),
    }
    if let Some(url) = GITHUB_URL {
        contact_block.push_str(&format!(
            "<p>Source and issues on <a href=\"{url}\" rel=\"noopener noreferrer\" target=\"_blank\">GitHub</a>.</p>",
            url = esc(url),
        ));
        contact_actions.push_str(&format!(
            "<a href=\"{url}\" rel=\"noopener noreferrer\" target=\"_blank\">GitHub</a>",
            url = esc(url),
        ));
    }
    if !contact_actions.is_empty() {
        contact_block.push_str("<div class=\"about-actions\">");
        contact_block.push_str(&contact_actions);
        contact_block.push_str("</div>");
    }

    let mut donate_block = String::from(
        "<p class=\"lead\">Hosting, cover storage, and catalog work cost money. Donations are optional and go toward keeping the site up—not paywalled scores.</p>",
    );
    let mut has_donate = false;
    if let Some(url) = DONATE_URL {
        has_donate = true;
        donate_block.push_str(&format!(
            "<div class=\"about-actions\"><a href=\"{url}\">Donate</a></div>",
            url = esc(url),
        ));
    }
    let crypto = [
        ("BTC", DONATE_BTC),
        ("ETH", DONATE_ETH),
        ("XMR", DONATE_XMR),
    ];
    let crypto_rows: Vec<String> = crypto
        .iter()
        .filter_map(|(coin, addr)| {
            let addr = (*addr)?;
            Some(format!(
                "<div class=\"crypto-row\">\
                   <span class=\"coin\">{coin}</span>\
                   <code>{addr}</code>\
                   <div class=\"crypto-actions\">\
                     <button type=\"button\" data-copy=\"{addr}\">Copy address</button>\
                   </div>\
                 </div>",
                coin = esc(coin),
                addr = esc(addr),
            ))
        })
        .collect();
    if !crypto_rows.is_empty() {
        has_donate = true;
        donate_block.push_str(
            "<p class=\"muted\">Crypto donations (Monero, Ethereum, Bitcoin)—send only on the matching network.</p>",
        );
        donate_block.push_str("<div class=\"crypto-list\">");
        for row in crypto_rows {
            donate_block.push_str(&row);
        }
        donate_block.push_str("</div>");
    }
    if !has_donate {
        donate_block.push_str(
            "<p class=\"muted\">Donation link and crypto addresses coming soon.</p>",
        );
    }

    format!(
        "<h1 class=\"page\">FAQ, contact &amp; donations</h1>\
         <p class=\"lead\">Answers to common questions, how to reach us, and how to keep SourceCritic online.</p>\
         <section class=\"about-block\" aria-labelledby=\"faq-heading\">\
           <h2 id=\"faq-heading\">FAQ</h2>\
           <p class=\"lead\">Short answers. For the math, see <a href=\"/score/\">how the score works</a>.</p>\
           <div class=\"faq\">\
             <details open>\
               <summary>What is SourceCritic?</summary>\
               <p>SourceCritic aggregates critic reviews into a single 0–100 score. Unlike a plain average, each outlet is judged against <em>its own</em> scoring habit, so a soft 70 and a harsh 70 are not treated as the same signal.</p>\
             </details>\
             <details>\
               <summary>How is the score calculated?</summary>\
               <p>Every review is normalized to 0–100, compared to that outlet&apos;s history as a $z$-score, mapped back to 0–100, then averaged one vote per outlet. The full walkthrough is on <a href=\"/score/\">The score</a>.</p>\
             </details>\
             <details>\
               <summary>Where do the reviews come from?</summary>\
               <p>Game metadata (name, platforms, dates, covers, synopsis) comes from the official IGDB API, powered by Twitch, or from RAWG when IGDB credentials are not configured. We do not copy covers or synopsis text from Metacritic or the PlayStation Store. Reviews are collected from each outlet: RSS and sitemap URLs listed in outlets.json, plus schema.org Review markup (reviewRating, author, datePublished). The crawler identifies itself as SourceCriticBot/1.0 (+https://sourcecritic.freedxm.com/about/), respects robots.txt, waits a few seconds between requests to the same domain, and keeps a cache. Only the score, a one- or two-sentence excerpt, the date, the author, and the canonical URL are stored. A review is attached to a game by normalized title plus IGDB id; ambiguous matches wait in review_queue.json. OpenCritic and Metacritic are not requested. OpenCritic Recommend % and Metacritic Users are not shown.</p>\
             </details>\
             <details>\
               <summary>Can I submit or correct a review?</summary>\
               <p>{contact_faq}</p>\
             </details>\
             <details>\
               <summary>Is SourceCritic affiliated with Metacritic or the outlets?</summary>\
               <p>No. SourceCritic is an independent project. Outlet names and scores belong to their publishers; we only re-score and present them under our method.</p>\
             </details>\
           </div>\
         </section>\
         <section class=\"about-block\" id=\"contact\" aria-labelledby=\"contact-heading\">\
           <h2 id=\"contact-heading\">Contact</h2>\
           {contact_block}\
           <h3>Lançamentos e notas da semana</h3>\
           <p class=\"muted\">Weekly releases and scores. The form opens the newsletter provider; nothing is stored on SourceCritic.</p>\
           {newsletter}\
         </section>\
         <section class=\"about-block\" id=\"donate\" aria-labelledby=\"donate-heading\">\
           <h2 id=\"donate-heading\">Donations</h2>\
           {donate_block}\
         </section>",
        contact_faq = contact_faq,
        contact_block = contact_block,
        newsletter = newsletter_form(),
        donate_block = donate_block,
    )
}

fn port_chip(key: &str, label: &str, score: i64, pressed: bool) -> String {
    format!(
        "<button type=\"button\" data-port=\"{key}\" aria-pressed=\"{pressed}\" aria-label=\"{label}, score {score}\">\
           <span class=\"port-name\">{label}</span>\
           <span class=\"port-score\">{score}</span>\
         </button>",
        key = esc(key),
        label = esc(label),
        score = score,
        pressed = if pressed { "true" } else { "false" },
    )
}

/// Mean mapped score for reviews of one port. Reviews with no port tag count on every port.
fn port_stats(game: &GameView, port_keys: &[String], port: Option<&str>) -> (i64, i64, usize) {
    let mut raw_sum = 0.0;
    let mut mapped_sum = 0.0;
    let mut n = 0usize;
    for review in game.reviews.iter().filter(|review| is_public_outlet(&review.outlet_slug)) {
        let mut keys: Vec<String> = review
            .platforms
            .iter()
            .map(|platform| platform_key(platform))
            .filter(|key| !key.is_empty())
            .collect();
        if keys.is_empty() {
            keys = port_keys.to_vec();
        }
        if let Some(port) = port {
            if !keys.iter().any(|key| key == port) {
                continue;
            }
        }
        raw_sum += review.raw_score_norm;
        mapped_sum += crate::engine::map_z(review.z_score, game.mu_global);
        n += 1;
    }
    if n == 0 {
        return (0, 0, 0);
    }
    let count = n as f64;
    (
        (mapped_sum / count).round() as i64,
        (raw_sum / count).round() as i64,
        n,
    )
}

fn method_page() -> String {
    let mut body = String::from(
        "<h1 class=\"page\">How the score is calculated</h1>\
         <p class=\"lead\">Pipeline: put every review on 0–100, measure how far it sits from <em>that outlet&apos;s</em> habit, convert each deviation back to 0–100, then average those mapped scores. Math first, then the code.</p>\
         <div class=\"walk\">",
    );

    body.push_str(&step(
        1,
        "Put every score on one scale",
        "Reviews arrive as 8/10, 4/5, or 80/100. Convert every review to the same continuous scale from 0 to 100 <em>before</em> any mean, spread, or $z$-score. Everything below uses these $x$ values only—never mixed raw scales.",
        r"$$
x = \frac{\mathrm{raw}}{\mathrm{scale}_{\max}} \times 100
$$
$$
\frac{8}{10}\times 100 = 80,\qquad \frac{4}{5}\times 100 = 80
$$",
        "def normalize(score, scale):\n    return score / scale * 100",
    ));

    body.push_str(&step(
        2,
        "Estimate how each outlet usually scores",
        "An <em>outlet</em> is a review site—call one of them $j$. Every score here is already on 0–100 from step 1; we write those scores as $x$.<br><br>First look at the whole catalog: $\\mu_{\\mathrm{global}}$ is the average of every $x$, and $\\sigma_{\\mathrm{global}}$ is how spread out those catalog scores are. That pair is the “typical critic” baseline.<br><br>Then look only at outlet $j$. Let $n_j$ be how many reviews it has, $\\mu_j$ the average of its own scores, and $\\sigma_j$ how spread out those scores are. Blend $\\mu_j$ and $\\sigma_j$ toward the catalog baseline so a thin history cannot dominate: $C = 30$ is how hard the catalog pulls. The results are $\\mu'_j$ (adjusted usual score) and $\\sigma'_j$ (adjusted spread, never below 5). Those two numbers are the habit used in the next step.",
        r"Catalog baseline — mean and spread of every $x$:
$$
\mu_{\mathrm{global}} = \mathrm{mean}(x),\qquad
\sigma_{\mathrm{global}} = \mathrm{std}(x)
$$
Outlet $j$ — blend its own mean $\mu_j$ and spread $\sigma_j$ toward that baseline, weighted by its review count $n_j$:
$$
\mu'_j = \frac{n_j\,\mu_j + C\,\mu_{\mathrm{global}}}{n_j + C}
$$
$$
\sigma'_j = \max\!\left(5,\;\frac{n_j\,\sigma_j + C\,\sigma_{\mathrm{global}}}{n_j + C}\right)
$$
$$
C = 30
$$",
        "# Catalog baseline from every review\nmu_global, sigma_global = mean_std(all_x)\n\n# Blend outlet j toward that baseline\n# n = how many reviews this outlet has (n_j)\ndef outlet_habit(n, mu, sigma, mu_global, sigma_global):\n    mu_adj = (n * mu + 30 * mu_global) / (n + 30)\n    sigma_adj = max(5, (n * sigma + 30 * sigma_global) / (n + 30))\n    return mu_adj, sigma_adj",
    ));

    body.push_str(&step(
        3,
        "Deviation vs the same outlet",
        "For each review, compute how far that score sits from <em>the same outlet&apos;s</em> adjusted habit—not from the global mean, and not from other outlets. $Z &gt; 0$ means better than that outlet usually gives. $Z &lt; 0$ means worse.",
        r"$$
Z_{i,j} = \frac{x_{i,j} - \mu'_j}{\sigma'_j}
$$
One $Z$ per outlet covering the game. Do not average the $Z$ values yet.",
        "def z(score, mean_adj, spread_adj):\n    # score and mean_adj are both on the 0-100 scale from step 1.\n    return (score - mean_adj) / spread_adj",
    ));

    body.push_str(&step(
        4,
        "Convert each deviation to 0–100",
        "Map <em>each</em> outlet $Z$ onto the published scale before mixing outlets. The center is the catalog mean $\\mu_{\\mathrm{global}}$ from step 2—so a typical review for its outlet ($Z=0$) lands near how critics score games overall, not at a fixed 50. Width $\\sigma_{\\mathrm{target}} = 10$ (each $+1$ in $Z$ moves that outlet&apos;s score by 10 points). Clamp anything outside $0$–$100$.",
        r"For each outlet $j$ covering game $i$:
$$
\mathrm{Score}_{i,j} = \mathrm{clamp}\!\left(\mu_{\mathrm{global}} + Z_{i,j}\cdot\sigma_{\mathrm{target}},\; 0,\; 100\right)
$$
$$
\sigma_{\mathrm{target}} = 10
$$
Examples when $\mu_{\mathrm{global}} = 65$:
$$
Z=0 \Rightarrow 65,\qquad
Z=1 \Rightarrow 75,\qquad
Z=-1.5 \Rightarrow 50
$$",
        "SIGMA_TARGET = 10  # +1 Z => +10 points for that outlet\n\ndef map_z(z, mu_global):\n    # Typical review (Z=0) lands on the catalog mean, not 50.\n    return min(100, max(0, mu_global + z * SIGMA_TARGET))",
    ));

    body.push_str(&step(
        5,
        "Average the converted scores",
        "Only now combine outlets: average the mapped 0–100 scores from step 4. One vote per outlet. Do not average raw scores, and do not average the $Z$ values first.",
        r"$$
\mathrm{Score}_i = \frac{\mathrm{Score}_{i,1} + \mathrm{Score}_{i,2} + \cdots + \mathrm{Score}_{i,N}}{N}
$$
$$
\mathrm{Score}_i = \mathrm{mean}\!\left(\mathrm{map\_z}(Z_{i,1}),\;\ldots,\;\mathrm{map\_z}(Z_{i,N})\right)
$$",
        "def game_score(zs):\n    # Convert each outlet Z first, then average the 0-100 scores.\n    return sum(map_z(z) for z in zs) / len(zs)",
    ));

    body.push_str(&step(
        6,
        "Why this is fair",
        "Setup: the same game gets a raw $70$ from two outlets. A simple average treats both as identical, so the game score is $70$. That is wrong if one outlet usually scores near $90$ and the other usually scores near $40$. The $70$ from the generous outlet is a <em>bad</em> review for them. The $70$ from the harsh outlet is a <em>good</em> review for them.<br><br>Important: step 4 does <em>not</em> take the raw $70$ and nudge it up or down. It asks “how unusual was this for that outlet?” and places that unusualness around the catalog mean $\\mu_{\\mathrm{global}}$. In this example the catalog mean is $65$.",
        r"Both outlets give raw score $70$. After their histories are adjusted (step 2):
$$
\mu'_{\mathrm{lenient}} = 81.56,\quad \sigma'_{\mathrm{lenient}} = 9.88
$$
$$
\mu'_{\mathrm{harsh}} = 48.60,\quad \sigma'_{\mathrm{harsh}} = 10.73
$$
Deviation vs <em>that same outlet</em> (step 3):
$$
Z_{\mathrm{lenient}} = \frac{70 - 81.56}{9.88} = -1.17
\quad\text{(below their usual scores)}
$$
$$
Z_{\mathrm{harsh}} = \frac{70 - 48.60}{10.73} = 1.99
\quad\text{(above their usual scores)}
$$
Map each $Z$ around $\mu_{\mathrm{global}}=65$ (step 4)—not onto the raw $70$:
$$
\mathrm{Score}_{\mathrm{lenient}} = 65 + (-1.17)\cdot 10 = 53.30
$$
$$
\mathrm{Score}_{\mathrm{harsh}} = 65 + 1.99\cdot 10 = 84.90
$$
Then average (step 5):
$$
\mathrm{Score} = \frac{53.30 + 84.90}{2} = 69.10
$$
The generous outlet's soft $70$ maps to $53.30$ (below the catalog mean). The harsh outlet's strong $70$ maps to $84.90$ (above it). Relative to each other, harsh praise pulls the game <em>up</em> and soft praise pulls it <em>down</em>. Neither mapped value is “the raw $70$ adjusted.” The simple mean ($70$) cannot see that. The SourceCritic score ($69.10$) can.",
        "mu_global = 65  # catalog mean in this example\nz_lenient = (70 - 81.56) / 9.88  # -1.17\nz_harsh   = (70 - 48.60) / 10.73 #  1.99\n\nscore_lenient = map_z(z_lenient, mu_global)  # 53.30\nscore_harsh   = map_z(z_harsh, mu_global)    # 84.90\n\ngame_score([z_lenient, z_harsh], mu_global)  # 69.10",
    ));

    body.push_str("</div>");
    body
}

fn ok(section: Section, title: &str, body: &str) -> (StatusCode, Html<String>) {
    (StatusCode::OK, document(section, title, body, None, &Seo::default()))
}

fn internal(error: String) -> (StatusCode, Html<String>) {
    eprintln!("internal error: {error}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        document(
            Section::Other,
            "Error",
            "<h1 class=\"page\">Internal error.</h1>",
            None,
            &Seo::default(),
        ),
    )
}

struct Seo {
    description: String,
    image: Option<String>,
    json_ld: String,
    extra_head: String,
}

impl Default for Seo {
    fn default() -> Self {
        Self {
            description: String::new(),
            image: None,
            json_ld: String::new(),
            extra_head: String::new(),
        }
    }
}

fn ad_unit(class: &str, min_height: &str) -> String {
    let client = ADSENSE_CLIENT.unwrap_or("");
    if client.is_empty() {
        return String::new();
    }
    format!(
        "<aside class=\"{class} ad-unit\" data-ad-client=\"{client}\" aria-label=\"Advertisement\">\
           <span class=\"site-ad-label\">Ad</span>\
           <div class=\"site-ad-slot\" style=\"min-height:{min_height}\">\
             <ins class=\"adsbygoogle\" style=\"display:block;min-height:{min_height}\" \
               data-ad-client=\"{client}\" data-ad-slot=\"8380670310\" \
               data-ad-format=\"rectangle\" data-full-width-responsive=\"false\"></ins>\
           </div>\
         </aside>",
        class = class,
        client = esc(client),
        min_height = min_height,
    )
}

fn ad_slot_html() -> String {
    ad_unit("site-ad-rail", "600px").replace(
        "class=\"site-ad-rail ad-unit\"",
        "class=\"site-ad-rail ad-unit\" id=\"site-ad\"",
    )
}

fn document(section: Section, title: &str, body: &str, path: Option<&str>, seo: &Seo) -> Html<String> {
    let (games, outlets, score, about) = match section {
        Section::Games => (" aria-current=\"page\"", "", "", ""),
        Section::Outlets => ("", " aria-current=\"page\"", "", ""),
        Section::Score => ("", "", " aria-current=\"page\"", ""),
        Section::About => ("", "", "", " aria-current=\"page\""),
        Section::Other => ("", "", "", ""),
    };
    let katex = if matches!(section, Section::Score | Section::Outlets | Section::About) {
        r#"<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.css" crossorigin="anonymous">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.js" crossorigin="anonymous"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/contrib/auto-render.min.js" crossorigin="anonymous"
  onload="renderMathInElement(document.body,{delimiters:[{left:'$$',right:'$$',display:true},{left:'$',right:'$',display:false}],throwOnError:false});"></script>"#
    } else {
        ""
    };
    let ad = ad_slot_html();
    let canonical = canonical_url(section, path);
    let full_title = if title.contains("SourceCritic") {
        title.to_string()
    } else {
        format!("{title} | SourceCritic")
    };
    let description = if seo.description.is_empty() {
        format!("{full_title}. Critic scores adjusted for each outlet's scoring habit.")
    } else {
        seo.description.clone()
    };
    let image = seo
        .image
        .clone()
        .unwrap_or_else(|| format!("{SITE_ORIGIN}/og-default.png"));
    let json_ld = if seo.json_ld.is_empty() {
        String::new()
    } else {
        format!(
            "<script type=\"application/ld+json\">{}</script>",
            seo.json_ld
        )
    };
    let ga = GA_MEASUREMENT_ID.unwrap_or("");
    Html(format!(
        "<!DOCTYPE html>\
         <html lang=\"en\">\
         <head>\
           <meta charset=\"utf-8\">\
           <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
           <title>{title}</title>\
           <meta name=\"description\" content=\"{description}\">\
           <link rel=\"canonical\" href=\"{canonical}\">\
           <link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">\
           <link rel=\"apple-touch-icon\" href=\"/apple-touch-icon.png\">\
           <meta property=\"og:title\" content=\"{title}\">\
           <meta property=\"og:description\" content=\"{description}\">\
           <meta property=\"og:image\" content=\"{image}\">\
           <meta property=\"og:url\" content=\"{canonical}\">\
           <meta property=\"og:type\" content=\"website\">\
           <meta name=\"twitter:card\" content=\"summary_large_image\">\
           <meta name=\"twitter:title\" content=\"{title}\">\
           <meta name=\"twitter:description\" content=\"{description}\">\
           <meta name=\"twitter:image\" content=\"{image}\">\
           {json_ld}\
           {extra_head}\
           <script>\
             window.dataLayer=window.dataLayer||[];\
             function gtag(){{dataLayer.push(arguments);}}\
             gtag('consent','default',{{\
               ad_storage:'denied',ad_user_data:'denied',ad_personalization:'denied',\
               analytics_storage:'denied',wait_for_update:500\
             }});\
           </script>\
           <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\
           <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\
           <link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&family=IBM+Plex+Serif:wght@600&display=swap\" rel=\"stylesheet\">\
           {katex}\
           <style>{CSS}</style>\
           <script>{SITE_JS}</script>\
         </head>\
         <body data-ga=\"{ga}\">\
           <div class=\"page-shell\">\
             <main>\
               <header>\
                 <a class=\"brand\" href=\"/\">SourceCritic</a>\
                 <nav>\
                   <a href=\"/\"{games}>Games</a>\
                   <a href=\"/outlets/\"{outlets}>Outlets</a>\
                   <a href=\"/score/\"{score}>The score</a>\
                   <a href=\"/about/\"{about}>About</a>\
                 </nav>\
               </header>\
               {cookie}\
               {body}\
               {footer}\
             </main>\
             {ad}\
           </div>\
         </body>\
         </html>",
        title = esc(&full_title),
        description = esc(&description),
        canonical = esc(&canonical),
        image = esc(&image),
        json_ld = json_ld,
        extra_head = seo.extra_head,
        ga = esc(ga),
        games = games,
        outlets = outlets,
        score = score,
        about = about,
        footer = site_footer(),
        cookie = cookie_banner(),
        ad = ad,
    ))
}

fn canonical_url(section: Section, path: Option<&str>) -> String {
    let raw = match path {
        Some(p) if p == "/" || p.is_empty() => "/".to_string(),
        Some(p) => p.to_string(),
        None => match section {
            Section::Games => "/".to_string(),
            Section::Outlets => "/outlets/".to_string(),
            Section::Score => "/score/".to_string(),
            Section::About => "/about/".to_string(),
            Section::Other => "/".to_string(),
        },
    };
    let with_slash = if raw == "/" || raw.ends_with('/') {
        raw
    } else {
        format!("{raw}/")
    };
    format!("{SITE_ORIGIN}{with_slash}")
}

/// Render every public HTML page for the static export.
/// Paths are relative (`index.html`, `game/{slug}/index.html`, …).
pub fn render_all_pages(db: &crate::db::Database) -> Result<Vec<(String, String)>, String> {
    let mut pages = Vec::new();

    let mut games = db.list_games().map_err(|e| e.to_string())?;
    let platforms = collect_platforms(&games);
    let years = collect_years(&games);
    sort_games(&mut games, "score");
    let filters = GameFilters {
        platform: "all".into(),
        year: "all".into(),
        min_reviews: DEFAULT_MIN_REVIEWS,
    };
    let catalog_total = games.len();
    let filtered: Vec<GameView> = games
        .iter()
        .filter(|game| game_matches(game, &filters))
        .cloned()
        .collect();
    let filtered_total = filtered.len();
    let page_count = filtered_total.div_ceil(PAGE_SIZE).max(1);
    for page in 1..=page_count {
        let start = (page - 1) * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(filtered_total);
        let slice = if filtered_total == 0 {
            &filtered[..]
        } else {
            &filtered[start..end]
        };
        let path = page_href(page);
        let file = if page == 1 {
            "index.html".to_string()
        } else {
            format!("page/{page}/index.html")
        };
        let description = format!(
            "Notas da crítica para {filtered_total} jogos, ajustadas pelo hábito de cada veículo. Página {page} de {page_count}."
        );
        pages.push((
            file,
            document(
                Section::Games,
                "SourceCritic: notas e reviews de jogos da crítica",
                &game_list(
                    slice,
                    "score",
                    &filters,
                    &platforms,
                    &years,
                    catalog_total,
                    filtered_total,
                    page,
                    page_count,
                ),
                Some(&path),
                &Seo {
                    description,
                    image: Some(format!("{SITE_ORIGIN}/og-default.png")),
                    json_ld: item_list_json("SourceCritic games", slice),
                    extra_head: pager_head(page, page_count),
                },
            )
            .0,
        ));
    }

    let mut used_public: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for game in &games {
        let mut detailed = db
            .get_game(&game.slug)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing game {}", game.slug))?;
        sort_game_reviews(&mut detailed.reviews, "name", detailed.mu_global);
        let mut slug = public_slug(&detailed.slug, &detailed.title);
        if !used_public.insert(slug.clone()) {
            slug = detailed.slug.clone();
        }
        let path = format!("/game/{slug}/");
        pages.push((
            format!("game/{slug}/index.html"),
            document(
                Section::Other,
                &format!("{}: nota da crítica e reviews", detailed.title),
                &game_page(&detailed, "name"),
                Some(&path),
                &game_seo(&detailed, &slug),
            )
            .0,
        ));
        if slug != detailed.slug {
            pages.push((
                format!("game/{}/index.html", detailed.slug),
                redirect_page(&path),
            ));
        }
    }
    pages.push((
        "game/baldur-s-gate-3/index.html".into(),
        redirect_page("/game/baldurs-gate-3/"),
    ));

    let mut outlets = db.list_outlets().map_err(|e| e.to_string())?;
    outlets.retain(|outlet| is_public_outlet(&outlet.slug));
    sort_outlets(&mut outlets, "reviews");
    pages.push((
        "outlets/index.html".into(),
        document(
            Section::Outlets,
            "Outlets | SourceCritic",
            &outlet_list(&outlets, "reviews"),
            Some("/outlets/"),
            &Seo {
                description: format!(
                    "{} critic outlets and the scoring habit behind each SourceCritic score.",
                    outlets.len()
                ),
                ..Seo::default()
            },
        )
        .0,
    ));

    for outlet in &outlets {
        let mut detailed = db
            .get_outlet(&outlet.slug)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing outlet {}", outlet.slug))?;
        sort_outlet_reviews(&mut detailed.reviews, "name", detailed.mu_global);
        let path = format!("/outlet/{}/", detailed.slug);
        pages.push((
            format!("outlet/{}/index.html", detailed.slug),
            document(
                Section::Outlets,
                &format!("{} reviews | SourceCritic", detailed.name),
                &outlet_page(&detailed, "name"),
                Some(&path),
                &Seo {
                    description: format!(
                        "{} has {} reviews in SourceCritic. Raw mean {}, shrunk mean {}.",
                        detailed.name,
                        detailed.n,
                        fmt_num(detailed.mu),
                        fmt_num(detailed.mu_prime)
                    ),
                    ..Seo::default()
                },
            )
            .0,
        ));
    }

    pages.push((
        "score/index.html".into(),
        document(
            Section::Score,
            "How the SourceCritic score is calculated",
            &method_page(),
            Some("/score/"),
            &Seo {
                description: "How SourceCritic turns each outlet's habit into a 0–100 score: normalize, z-score, map, then average.".into(),
                ..Seo::default()
            },
        )
        .0,
    ));
    pages.push((
        "about/index.html".into(),
        document(
            Section::About,
            "About SourceCritic: methodology and sources",
            &about_page(),
            Some("/about/"),
            &Seo {
                description: "Where SourceCritic scores come from, how reviews are collected, and how to get in touch.".into(),
                ..Seo::default()
            },
        )
        .0,
    ));
    pages.push((
        "privacy/index.html".into(),
        document(
            Section::Other,
            "Privacy policy | SourceCritic",
            &legal_page(true),
            Some("/privacy/"),
            &Seo {
                description: "What SourceCritic collects, how AdSense and Analytics cookies work, and your LGPD rights.".into(),
                ..Seo::default()
            },
        )
        .0,
    ));
    pages.push((
        "terms/index.html".into(),
        document(
            Section::Other,
            "Terms of use | SourceCritic",
            &legal_page(false),
            Some("/terms/"),
            &Seo {
                description: "Terms for using SourceCritic. Reviews belong to the outlets that published them.".into(),
                ..Seo::default()
            },
        )
        .0,
    ));
    pages.extend(discovery_pages(&games, &outlets));
    pages.push((
        "404.html".into(),
        document(
            Section::Other,
            "Not found",
            "<h1 class=\"page\">Page not found.</h1><p class=\"lead\"><a href=\"/\">Back to games</a></p>",
            Some("/"),
            &Seo::default(),
        )
        .0,
    ));

    Ok(pages)
}

pub fn games_index_json(games: &[GameView]) -> String {
    let mut items = Vec::new();
    for game in games {
        let platforms: Vec<String> = game.platforms.iter().map(|platform| platform_key(platform)).collect();
        let labels = platforms
            .iter()
            .map(|key| platform_label(key))
            .collect::<Vec<_>>()
            .join(", ");
        let stem = game
            .cover_path
            .as_deref()
            .and_then(|path| path.rsplit('/').next())
            .and_then(|file| file.rsplit_once('.').map(|(name, _)| name.to_string()));
        let cover = stem.map(|stem| format!("/covers/{stem}-112.webp"));
        items.push(serde_json::json!({
            "slug": public_slug(&game.slug, &game.title),
            "title": game.title,
            "score": game.agg_score.unwrap_or(0.0),
            "mean": game.simple_mean.unwrap_or(0.0),
            "reviews": game.review_count,
            "year": game.release_year,
            "platforms": platforms,
            "platform_labels": labels,
            "cover": cover,
            "provisional": game.review_count < 5,
        }));
    }
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
}


fn esc(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn is_public_outlet(slug: &str) -> bool {
    !matches!(slug, "metacritic-users" | "opencritic-recommend")
}

fn redirect_page(target: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <link rel=\"canonical\" href=\"{origin}{target}\">\
         <meta http-equiv=\"refresh\" content=\"0; url={target}\">\
         <title>Redirect</title></head><body>\
         <p><a href=\"{target}\">Continue</a></p></body></html>",
        origin = SITE_ORIGIN,
        target = esc(target),
    )
}

fn game_seo(game: &GameView, slug: &str) -> Seo {
    let score = fmt_score(game.agg_score.unwrap_or(0.0));
    let year = game
        .release_year
        .map(|year| format!(" ({year})"))
        .unwrap_or_default();
    let platforms = game
        .platforms
        .iter()
        .map(|platform| platform_label(&platform_key(platform)))
        .collect::<Vec<_>>()
        .join(", ");
    let platform_bit = if platforms.is_empty() {
        String::new()
    } else {
        format!(" Platforms: {platforms}.")
    };
    let description = format!(
        "{}{year} has a SourceCritic score of {score} from {} critic reviews.{platform_bit}",
        game.title, game.review_count
    );
    let image = game.cover_path.as_deref().map(|path| {
        let stem = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .rsplit_once('.')
            .map(|(name, _)| name)
            .unwrap_or(path);
        format!("{SITE_ORIGIN}/covers/{stem}-400.webp")
    });
    Seo {
        description,
        image,
        json_ld: game_json_ld(game, slug),
        extra_head: String::new(),
    }
}

fn game_json_ld(game: &GameView, slug: &str) -> String {
    let mut reviews = Vec::new();
    for review in game.reviews.iter().filter(|review| is_public_outlet(&review.outlet_slug)) {
        let url = review
            .review_url
            .as_deref()
            .filter(|url| url.starts_with("http://") || url.starts_with("https://"));
        let rating = fmt_score(review.raw_score_norm);
        let mut item = serde_json::json!({
            "@type": "Review",
            "author": {"@type": "Organization", "name": review.outlet_name},
            "reviewRating": {
                "@type": "Rating",
                "ratingValue": rating,
                "bestRating": "100",
                "worstRating": "0"
            }
        });
        if let Some(url) = url {
            item["url"] = serde_json::Value::String(url.to_string());
        }
        reviews.push(item);
    }
    let mut value = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "VideoGame",
        "name": game.title,
        "url": format!("{SITE_ORIGIN}/game/{slug}/"),
        "description": game.synopsis.clone().unwrap_or_default(),
        "gamePlatform": game.platforms.iter().map(|platform| platform_label(&platform_key(platform))).collect::<Vec<_>>(),
        "review": reviews,
    });
    if let Some(year) = game.release_year {
        value["datePublished"] = serde_json::Value::String(year.to_string());
    }
    if game.review_count >= 5 {
        value["aggregateRating"] = serde_json::json!({
            "@type": "AggregateRating",
            "ratingValue": fmt_score(game.agg_score.unwrap_or(0.0)),
            "bestRating": "100",
            "worstRating": "0",
            "ratingCount": game.review_count
        });
    }
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".into())
}

fn fmt_score(value: f64) -> String {
    format!("{}", value.round() as i64)
}

fn fmt_z(value: f64) -> String {
    format!("{:.2}", value)
}

fn fold_accents(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

fn slugify_title(title: &str) -> String {
    let mut cleaned = String::new();
    for ch in title.chars() {
        if matches!(ch, '\'' | '’' | 'ʼ' | '`') {
            continue;
        }
        cleaned.push(ch);
    }
    let folded = fold_accents(&cleaned.to_lowercase());
    let mut slug = String::new();
    let mut dash = false;
    for ch in folded.chars() {
        if ch.is_ascii_alphanumeric() {
            if dash && !slug.is_empty() {
                slug.push('-');
            }
            dash = false;
            slug.push(ch);
        } else {
            dash = true;
        }
    }
    slug
}

/// Apostrophes are deleted before hyphenating (`baldurs-gate-3`, `blueys-happy-snaps`).
/// Existing slugs change only when that rule reproduces the title slug.
fn public_slug(slug: &str, title: &str) -> String {
    let from_title = slugify_title(title);
    let collapsed = slug.replace("-s-", "s-");
    if from_title == collapsed && collapsed != slug {
        collapsed
    } else {
        slug.to_string()
    }
}

fn fmt_num(value: f64) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    format!("{rounded:.4}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Subtitle figure color from score vs mean: red if score > mean, purple if score < mean.
fn cmp_num_html(label: &str, score: f64, mean: f64) -> String {
    let label = esc(label);
    if !score.is_finite() || !mean.is_finite() || (score - mean).abs() < f64::EPSILON {
        return label;
    }
    let class = if score > mean { "cmp-low" } else { "cmp-high" };
    format!("<span class=\"{class}\">{label}</span>")
}

fn score_compact_html(value: f64) -> String {
    let full = fmt_score(value);
    format!("<span class=\"score\">{full}</span>", full = esc(&full))
}

/// Metacritic descriptions often end with leftover tags like `[Nintendo]` or `[GTA VI]`.
fn clean_synopsis(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    loop {
        let trimmed = text.trim_end();
        let Some(close) = trimmed.rfind(']') else {
            text = trimmed.to_string();
            break;
        };
        let Some(open) = trimmed[..close].rfind('[') else {
            text = trimmed.to_string();
            break;
        };
        if close + 1 != trimmed.len() {
            text = trimmed.to_string();
            break;
        }
        let tag = &trimmed[open + 1..close];
        if tag.is_empty() || tag.chars().any(|ch| ch == '[' || ch == ']') {
            text = trimmed.to_string();
            break;
        }
        text = trimmed[..open].trim_end().to_string();
    }
    text
}

#[cfg(test)]
fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for ch in value.chars().flat_map(|ch| ch.to_lowercase()) {
        let ch = match ch {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        };
        if ch.is_ascii_alphanumeric() {
            if dash && !slug.is_empty() {
                slug.push('-');
            }
            dash = false;
            slug.push(ch);
        } else {
            dash = true;
        }
    }
    slug
}

fn legal_page(privacy: bool) -> String {
    if privacy {
        String::from(
            "<h1 class=\"page\">Privacy policy</h1>\
             <p class=\"lead\">SourceCritic is a static site. This notice covers the LGPD (Brazil) and the GDPR (EEA and UK).</p>\
             <section class=\"about-block\"><h2>English</h2>\
             <p>We collect the least we can. The pages themselves do not ask for an account. If you subscribe to the newsletter, the email address goes to the provider named in the form (Buttondown, MailerLite, or Beehiiv) and is used only to send the weekly releases and scores. If you email us, we keep that message to reply.</p>\
             <p>Advertising (Google AdSense) and measurement (Google Analytics) cookies stay off until you accept them. Visitors in the EEA and the UK see Google's certified Privacy &amp; messaging CMP. Everyone else sees our banner. Consent Mode v2 defaults to denied for ad_storage, ad_user_data, ad_personalization, and analytics_storage. Rejecting leaves those tags unloaded.</p>\
             <p>Purpose: show critic scores, fund hosting with ads, and understand which pages are used. Legal bases: consent for cookies, and legitimate interest for the publishing of publicly available review scores.</p>\
             <p>You can ask for access, correction, deletion, or portability of personal data we hold, and you can withdraw cookie consent by clearing site data. Contact <a href=\"mailto:joeltondbrandao@freedxm.com\">joeltondbrandao@freedxm.com</a>.</p>\
             <p>Reviews belong to the outlets that wrote them. SourceCritic shows the score, a short excerpt of one or two sentences, and a link to the original. We do not republish the full text.</p>\
             </section>\
             <section class=\"about-block\"><h2>Português</h2>\
             <p>O SourceCritic é um site estático. Não pedimos conta. Se você assina a newsletter, o e-mail vai para o serviço indicado no formulário e serve só para os lançamentos e as notas da semana. Se você escreve para o contato, guardamos a mensagem para responder.</p>\
             <p>Cookies de publicidade (Google AdSense) e de medição (Google Analytics) ficam bloqueados até o aceite. Visitantes da Europa e do Reino Unido veem a CMP certificada do Google (Privacy &amp; messaging, do AdSense). Os demais veem o nosso aviso. O Consent Mode v2 começa negado para ad_storage, ad_user_data, ad_personalization e analytics_storage.</p>\
             <p>Finalidade: exibir notas da crítica, manter o site com anúncios e saber quais páginas são lidas. Bases: consentimento para cookies e legítimo interesse para publicar notas já públicas.</p>\
             <p>Pela LGPD você pode pedir acesso, correção, exclusão, portabilidade e informação sobre o uso. Para retirar o consentimento, apague os dados do site neste navegador. Contato: <a href=\"mailto:joeltondbrandao@freedxm.com\">joeltondbrandao@freedxm.com</a>.</p>\
             <p>As reviews pertencem aos veículos. O site mostra a nota, um trecho curto de uma ou duas frases e o link. Não republicamos o texto integral.</p>\
             </section>"
        )
    } else {
        String::from(
            "<h1 class=\"page\">Terms of use</h1>\
             <p class=\"lead\">Using SourceCritic means you accept these terms.</p>\
             <section class=\"about-block\"><h2>English</h2>\
             <p>Scores are an editorial transformation of published critic reviews. They are not an official score from any outlet, Metacritic, or OpenCritic. The method is described on <a href=\"/score/\">how the score works</a> and <a href=\"/about/\">about</a>.</p>\
             <p>Review text, scores, and names belong to the outlets. We display the score, a short excerpt, and a link. Do not treat an excerpt as the full review. Affiliate buttons (\"Onde comprar\") may earn a commission at no extra cost to you; prices are not shown until a store feed is connected.</p>\
             <p>The site is provided as is. We do not promise that every score, platform, or year is complete. For privacy requests, use the address on <a href=\"/privacy/\">privacy</a>.</p>\
             </section>\
             <section class=\"about-block\"><h2>Português</h2>\
             <p>As notas são uma leitura editorial de reviews já publicadas. Não são a nota oficial de um veículo, do Metacritic ou do OpenCritic. O método está em <a href=\"/score/\">como a nota é calculada</a> e em <a href=\"/about/\">sobre</a>.</p>\
             <p>O texto, a nota e o nome pertencem ao veículo. Mostramos a nota, um trecho curto e o link. Os botões \"Onde comprar\" podem gerar comissão sem custo extra; o preço em reais fica preparado para uma integração futura.</p>\
             <p>O site é oferecido no estado em que está. Pedidos sobre dados pessoais vão para o contato da página de <a href=\"/privacy/\">privacidade</a>.</p>\
             </section>"
        )
    }
}

fn buy_block(title: &str) -> String {
    let config = include_str!("../../data/affiliates.json");
    let Ok(value) = serde_json::from_str::<serde_json::Value>(config) else {
        return String::new();
    };
    let Some(stores) = value.get("stores").and_then(|item| item.as_array()) else {
        return String::new();
    };
    let query = urlencoding_min(title);
    let mut buttons = String::new();
    for store in stores {
        let name = store.get("name").and_then(|item| item.as_str()).unwrap_or("Store");
        let id = store.get("affiliate_id").and_then(|item| item.as_str()).unwrap_or("");
        let template = store.get("url_template").and_then(|item| item.as_str()).unwrap_or("");
        if template.is_empty() {
            continue;
        }
        let url = template
            .replace("{query}", &query)
            .replace("{affiliate_id}", id);
        let price = store.get("price_brl").and_then(|item| item.as_f64());
        let price_html = match price {
            Some(price) => format!("<small>R$ {price:.2}</small>"),
            None => "<small data-price-brl></small>".to_string(),
        };
        buttons.push_str(&format!(
            "<a href=\"{url}\" rel=\"sponsored nofollow noopener\" target=\"_blank\">{name}{price_html}</a>",
            url = esc(&url),
            name = esc(name),
            price_html = price_html,
        ));
    }
    let note = value
        .get("disclosure_pt")
        .and_then(|item| item.as_str())
        .unwrap_or("Links de afiliado: podemos receber comissão sem custo para você.");
    format!(
        "<section class=\"buy-block\"><h2>Onde comprar</h2>\
         <div class=\"buy-actions\">{buttons}</div>\
         <p class=\"buy-note\">{note}</p></section>",
        note = esc(note),
    )
}

fn urlencoding_min(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(*byte as char),
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn population_std(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|value| {
        let delta = value - mean;
        delta * delta
    }).sum::<f64>() / values.len() as f64;
    var.sqrt()
}

fn discovery_pages(games: &[GameView], outlets: &[crate::db::OutletView]) -> Vec<(String, String)> {
    let mut pages = Vec::new();
    let mut divisive: Vec<&GameView> = games
        .iter()
        .filter(|game| game.review_count >= 5)
        .collect();
    divisive.sort_by(|a, b| {
        let a_std = population_std(&a.reviews.iter().map(|review| review.raw_score_norm).collect::<Vec<_>>());
        let b_std = population_std(&b.reviews.iter().map(|review| review.raw_score_norm).collect::<Vec<_>>());
        b_std.partial_cmp(&a_std).unwrap_or(std::cmp::Ordering::Equal)
    });
    divisive.truncate(40);
    let lead_std = divisive.first().map(|game| {
        population_std(&game.reviews.iter().map(|review| review.raw_score_norm).collect::<Vec<_>>())
    });
    let intro = match (divisive.first(), lead_std) {
        (Some(game), Some(std)) => format!(
            "{} games with at least 5 reviews, ordered by how far the outlets sit from each other. {} leads, with a standard deviation of {:.1} points.",
            divisive.len(),
            esc(&game.title),
            std
        ),
        _ => "Not enough reviews yet to rank disagreements between outlets.".into(),
    };
    pages.push(list_page(
        "most-divisive/index.html",
        "/most-divisive/",
        "Most divisive games | SourceCritic",
        "Most divisive",
        &intro,
        &divisive,
    ));

    let mut gaps: Vec<&GameView> = games
        .iter()
        .filter(|game| game.review_count >= 5)
        .filter(|game| game.agg_score.is_some() && game.simple_mean.is_some())
        .collect();
    gaps.sort_by(|a, b| {
        let a_gap = (a.agg_score.unwrap_or(0.0) - a.simple_mean.unwrap_or(0.0)).abs();
        let b_gap = (b.agg_score.unwrap_or(0.0) - b.simple_mean.unwrap_or(0.0)).abs();
        b_gap.partial_cmp(&a_gap).unwrap_or(std::cmp::Ordering::Equal)
    });
    gaps.truncate(40);
    let gap_intro = if let Some(game) = gaps.first() {
        let gap = (game.agg_score.unwrap_or(0.0) - game.simple_mean.unwrap_or(0.0)).abs();
        format!(
            "Where the adjusted SourceCritic score moves furthest from a plain average. {} differs by {:.0} points.",
            esc(&game.title),
            gap
        )
    } else {
        "No scored games yet.".into()
    };
    pages.push(list_page(
        "disagreements/index.html",
        "/disagreements/",
        "Where SourceCritic disagrees with the average | SourceCritic",
        "Biggest gaps",
        &gap_intro,
        &gaps,
    ));

    let mut generous: Vec<&crate::db::OutletView> = outlets.iter().filter(|outlet| outlet.n >= 10).collect();
    generous.sort_by(|a, b| b.mu_prime.partial_cmp(&a.mu_prime).unwrap_or(std::cmp::Ordering::Equal));
    let mut harsh = generous.clone();
    harsh.sort_by(|a, b| a.mu_prime.partial_cmp(&b.mu_prime).unwrap_or(std::cmp::Ordering::Equal));
    generous.truncate(15);
    harsh.truncate(15);
    let rank_intro = match (generous.first(), harsh.first()) {
        (Some(high), Some(low)) => format!(
            "Habits already shrunk toward the catalog. {} scores highest on average (μ′ {}); {} scores lowest (μ′ {}).",
            esc(&high.name),
            fmt_num(high.mu_prime),
            esc(&low.name),
            fmt_num(low.mu_prime)
        ),
        _ => "Outlet habits appear once a publication has at least 10 reviews.".into(),
    };
    let mut body = format!(
        "<h1 class=\"page\">Outlet habits</h1><p class=\"lead\">{rank_intro}</p><h2>Most generous</h2><div id=\"outlet-list\">"
    );
    for outlet in &generous {
        body.push_str(&format!(
            "<a class=\"row\" href=\"/outlet/{slug}/\"><span><strong>{name}</strong><small>μ′ {mu}</small></span><span class=\"score\">{mu}</span></a>",
            slug = esc(&outlet.slug),
            name = esc(&outlet.name),
            mu = fmt_num(outlet.mu_prime),
        ));
    }
    body.push_str("</div><h2>Harshest</h2><div>");
    for outlet in &harsh {
        body.push_str(&format!(
            "<a class=\"row\" href=\"/outlet/{slug}/\"><span><strong>{name}</strong><small>μ′ {mu}</small></span><span class=\"score\">{mu}</span></a>",
            slug = esc(&outlet.slug),
            name = esc(&outlet.name),
            mu = fmt_num(outlet.mu_prime),
        ));
    }
    body.push_str("</div>");
    pages.push((
        "outlets/ranking/index.html".into(),
        document(
            Section::Outlets,
            "Most generous and harshest outlets | SourceCritic",
            &body,
            Some("/outlets/ranking/"),
            &Seo {
                description: rank_intro.replace("&amp;", "&"),
                json_ld: String::new(),
                image: None,
                extra_head: String::new(),
            },
        )
        .0,
    ));

    let best_platforms = [
        "ps5", "ps4", "ps3", "ps2", "ps1", "psp", "vita", "switch", "switch-2", "3ds", "ds",
        "wii", "wii-u", "gamecube", "n64", "pc", "xbox-series", "xbox-one", "xbox-360", "xbox", "ios",
    ];
    let mut best_index = String::from(
        "<h1 class=\"page\">Best of</h1><p class=\"lead\">Highest SourceCritic scores for a platform and a year, among games with at least 5 reviews.</p><div>",
    );
    let mut best_count = 0usize;
    let platform_keys = collect_platforms(games);
    let year_list = collect_years(games);
    for (key, label) in platform_keys.iter().filter(|(key, _)| best_platforms.contains(&key.as_str())) {
        for year in &year_list {
            let mut picked: Vec<&GameView> = games
                .iter()
                .filter(|game| game.review_count >= 5 && game.release_year == Some(*year))
                .filter(|game| game.platforms.iter().any(|platform| platform_key(platform) == *key))
                .collect();
            if picked.len() < 5 {
                continue;
            }
            picked.sort_by(|a, b| {
                b.agg_score
                    .partial_cmp(&a.agg_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            picked.truncate(25);
            let avg = picked.iter().filter_map(|game| game.agg_score).sum::<f64>() / picked.len() as f64;
            let leader = &picked[0];
            let intro = format!(
                "Melhores jogos de {label} em {year}. Os {n} primeiros têm nota média {avg}. {title} lidera com {score}.",
                n = picked.len(),
                avg = fmt_score(avg),
                title = esc(&leader.title),
                score = fmt_score(leader.agg_score.unwrap_or(0.0)),
            );
            let path = format!("/best/{key}/{year}/");
            pages.push(list_page(
                &format!("best/{key}/{year}/index.html"),
                &path,
                &format!("Melhores jogos de {label} em {year} | SourceCritic"),
                &format!("Melhores de {label} em {year}"),
                &intro,
                &picked,
            ));
            best_index.push_str(&format!(
                "<a class=\"row\" href=\"{path}\"><span><strong>Melhores jogos de {label} em {year}</strong></span></a>"
            ));
            best_count += 1;
        }
    }
    best_index.push_str("</div>");
    if best_count == 0 {
        best_index.push_str("<p class=\"empty\">No platform-year has five scored games yet.</p>");
    }
    pages.push((
        "best/index.html".into(),
        document(
            Section::Other,
            "Best games by platform and year | SourceCritic",
            &best_index,
            Some("/best/"),
            &Seo {
                description: "Lists of the highest SourceCritic scores by platform and year.".into(),
                ..Seo::default()
            },
        )
        .0,
    ));
    pages
}

fn list_page(
    file: &str,
    path: &str,
    title: &str,
    heading: &str,
    intro: &str,
    games: &[&GameView],
) -> (String, String) {
    let mut body = format!(
        "<h1 class=\"page\">{heading}</h1><p class=\"lead\">{intro}</p><div id=\"game-list\">",
        heading = esc(heading),
    );
    for game in games {
        let slug = public_slug(&game.slug, &game.title);
        body.push_str(&format!(
            "<a class=\"row\" href=\"/game/{slug}/\"><span class=\"title-block\"><strong>{title}</strong><small>{reviews} reviews</small></span><span class=\"score\">{score}</span></a>",
            slug = esc(&slug),
            title = esc(&game.title),
            reviews = game.review_count,
            score = fmt_score(game.agg_score.unwrap_or(0.0)),
        ));
    }
    body.push_str("</div>");
    let owned: Vec<GameView> = games.iter().map(|game| (*game).clone()).collect();
    (
        file.to_string(),
        document(
            Section::Other,
            title,
            &body,
            Some(path),
            &Seo {
                description: intro.replace("&amp;", "&"),
                json_ld: item_list_json(heading, &owned),
                image: Some(format!("{SITE_ORIGIN}/og-default.png")),
                extra_head: String::new(),
            },
        )
        .0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_titles() {
        assert_eq!(slugify("Hades II"), "hades-ii");
        assert_eq!(slugify("Ação"), "acao");
        assert_eq!(public_slug("bluey-s-happy-snaps", "Bluey's Happy Snaps"), "blueys-happy-snaps");
        assert_eq!(public_slug("baldurs-gate-3", "Baldur's Gate 3"), "baldurs-gate-3");
        assert_eq!(
            public_slug(
                "dragon-quest-xi-s-echoes-of-an-elusive-age",
                "Dragon Quest XI S: Echoes of an Elusive Age - Definitive Edition"
            ),
            "dragon-quest-xi-s-echoes-of-an-elusive-age"
        );
    }

    #[test]
    fn formats_without_trailing_zeros() {
        assert_eq!(fmt_num(80.0), "80");
        assert_eq!(fmt_num(75.5), "75.5");
    }

    #[test]
    fn strips_trailing_metacritic_tags() {
        assert_eq!(
            clean_synopsis("A great adventure. [GTA VI]"),
            "A great adventure."
        );
        assert_eq!(
            clean_synopsis("Paint job. [Xbox Live Arcade] [Namco Bandai Games]"),
            "Paint job."
        );
        assert_eq!(clean_synopsis("No tags here."), "No tags here.");
    }
}
