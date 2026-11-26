//! HTML pages. Each URL returns a finished document.

use std::collections::{BTreeMap, BTreeSet};

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
  flex-direction: column;
  align-items: stretch;
  gap: 14px;
  padding-bottom: 16px;
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

.site-nav {
  align-items: center;
  column-gap: 18px;
  row-gap: 8px;
}

@media (min-width: 900px) {
  .nav-group + .nav-group {
    margin-left: 6px;
    padding-left: 22px;
    border-left: 1px solid var(--line);
  }
}

.nav-group {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 6px 16px;
}

.nav-drop { position: relative; }

.nav-drop-toggle::after {
  content: "";
  display: inline-block;
  margin-left: 6px;
  border: 4px solid transparent;
  border-top-color: currentColor;
  border-bottom: 0;
  vertical-align: 2px;
}

.nav-drop::before {
  content: "";
  position: absolute;
  z-index: 29;
  left: 0;
  right: 0;
  top: 100%;
  height: 12px;
}

.nav-menu {
  display: none;
  position: absolute;
  z-index: 30;
  top: calc(100% + 8px);
  left: 0;
  width: max-content;
  max-width: min(560px, calc(100vw - 40px));
  max-height: min(70vh, 540px);
  overflow: auto;
  padding: 6px;
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 14px;
  box-shadow: 0 16px 36px rgba(26, 18, 24, 0.12);
}

.nav-drop:hover .nav-menu,
.nav-drop:focus-within .nav-menu,
.nav-drop.open .nav-menu {
  display: block;
}

.nav-best {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 2px 8px;
  padding: 7px 8px;
  border-radius: 10px;
}

.nav-best + .nav-best { border-top: 1px solid var(--line); }

header .site-nav .nav-platform {
  flex: 0 0 11rem;
  padding: 2px 0;
  color: var(--ink);
  font-weight: 600;
  box-shadow: none;
}

header .site-nav .nav-years {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  flex: 1 1 180px;
}

header .site-nav .nav-menu a {
  padding: 2px 7px;
  border-radius: 999px;
  color: var(--muted);
  font-weight: 500;
  box-shadow: none;
}

header .site-nav .nav-menu a:hover {
  color: var(--ink);
  background: var(--surface-hover);
}

header .site-nav .nav-menu a[aria-current="page"] {
  color: var(--accent);
  background: var(--accent-soft);
  box-shadow: none;
}

@media (max-width: 900px) {
  .nav-menu { left: auto; right: 0; }
  header .site-nav .nav-platform { flex-basis: 100%; }
}

header .site-nav a {
  padding: 0 0 2px;
  border-radius: 0;
  background: none;
  color: var(--muted);
  font-size: 0.95rem;
  font-weight: 500;
  line-height: 1.3;
}

header .site-nav a:hover {
  color: var(--ink);
  background: none;
}

header .site-nav a[aria-current="page"] {
  color: var(--accent);
  background: none;
  font-weight: 600;
  box-shadow: inset 0 -2px 0 var(--accent);
}

.page {
  margin: 28px 0 10px;
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.85rem;
  font-weight: 600;
  letter-spacing: -0.03em;
  line-height: 1.15;
}

.crumbs {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0;
  margin: 18px 0 0;
  padding: 0;
  list-style: none;
  color: var(--muted);
  font-size: 0.88rem;
}

.crumbs li { display: inline; }

.crumbs a {
  padding: 0;
  border-radius: 0;
  background: none;
  color: var(--muted);
  font-size: 0.88rem;
}

.crumbs a:hover { color: var(--accent); background: none; }

.crumbs [aria-current="page"] { color: var(--ink); }

.crumbs .sep { margin: 0 0.4em; color: var(--muted); }

.related { margin-top: 28px; }

.related h2 {
  margin: 0 0 10px;
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.15rem;
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
.row > span:first-child { min-width: 0; flex: 1 1 auto; }
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
.review, .unscored {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px 16px;
  padding: 14px 0;
  border-bottom: 1px solid var(--line);
}
/* `display: grid` above would otherwise override the hidden attribute. */
.review[hidden],
.unscored[hidden] {
  display: none !important;
}
.review:first-of-type, .unscored:first-of-type { border-top: 1px solid var(--line); }
.review strong, .unscored strong { display: block; font-size: 1.02rem; }
.review small, .unscored small {
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
.review .excerpt, .unscored .excerpt {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--text-body);
  font-size: 0.95rem;
  line-height: 1.5;
  max-width: 42rem;
}
.review .source, .unscored .source {
  grid-column: 1 / -1;
  font-size: 0.88rem;
}
.review strong a, .unscored strong a { color: inherit; text-decoration: none; }
.review strong a:hover, .unscored strong a:hover { color: var(--accent); text-decoration: underline; }
.review .source a, .unscored .source a { color: var(--accent); }
.review .source.unavailable, .unscored .source.unavailable { color: var(--muted); }

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

/* Decision: only the sorted figure is the headline. Sort by score shows the
   SourceCritic score; the simple mean stays in the subtitle. `inline-flex`
   would otherwise paint the hidden figure beside it. */
.score-compact[hidden],
[data-score-display][hidden],
[data-mean-display][hidden],
[data-z-display][hidden],
[data-score-meta][hidden],
[data-z-meta][hidden] {
  display: none !important;
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
.score-compact:focus-visible .frac,
.score-hero:hover .frac,
.score-hero:focus-within .frac,
.score-compact.score-open .frac {
  max-width: 5ch;
  opacity: 1;
}

/* Decision: the hero and the list subtitle use the same hover fraction as the
   headline number, without the wide list-score slot. */
.score-hero-value.score-compact,
.score-meta-num.score-compact {
  min-width: 0;
}

/* Decision: discovery lists show the score and the simple mean together, both
   as headline figures, so the gap is visible without opening the game.
   Red means the score is above the mean; purple means it is below. */
.score-pair {
  display: flex;
  gap: 16px;
  align-items: flex-end;
  flex: none;
}
.score-pair-item {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  min-width: 3.2rem;
}
.score-pair-item .score-compact {
  font-family: "IBM Plex Serif", Georgia, serif;
  font-size: 1.8rem;
  font-weight: 600;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
}
.score-pair .cmp-low .score-compact { color: var(--cmp-low); }
.score-pair .cmp-high .score-compact { color: var(--cmp-high); }
.score-pair-item small {
  margin: 0;
  color: var(--muted);
  font-size: 0.68rem;
  font-weight: 650;
  letter-spacing: 0.06em;
  text-transform: uppercase;
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

.outlet-search {
  margin: 4px 0 12px;
}

.outlet-search label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--muted);
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.outlet-search input[type="search"] {
  font: inherit;
  text-transform: none;
  letter-spacing: 0;
  color: var(--ink);
  width: 100%;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: white;
  padding: 10px 12px;
}

.outlet-search input[type="search"]::placeholder {
  color: color-mix(in srgb, var(--muted) 75%, white);
}

.outlet-search input[type="search"]:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--accent) 45%, white);
  outline-offset: 2px;
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

nav.sort button[data-sort][aria-current="true"][data-dir="asc"]::after { content: " ↑"; }
nav.sort button[data-sort][aria-current="true"][data-dir="desc"]::after { content: " ↓"; }

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
  color: #3a3340;
  font-size: 0.75rem;
  font-weight: 650;
  letter-spacing: 0.08em;
  text-transform: uppercase;
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

.provisional, .early {
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
  margin: 0 0 14px;
  padding: 0;
  border: 0;
  background: transparent;
}
.buy-block h2 {
  margin: 0 0 6px;
  font-family: "IBM Plex Sans", sans-serif;
  font-size: 0.78rem;
  font-weight: 650;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--muted);
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
  .nav-group { column-gap: 14px; }
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
  .row strong { white-space: normal; overflow-wrap: anywhere; }
  .score { font-size: 1.4rem; margin-left: 6px; padding-left: 0; }
  .score-pair { gap: 10px; }
  .score-pair-item .score-compact { font-size: 1.35rem; }
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
#[allow(dead_code)]
const ADSENSE_CLIENT: Option<&str> = Some("ca-pub-5490722539415533");
#[allow(dead_code)]
const GA_MEASUREMENT_ID: Option<&str> = None;
const PAGE_SIZE: usize = 50;
const DEFAULT_MIN_REVIEWS: i64 = 0;
/// The explicit 10+ filter still keeps a release from the last 30 days once it has 3 reviews.
const RECENT_LISTING_MIN: i64 = 10;
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
  function defaultDir(key) {
    return key === "title" || key === "name" ? "asc" : "desc";
  }
  function setCurrent(nav, key, dir) {
    if (!nav) return;
    nav.querySelectorAll("[data-sort]").forEach(function (btn) {
      if (btn.getAttribute("data-sort") === key) {
        btn.setAttribute("aria-current", "true");
        btn.setAttribute("data-dir", dir || defaultDir(key));
      } else {
        btn.removeAttribute("aria-current");
        btn.removeAttribute("data-dir");
      }
    });
  }
  function compareRows(a, b, mode, dir) {
    var sign = dir === "asc" ? 1 : -1;
    var cmp = 0;
    if (mode === "title" || mode === "name") cmp = text(a, "data-title").localeCompare(text(b, "data-title"));
    else if (mode === "reviews") cmp = num(a, "data-reviews") - num(b, "data-reviews");
    else if (mode === "mean") cmp = num(a, "data-mean") - num(b, "data-mean");
    else if (mode === "mu") cmp = num(a, "data-mu") - num(b, "data-mu");
    else if (mode === "mu_prime") cmp = num(a, "data-mu-prime") - num(b, "data-mu-prime");
    else if (mode === "z") cmp = num(a, "data-z") - num(b, "data-z");
    else if (mode === "score") cmp = num(a, "data-score") - num(b, "data-score");
    if (cmp === 0 && mode !== "title" && mode !== "name") {
      return text(a, "data-title").localeCompare(text(b, "data-title"));
    }
    return sign * cmp;
  }
  function sortChildren(list, key, mode, dir) {
    dir = dir || defaultDir(key);
    var rows = Array.prototype.slice.call(list.children);
    rows.sort(function (a, b) { return compareRows(a, b, mode, dir); });
    rows.forEach(function (row) { list.appendChild(row); });
    setCurrent(list.parentElement.querySelector(".sort"), key, dir);
    if (list.id === "game-list") {
      list.querySelectorAll(".row").forEach(function (row) {
        var scoreEl = row.querySelector("[data-score-display]");
        var meanEl = row.querySelector("[data-mean-display]");
        var meta = row.querySelector("[data-score-meta]");
        if (!scoreEl || !meanEl || !meta) return;
        // Decision: sort by score highlights the SourceCritic score and puts the
        // simple mean in the subtitle. Sort by simple mean does the opposite.
        // Red still means score > mean, purple means score < mean.
        var showMean = key === "mean";
        scoreEl.hidden = showMean;
        meanEl.hidden = !showMean;
        var score = num(row, "data-score");
        var mean = num(row, "data-mean");
        meta.innerHTML = showMean
          ? ("score " + colorFigure(score, mean, score, "score-compact score-meta-num"))
          : ("simple mean " + colorFigure(score, mean, mean, "score-compact score-meta-num"));
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
  // Decision: the number at rest is the integer. Hover and keyboard focus
  // reveal two decimal places on the headline, the subtitle, and the game page.
  function scoreParts(value) {
    var n = Number(value);
    if (!isFinite(n)) n = 0;
    var text = (Math.round(n * 100) / 100).toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
    var dot = text.indexOf(".");
    if (dot < 0) return { text: text, html: '<span class="whole">' + text + "</span>" };
    return {
      text: text,
      html: '<span class="whole">' + text.slice(0, dot) + '</span><span class="frac">' + text.slice(dot) + "</span>"
    };
  }
  function figure(value, className) {
    var parts = scoreParts(value);
    return '<span class="' + className + '" title="' + parts.text + '">' + parts.html + "</span>";
  }
  function colorFigure(score, mean, value, className) {
    var html = figure(value, className);
    if (!isFinite(score) || !isFinite(mean) || score === mean) return html;
    var cls = score > mean ? "cmp-low" : "cmp-high";
    return '<span class="' + cls + '">' + html + "</span>";
  }
  function fold(value) {
    return (value || "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  }
  function formState(form) {
    return {
      platform: (form.platform && form.platform.value) || "all",
      year: (form.year && form.year.value) || "all",
      min: String((form.min_reviews && form.min_reviews.value) || "0"),
      q: ((form.q && form.q.value) || "").trim(),
      sort: (form.getAttribute("data-sort") || "score"),
      dir: (form.getAttribute("data-dir") || defaultDir(form.getAttribute("data-sort") || "score"))
    };
  }
  function stateIsDefault(state) {
    return state.platform === "all" && state.year === "all" && state.min === "0" && !state.q && state.sort === "score" && state.dir !== "asc";
  }
  function filtersActive(form) {
    return !stateIsDefault(formState(form));
  }
  function currentPage() {
    var params = new URLSearchParams(location.search);
    var queryPage = Number(params.get("page"));
    if (queryPage >= 1) return queryPage;
    var match = (location.pathname || "").match(/\/page\/(\d+)\/?$/);
    return match ? Number(match[1]) : 1;
  }
  function browseUrl(state, page) {
    var params = new URLSearchParams();
    if (state.platform !== "all") params.set("platform", state.platform);
    if (state.year !== "all") params.set("year", state.year);
    if (state.min !== "0") params.set("min", state.min);
    if (state.sort && state.sort !== "score") params.set("sort", state.sort);
    if (state.dir && state.dir !== defaultDir(state.sort || "score")) params.set("dir", state.dir);
    if (state.q) params.set("q", state.q);
    var path = "/";
    if (stateIsDefault(state)) {
      if (page > 1) path = "/page/" + page + "/";
    } else if (page > 1) {
      params.set("page", String(page));
    }
    var qs = params.toString();
    return path + (qs ? "?" + qs : "");
  }
  function writeQuery(state, page) {
    history.replaceState(null, "", browseUrl(state, page || 1));
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
    var dir = params.get("dir");
    form.setAttribute("data-dir", dir === "asc" || dir === "desc" ? dir : defaultDir(form.getAttribute("data-sort") || "score"));
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
    function isRecent(game) {
      if (!game.released) return false;
      var then = Date.parse(game.released + "T00:00:00Z");
      if (!isFinite(then)) return false;
      var days = (Date.now() - then) / 86400000;
      return days >= 0 && days <= 30;
    }
    function passesMin(game, minReviews) {
      var n = Number(game.reviews || 0);
      if (!(minReviews > 0) || n >= minReviews) return true;
      return String(minReviews) === "10" && isRecent(game) && n >= 3;
    }
    var minReviews = Number(state.min || 0);
    var matched = games.filter(function (game) {
      if (state.platform !== "all" && (game.platforms || []).indexOf(state.platform) === -1) return false;
      if (state.year !== "all" && String(game.year || "") !== state.year) return false;
      if (!passesMin(game, minReviews)) return false;
      if (q && fold(game.title).indexOf(q) === -1) return false;
      return true;
    });
    var sort = state.sort || "score";
    var dir = state.dir || defaultDir(sort);
    var sign = dir === "asc" ? 1 : -1;
    matched.sort(function (a, b) {
      var cmp = 0;
      if (sort === "title") cmp = fold(a.title).localeCompare(fold(b.title));
      else if (sort === "reviews") cmp = Number(a.reviews) - Number(b.reviews);
      else if (sort === "mean") cmp = Number(a.mean) - Number(b.mean);
      else cmp = Number(a.score) - Number(b.score);
      if (cmp === 0) return fold(a.title).localeCompare(fold(b.title));
      return sign * cmp;
    });
    var pageSize = 50;
    var page = Math.max(1, currentPage());
    var pages = Math.max(1, Math.ceil(matched.length / pageSize));
    if (page > pages) page = pages;
    var slice = matched.slice((page - 1) * pageSize, page * pageSize);
    var html = "";
    slice.forEach(function (game, index) {
      var score = Number(game.score) || 0;
      var mean = Number(game.mean) || 0;
      var showMean = sort === "mean";
      var nReviews = Number(game.reviews) || 0;
      var provisional = isRecent(game) && nReviews < 10
        ? " <span class=\"early\">Early score - based on " + nReviews + " reviews</span>"
        : (nReviews < 5 ? " <span class=\"provisional\">provisional score</span>" : "");
      var bits = "";
      if (game.year) bits += game.year;
      if (game.platform_labels) bits += (bits ? " · " : "") + game.platform_labels;
      var headline = figure(showMean ? mean : score, "score score-compact") + provisional;
      var subtitle = showMean
        ? ("score " + colorFigure(score, mean, score, "score-compact score-meta-num"))
        : ("simple mean " + colorFigure(score, mean, mean, "score-compact score-meta-num"));
      var title = String(game.title || "").replace(/[&<>"]/g, function (ch) {
        return ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch];
      });
      html += '<a class="row" href="/game/' + game.slug + '/" data-score="' + score + '" data-mean="' + mean + '"><span class="meta">' +
        (game.cover ? '<img class="cover" src="' + game.cover + '" alt="Cover of ' + title + '" width="112" height="157" loading="lazy">' : "") +
        '<span class="title-block"><strong>' + title + '</strong><small><span class="row-secondary" data-score-meta>' +
        subtitle + '</span><span class="row-dot row-secondary"> · </span><span class="row-count">' +
        game.reviews + ' reviews</span>' + (bits ? '<span class="row-secondary"> · ' + bits + '</span>' : '') +
        '</small></span></span>' +
        (showMean
          ? '<span data-score-display hidden></span><span data-mean-display>' + headline + '</span>'
          : '<span data-score-display>' + headline + '</span><span data-mean-display hidden></span>') +
        '</a>';
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
    if (pager) {
      if (pages < 2) {
        pager.hidden = true;
      } else {
        pager.hidden = false;
        var prev = page > 1
          ? '<a href="' + browseUrl(state, page - 1).replace(/&/g, "&amp;") + '" data-page="' + (page - 1) + '" rel="prev">Previous</a>'
          : "";
        var next = page < pages
          ? '<a href="' + browseUrl(state, page + 1).replace(/&/g, "&amp;") + '" data-page="' + (page + 1) + '" rel="next">Next</a>'
          : "";
        pager.innerHTML = prev + "<span>Page " + page + " of " + pages + "</span>" + next;
      }
    }
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
  function syncBrowse(page) {
    var form = document.getElementById("game-filters");
    if (!form) return;
    var state = formState(form);
    var nextPage = typeof page === "number" && page >= 1 ? page : 1;
    if (stateIsDefault(state)) {
      var list = document.getElementById("game-list");
      if (list && list.getAttribute("data-client") === "1") {
        location.href = "/";
        return;
      }
      var reset = document.getElementById("filter-reset");
      if (reset) reset.hidden = true;
      writeQuery(state, 1);
      return;
    }
    writeQuery(state, nextPage);
    ensureIndex().then(renderClient);
  }
  function resetGameFilters() {
    var form = document.getElementById("game-filters");
    if (!form) return;
    if (form.platform) form.platform.value = "all";
    if (form.year) form.year.value = "all";
    if (form.min_reviews) form.min_reviews.value = "0";
    if (form.q) form.q.value = "";
    form.setAttribute("data-sort", "score");
    form.setAttribute("data-dir", "desc");
    var nav = document.querySelector(".browse-toolbar .sort");
    setCurrent(nav, "score", "desc");
    syncBrowse();
  }
  function bindOutletSearch() {
    var form = document.getElementById("outlet-search");
    var list = document.getElementById("outlet-list");
    if (!form || !list) return;
    var input = form.querySelector("input[type='search']");
    var empty = document.getElementById("outlet-empty");
    var meta = document.getElementById("outlet-search-meta");
    function apply() {
      var q = fold((input ? input.value : "").trim());
      var shown = 0;
      var total = 0;
      Array.prototype.forEach.call(list.children, function (row) {
        if (!row.classList || !row.classList.contains("row")) return;
        total += 1;
        var name = fold(row.getAttribute("data-title") || "");
        var match = !q || name.indexOf(q) !== -1;
        row.hidden = !match;
        if (match) shown += 1;
      });
      if (empty) empty.hidden = shown !== 0;
      if (meta) {
        if (!q) {
          meta.hidden = true;
          meta.textContent = "";
        } else {
          meta.hidden = false;
          meta.textContent = "Showing " + shown + " of " + total + (total === 1 ? " outlet." : " outlets.");
        }
      }
    }
    if (input) input.addEventListener("input", apply);
    form.addEventListener("submit", function (ev) {
      ev.preventDefault();
      apply();
    });
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
      var key = btn.getAttribute("data-sort") || defaultKey;
      var dir = defaultDir(key);
      if (btn.getAttribute("aria-current") === "true") {
        dir = btn.getAttribute("data-dir") === "asc" ? "desc" : "asc";
      }
      if (listId === "game-list") {
        var form = document.getElementById("game-filters");
        if (form) {
          form.setAttribute("data-sort", key);
          form.setAttribute("data-dir", dir);
        }
        setCurrent(nav, key, dir);
        syncBrowse();
        return;
      }
      sortChildren(list, key, key, dir);
    });
    if (listId !== "game-list") {
      var current = nav.querySelector("[aria-current='true']");
      var key = current ? current.getAttribute("data-sort") : defaultKey;
      var dir = current && current.getAttribute("data-dir") || defaultDir(key);
      sortChildren(list, key, key, dir);
    }
  }
  function bindBestMenu() {
    var drop = document.querySelector(".nav-drop");
    if (!drop) return;
    var toggle = drop.querySelector(".nav-drop-toggle");
    if (!toggle) return;
    var hover = window.matchMedia("(hover: hover) and (pointer: fine)").matches;
    if (hover) return;
    toggle.addEventListener("click", function (ev) {
      if (drop.classList.contains("open")) return;
      ev.preventDefault();
      drop.classList.add("open");
    });
    document.addEventListener("click", function (ev) {
      if (!drop.contains(ev.target)) drop.classList.remove("open");
    });
  }
  function bindPortFilter() {
    var list = document.getElementById("game-reviews");
    var bar = document.getElementById("port-filter");
    var meta = document.getElementById("port-filter-meta");
    if (!list || !bar) return;
    var hero = document.getElementById("score-hero-value");
    var heroMeta = document.getElementById("score-hero-meta");
    if (hero && !hero.getAttribute("data-base-html")) hero.setAttribute("data-base-html", hero.innerHTML);
    if (heroMeta && !heroMeta.getAttribute("data-base-html")) heroMeta.setAttribute("data-base-html", heroMeta.innerHTML);
    function apply(port) {
      var shown = 0;
      var rawSum = 0;
      var rawN = 0;
      var mappedSum = 0;
      var mappedN = 0;
      function consider(row) {
        var plats = (row.getAttribute("data-platforms") || "").split(/\s+/).filter(Boolean);
        var ok = port === "all" || plats.indexOf(port) !== -1;
        row.hidden = !ok;
        if (!ok || !row.classList.contains("review")) return;
        shown += 1;
        var raw = num(row, "data-raw-norm");
        var mapped = num(row, "data-mapped");
        if (isFinite(raw)) { rawSum += raw; rawN += 1; }
        if (isFinite(mapped)) { mappedSum += mapped; mappedN += 1; }
      }
      Array.prototype.forEach.call(list.querySelectorAll(".review"), consider);
      var notes = document.getElementById("unscored-reviews");
      if (notes) Array.prototype.forEach.call(notes.querySelectorAll(".unscored"), consider);
      Array.prototype.forEach.call(bar.querySelectorAll("[data-port]"), function (btn) {
        btn.setAttribute("aria-pressed", btn.getAttribute("data-port") === port ? "true" : "false");
      });
      var label = port === "all" ? "All platforms" : (bar.querySelector('[data-port="' + port + '"] .port-name') || {}).textContent || port;
      if (port === "all") {
        if (hero) hero.innerHTML = hero.getAttribute("data-base-html") || hero.innerHTML;
        if (heroMeta) heroMeta.innerHTML = heroMeta.getAttribute("data-base-html") || heroMeta.innerHTML;
      } else {
        var score = mappedN ? Math.round(mappedSum / mappedN) : 0;
        var mean = rawN ? Math.round(rawSum / rawN) : 0;
        if (hero) hero.innerHTML = '<span class="whole">' + score + "</span>";
        if (heroMeta) {
          heroMeta.textContent = label + " · simple mean " + mean + " · " + shown + (shown === 1 ? " review" : " reviews");
        }
      }
      if (meta) {
        meta.textContent = label + " · " + shown + (shown === 1 ? " review" : " reviews");
      }
    }
    bar.addEventListener("click", function (ev) {
      var btn = ev.target.closest("[data-port]");
      if (!btn || !bar.contains(btn)) return;
      apply(btn.getAttribute("data-port") || "all");
    });
    apply("all");
  }
  document.addEventListener("DOMContentLoaded", function () {
    var form = document.getElementById("game-filters");
    if (form) {
      readQuery(form);
      form.addEventListener("submit", function (ev) { ev.preventDefault(); syncBrowse(1); });
      form.addEventListener("change", function () { syncBrowse(1); });
      form.addEventListener("input", function () { syncBrowse(1); });
    }
    document.addEventListener("click", function (ev) {
      var link = ev.target.closest && ev.target.closest("[id=pager] a");
      if (!link) return;
      var filters = document.getElementById("game-filters");
      if (!filters || stateIsDefault(formState(filters))) return;
      ev.preventDefault();
      var explicit = Number(link.getAttribute("data-page"));
      var target = explicit >= 1 ? explicit : currentPage() + (link.getAttribute("rel") === "prev" ? -1 : 1);
      syncBrowse(Math.max(1, target));
    });
    var reset = document.getElementById("filter-reset");
    if (reset) reset.addEventListener("click", resetGameFilters);
    bindSort("game-list", "score");
    bindSort("outlet-list", "reviews");
    bindOutletSearch();
    bindSort("game-reviews", "name");
    bindSort("outlet-reviews", "name");
    bindPortFilter();
    bindBestMenu();
    if (form && !stateIsDefault(formState(form))) syncBrowse(currentPage());
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
  function gtagNpa() {
    window.dataLayer = window.dataLayer || [];
    function gtag(){ window.dataLayer.push(arguments); }
    window.gtag = window.gtag || gtag;
    gtag("consent", "update", {
      ad_storage: "granted",
      ad_user_data: "denied",
      ad_personalization: "denied",
      analytics_storage: "denied"
    });
  }
  function loadAds() {
    var mode = localStorage.getItem("sc_consent");
    if (mode !== "granted" && mode !== "npa") return;
    var nonPersonalized = mode === "npa";
    var units = document.querySelectorAll(".ad-unit");
    if (!units.length) return;
    var client = units[0].getAttribute("data-ad-client") || "";
    function fill() {
      try {
        window.adsbygoogle = window.adsbygoogle || [];
        if (nonPersonalized) window.adsbygoogle.requestNonPersonalizedAds = 1;
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
    if (saved === "npa") {
      if (banner) banner.hidden = true;
      gtagNpa();
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
      localStorage.setItem("sc_consent", "npa");
      banner.hidden = true;
      gtagNpa();
      loadAds();
    });
  }
})();
"#;

#[derive(Clone, Copy)]
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

fn publish_nav_from_state(state: &AppState) {
    let Ok(db) = lock(state) else {
        return;
    };
    let Ok(games) = db.list_games() else {
        return;
    };
    publish_best_nav(&games);
}

pub(super) async fn privacy(State(state): State<AppState>) -> impl IntoResponse {
    publish_nav_from_state(&state);
    ok_seo(
        Section::Other,
        "Privacy policy | SourceCritic",
        &legal_page(true),
        "/privacy/",
        "What SourceCritic collects, how AdSense and Analytics cookies work, and your LGPD rights.",
    )
}

pub(super) async fn terms(State(state): State<AppState>) -> impl IntoResponse {
    publish_nav_from_state(&state);
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

pub(super) async fn best_platform(
    State(state): State<AppState>,
    Path(platform): Path<String>,
) -> impl IntoResponse {
    discovery_response(state, &format!("best/{platform}/index.html")).await
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
    publish_best_nav(&games);
    match discovery_pages(&games, &outlets)
        .into_iter()
        .find(|(path, _)| path == file)
    {
        Some((_, html)) => {
            if let Some(target) = refresh_target(&html) {
                return (
                    StatusCode::MOVED_PERMANENTLY,
                    [(axum::http::header::LOCATION, target)],
                )
                    .into_response();
            }
            (StatusCode::OK, Html(html)).into_response()
        }
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
    publish_best_nav(&games);
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
        "SourceCritic: critic scores and reviews"
    } else {
        "SourceCritic: critic scores and reviews"
    };
    let description = format!(
        "Critic scores for {filtered_total} games, adjusted for each outlet's habit. Page {page} of {page_count}."
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
    let loaded = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()),
        };
        let mut games = match db.list_games() {
            Ok(games) => games,
            Err(error) => return internal(error.to_string()),
        };
        let found = match db.get_game(&slug) {
            Ok(game) => game.or_else(|| {
                games
                    .iter()
                    .find(|game| public_slug(&game.slug, &game.title) == slug)
                    .cloned()
            }),
            Err(error) => return internal(error.to_string()),
        };
        sort_games(&mut games, "score");
        (found, games)
    };
    let (found, games) = loaded;
    publish_best_nav(&games);
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
    let catalog = best_catalog(&games);
    let slugs = published_slugs(&games);
    let ctx = PageCtx {
        games: &games,
        catalog: &catalog,
        slugs: &slugs,
    };
    let public = published_slug(&game, &ctx);
    let path = format!("/game/{public}/");
    (
        StatusCode::OK,
        document(
            Section::Other,
            &format!("{}: critic score and reviews", game.title),
            &game_page(&game, sort, &ctx),
            Some(&path),
            &game_seo(&game, &public),
        ),
    )
}

pub(super) async fn method(State(state): State<AppState>) -> impl IntoResponse {
    publish_nav_from_state(&state);
    ok(Section::Score, "The score", &method_page())
}

pub(super) async fn about(State(state): State<AppState>) -> impl IntoResponse {
    publish_nav_from_state(&state);
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
        if let Ok(games) = db.list_games() {
            publish_best_nav(&games);
        }
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
    let loaded = {
        let db = match lock(&state) {
            Ok(db) => db,
            Err(_) => return internal("lock poisoned".to_string()),
        };
        let mut games = match db.list_games() {
            Ok(games) => games,
            Err(error) => return internal(error.to_string()),
        };
        publish_best_nav(&games);
        games.retain(|game| withhold_page(game).is_none());
        let published = published_slugs(&games);
        let found = match db.get_outlet(&slug) {
            Ok(outlet) => outlet,
            Err(error) => return internal(error.to_string()),
        };
        (found, published)
    };
    let (found, published) = loaded;
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
    ok(Section::Outlets, &outlet.name, &outlet_page(&outlet, sort, &published))
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

/// Platforms named in a review's own text or URL. A bare "PlayStation" does not
/// survive when the same text also names PlayStation 2, 3, 4, or 5.
pub fn mentioned_platforms(text: &str) -> Vec<String> {
    let hay: String = text
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();
    const PHRASES: &[(&str, &str)] = &[
        ("nintendo switch 2", "switch-2"),
        ("switch 2", "switch-2"),
        ("nintendo switch", "switch"),
        ("playstation 5", "ps5"),
        ("playstation 4", "ps4"),
        ("playstation 3", "ps3"),
        ("playstation 2", "ps2"),
        ("playstation vita", "vita"),
        ("ps vita", "vita"),
        ("playstation portable", "psp"),
        ("xbox series x s", "xbox-series"),
        ("xbox series x", "xbox-series"),
        ("xbox series s", "xbox-series"),
        ("xbox series", "xbox-series"),
        ("series x", "xbox-series"),
        ("series s", "xbox-series"),
        ("xbox one", "xbox-one"),
        ("xbox 360", "xbox-360"),
        ("nintendo 64", "n64"),
        ("nintendo 3ds", "3ds"),
        ("nintendo ds", "ds"),
        ("game boy advance", "gba"),
        ("game boy color", "gbc"),
        ("game boy", "gb"),
        ("game cube", "gamecube"),
        ("gamecube", "gamecube"),
        ("wii u", "wii-u"),
        ("dreamcast", "dreamcast"),
        ("meta quest", "meta-quest"),
        ("iphone", "ios"),
        ("ipad", "ios"),
        ("windows", "pc"),
        ("ps5", "ps5"),
        ("ps4", "ps4"),
        ("ps3", "ps3"),
        ("ps2", "ps2"),
        ("ps1", "ps1"),
        ("psvita", "vita"),
        ("switch", "switch"),
        ("playstation", "ps1"),
        ("xbox", "xbox"),
        ("wii", "wii"),
        ("3ds", "3ds"),
        ("n64", "n64"),
        ("psp", "psp"),
        ("vita", "vita"),
        ("pc", "pc"),
    ];
    let mut keys: Vec<String> = Vec::new();
    for (phrase, key) in PHRASES {
        if has_phrase(&hay, phrase) && !keys.iter().any(|existing| existing == key) {
            keys.push((*key).to_string());
        }
    }
    let specific_playstation = keys.iter().any(|key| {
        matches!(key.as_str(), "ps5" | "ps4" | "ps3" | "ps2" | "vita" | "psp")
    });
    if specific_playstation {
        keys.retain(|key| key != "ps1");
    }
    if keys.iter().any(|key| key == "switch-2") {
        keys.retain(|key| key != "switch");
    }
    if keys
        .iter()
        .any(|key| matches!(key.as_str(), "xbox-series" | "xbox-one" | "xbox-360"))
    {
        keys.retain(|key| key != "xbox");
    }
    if keys.iter().any(|key| key == "wii-u") {
        keys.retain(|key| key != "wii");
    }
    keys.sort();
    keys
}

fn has_phrase(hay: &str, needle: &str) -> bool {
    let mut start = 0;
    while let Some(rel) = hay[start..].find(needle) {
        let index = start + rel;
        let before_ok = hay[..index]
            .chars()
            .next_back()
            .map(|ch| !ch.is_ascii_alphanumeric())
            .unwrap_or(true);
        let after = index + needle.len();
        let after_ok = hay[after..]
            .chars()
            .next()
            .map(|ch| !ch.is_ascii_alphanumeric())
            .unwrap_or(true);
        if before_ok && after_ok {
            return true;
        }
        start = after;
        if start >= hay.len() {
            break;
        }
    }
    false
}

fn platform_key(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    match lower.as_str() {
        "playstation 5" | "ps5" | "playstation-5" | "playstation5" => "ps5".into(),
        "playstation 4" | "ps4" | "playstation-4" | "playstation4" => "ps4".into(),
        "playstation 3" | "ps3" | "playstation-3" | "playstation3" => "ps3".into(),
        "playstation 2" | "ps2" | "playstation-2" | "playstation2" => "ps2".into(),
        "playstation" | "ps1" | "psx" => "ps1".into(),
        "playstation vita" | "ps vita" | "psvita" | "vita" | "playstation-vita" => "vita".into(),
        "psp" | "playstation portable" | "playstation-portable" => "psp".into(),
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
                if part.eq_ignore_ascii_case("playstation") {
                    return "PlayStation".to_string();
                }
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

fn catalog_port_keys(game: &GameView) -> Vec<String> {
    let mut keys: Vec<String> = game
        .platforms
        .iter()
        .map(|platform| platform_key(platform))
        .filter(|key| !key.is_empty())
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

/// Ports that arrived later, when the original platform is also a release.
/// An untagged review belongs to the original releases, not to that later port.
fn later_ports(catalog: &[String]) -> Vec<&'static str> {
    let mut later = Vec::new();
    let has = |key: &str| catalog.iter().any(|item| item == key);
    if has("switch") && has("switch-2") {
        later.push("switch-2");
    }
    if has("wii") && has("wii-u") {
        later.push("wii-u");
    }
    later
}

fn named_port_keys(review: &ReviewView) -> Vec<String> {
    let mut keys: Vec<String> = review
        .platforms
        .iter()
        .map(|platform| platform_key(platform))
        .filter(|key| !key.is_empty())
        .collect();
    if keys.is_empty() {
        let mut text = review.excerpt.clone().unwrap_or_default();
        if let Some(url) = &review.review_url {
            text.push(' ');
            text.push_str(url);
        }
        keys = mentioned_platforms(&text);
    }
    keys.sort();
    keys.dedup();
    keys
}

/// Chips are the game's own releases. A review that names one of them stays on
/// those only. A review that names some other machine stays on All. A review
/// that names nothing stays on the original releases, not on a later port.
fn review_port_keys(review: &ReviewView, catalog: &[String]) -> Vec<String> {
    let named = named_port_keys(review);
    let mut matched: Vec<String> = named
        .iter()
        .filter(|key| catalog.iter().any(|item| item == *key))
        .cloned()
        .collect();
    matched.sort();
    matched.dedup();
    if !matched.is_empty() {
        return matched;
    }
    if !named.is_empty() {
        return Vec::new();
    }
    let later = later_ports(catalog);
    catalog
        .iter()
        .filter(|key| !later.iter().any(|port| port == key))
        .cloned()
        .collect()
}

fn port_score(
    tagged: &[(&ReviewView, Vec<String>)],
    mu_global: f64,
    port: Option<&str>,
) -> String {
    let mut sum = 0.0;
    let mut n = 0usize;
    for (review, keys) in tagged {
        if !counts_for_score(review) {
            continue;
        }
        if let Some(port) = port {
            if !keys.iter().any(|key| key == port) {
                continue;
            }
        }
        sum += crate::engine::map_z(review.z_score, mu_global);
        n += 1;
    }
    if n == 0 {
        "—".to_string()
    } else {
        format!("{}", (sum / n as f64).round() as i64)
    }
}

fn port_chip(key: &str, label: &str, score: &str, pressed: bool) -> String {
    format!(
        "<button type=\"button\" data-port=\"{key}\" aria-pressed=\"{pressed}\" aria-label=\"{label}, score {score}\">\
           <span class=\"port-name\">{label}</span>\
           <span class=\"port-score\">{score}</span>\
         </button>",
        key = esc(key),
        label = esc(label),
        score = esc(score),
        pressed = if pressed { "true" } else { "false" },
    )
}

#[allow(dead_code)]
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = (year - era * 400) as u64;
    let month_prime = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * month_prime as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

fn ordinal_date(value: &str) -> Option<i64> {
    let mut parts = value.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn today_ordinal() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86_400)
}

/// Released in the last 30 days. Games with no day-level date keep the 10-review rule.
fn is_recent_on(released_on: Option<&str>, today: i64) -> bool {
    let Some(released) = released_on.and_then(ordinal_date) else {
        return false;
    };
    let age = today - released;
    (0..=30).contains(&age)
}

fn is_recent(game: &GameView) -> bool {
    is_recent_on(game.released_on.as_deref(), today_ordinal())
}

fn passes_min_on(count: i64, released_on: Option<&str>, today: i64, min_reviews: i64) -> bool {
    if count >= min_reviews {
        return true;
    }
    // Choosing 10+ still keeps a release from the last 30 days once it has 3 reviews.
    min_reviews == RECENT_LISTING_MIN && is_recent_on(released_on, today) && count >= 3
}

fn passes_listing_min(game: &GameView, min_reviews: i64) -> bool {
    passes_min_on(
        game.review_count,
        game.released_on.as_deref(),
        today_ordinal(),
        min_reviews,
    )
}

/// Recent releases stay off the site until 3 critic reviews. Older games are unchanged.
pub(crate) fn withhold_page(game: &GameView) -> Option<String> {
    if is_recent(game) && game.review_count < 3 {
        return Some(format!(
            "{} critic reviews, below the 3-review minimum",
            game.review_count
        ));
    }
    None
}

fn early_badge(count: i64, recent: bool) -> String {
    if recent && count < 10 {
        format!(" <span class=\"early\">Early score - based on {count} reviews</span>")
    } else if count < 5 {
        " <span class=\"provisional\">provisional score</span>".to_string()
    } else {
        String::new()
    }
}

fn game_matches(game: &GameView, filters: &GameFilters) -> bool {
    if !passes_listing_min(game, filters.min_reviews) {
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
        let dir_attr = if current == *key {
            format!(" data-dir=\"{}\"", default_sort_dir(key))
        } else {
            String::new()
        };
        html.push_str(&format!(
            "<button type=\"button\" data-sort=\"{key}\"{current_attr}{dir_attr}>{label}</button>",
            key = esc(key),
            current_attr = current_attr,
            dir_attr = dir_attr,
            label = esc(label),
        ));
    }
    html.push_str("</nav>");
    html
}

fn default_sort_dir(key: &str) -> &'static str {
    match key {
        "title" | "name" => "asc",
        _ => "desc",
    }
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

fn aria_current(active: bool) -> &'static str {
    if active {
        " aria-current=\"page\""
    } else {
        ""
    }
}

#[derive(Clone)]
struct BestNavItem {
    key: String,
    label: String,
    years: Vec<i32>,
}

static BEST_NAV: std::sync::Mutex<Vec<BestNavItem>> = std::sync::Mutex::new(Vec::new());

fn best_nav_items(games: &[GameView]) -> Vec<BestNavItem> {
    let catalog = best_catalog(games);
    let mut items = Vec::new();
    for key in best_platform_keys() {
        if !catalog.hubs.contains(*key) {
            continue;
        }
        let mut years: Vec<i32> = catalog
            .years
            .iter()
            .filter(|(platform, _)| platform == key)
            .map(|(_, year)| *year)
            .collect();
        years.sort_unstable_by(|a, b| b.cmp(a));
        if years.is_empty() {
            continue;
        }
        items.push(BestNavItem {
            key: (*key).to_string(),
            label: platform_label(key),
            years,
        });
    }
    items
}

fn publish_best_nav(games: &[GameView]) {
    let items = best_nav_items(games);
    match BEST_NAV.lock() {
        Ok(mut slot) => *slot = items,
        Err(poisoned) => *poisoned.into_inner() = items,
    }
}

fn best_menu_markup(items: &[BestNavItem], path: &str) -> String {
    let on_best = path == "/best" || path.starts_with("/best/");
    if items.is_empty() {
        return format!(
            "<a href=\"/best/\"{best}>Best of</a>",
            best = aria_current(on_best)
        );
    }
    let mut html = format!(
        "<div class=\"nav-drop\"><a class=\"nav-drop-toggle\" href=\"/best/\" aria-haspopup=\"true\"{best}>Best of</a><div class=\"nav-menu\">",
        best = aria_current(on_best)
    );
    for item in items {
        let platform_path = format!("/best/{}/", item.key);
        let on_platform = path.trim_end_matches('/') == platform_path.trim_end_matches('/');
        html.push_str("<div class=\"nav-best\">");
        html.push_str(&format!(
            "<a class=\"nav-platform\" href=\"{platform_path}\"{current}>{label}</a>",
            current = aria_current(on_platform),
            label = esc(&item.label),
        ));
        if !item.years.is_empty() {
            html.push_str("<span class=\"nav-years\">");
            for year in &item.years {
                let year_path = format!("/best/{key}/{year}/", key = item.key);
                let on_year = path.trim_end_matches('/') == year_path.trim_end_matches('/');
                html.push_str(&format!(
                    "<a href=\"{year_path}\"{current}>{year}</a>",
                    current = aria_current(on_year),
                ));
            }
            html.push_str("</span>");
        }
        html.push_str("</div>");
    }
    html.push_str("</div></div>");
    html
}

fn site_header(section: Section, path: Option<&str>) -> String {
    let path = path.unwrap_or("");
    let on_ranking = path.starts_with("/outlets/ranking");
    let items = match BEST_NAV.lock() {
        Ok(slot) => slot.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    let best = best_menu_markup(&items, path);
    format!(
        "<header>\
           <a class=\"brand\" href=\"/\">SourceCritic</a>\
           <nav class=\"site-nav\" aria-label=\"Site\">\
             <div class=\"nav-group\">\
               <a href=\"/\"{games}>Games</a>\
               <a href=\"/outlets/\"{outlets}>Outlets</a>\
               <a href=\"/score/\"{score}>The score</a>\
               <a href=\"/about/\"{about}>About</a>\
             </div>\
             <div class=\"nav-group\">\
               {best}\
               <a href=\"/most-divisive/\"{divisive}>Most divisive</a>\
               <a href=\"/disagreements/\"{disagreements}>Disagreements</a>\
               <a href=\"/outlets/ranking/\"{habits}>Outlet habits</a>\
             </div>\
           </nav>\
         </header>",
        games = aria_current(matches!(section, Section::Games)),
        outlets = aria_current(matches!(section, Section::Outlets) && !on_ranking),
        score = aria_current(matches!(section, Section::Score)),
        about = aria_current(matches!(section, Section::About)),
        divisive = aria_current(path.starts_with("/most-divisive")),
        disagreements = aria_current(path.starts_with("/disagreements")),
        habits = aria_current(on_ranking),
    )
}

fn site_footer() -> String {
    "<footer class=\"site-footer\">\
       <nav aria-label=\"Footer\">\
         <a href=\"/privacy/\">Privacy</a>\
         <a href=\"/terms/\">Terms</a>\
         <a href=\"/about/\">About</a>\
       </nav>\
       <p>Game metadata and cover art come from IGDB, powered by Twitch. Reviews remain the property of each publication: SourceCritic shows the score, a short excerpt, and a link.</p>\
     </footer>"
        .to_string()
}

fn cookie_banner() -> String {
    String::from(
        "<div class=\"cookie-banner\" id=\"cookie-banner\">\
           <p>Ad and analytics cookies stay off until you accept. \
           Outside Europe, the UK, and Switzerland, rejecting still allows non-personalized ads. \
           <a href=\"/privacy/\">Privacy</a></p>\
           <div class=\"cookie-actions\">\
             <button type=\"button\" class=\"reject\" id=\"cookie-reject\">Reject</button>\
             <button type=\"button\" class=\"accept\" id=\"cookie-accept\">Accept</button>\
           </div>\
         </div>",
    )
}

fn site_config() -> serde_json::Value {
    serde_json::from_str(include_str!("../../data/config.json")).unwrap_or_else(|_| serde_json::json!({}))
}

fn gtm_container_id() -> Option<String> {
    let id = configured_id("gtm_id")?;
    let valid = id.starts_with("GTM-")
        && id.len() > 4
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if valid {
        Some(id)
    } else {
        None
    }
}

fn configured_id(key: &str) -> Option<String> {
    let value = site_config()
        .get(key)
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if value.is_empty() {
        return None;
    }
    if value.contains("YOUR_") {
        warn_placeholder(key);
        return None;
    }
    Some(value)
}

fn warn_placeholder(key: &str) {
    use std::sync::Mutex;
    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut seen = SEEN.lock().unwrap_or_else(|err| err.into_inner());
    if seen.iter().any(|item| item == key) {
        return;
    }
    eprintln!("warning: {key} is still a placeholder, so that integration is hidden");
    seen.push(key.to_string());
}

fn newsletter_form() -> String {
    let Some(username) = configured_id("buttondown_username") else {
        return String::new();
    };
    let endpoint = format!(
        "https://buttondown.com/api/emails/embed-subscribe/{username}"
    );
    format!(
        "<h3>This week's releases and scores</h3>\
         <p class=\"muted\">The form opens the newsletter provider. SourceCritic does not store the address.</p>\
         <form class=\"newsletter\" action=\"{endpoint}\" method=\"post\" target=\"_blank\">\
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
        // Decision: sort by score highlights the SourceCritic score and puts the
        // simple mean in the subtitle. Sort by simple mean does the opposite.
        // Red still means score > mean; purple means score < mean.
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
        let meta_html = if sort == "mean" {
            format!(
                "score {}",
                colored_figure(score_value, score_value, mean_value)
            )
        } else {
            format!(
                "simple mean {}",
                colored_figure(mean_value, score_value, mean_value)
            )
        };
        let provisional = early_badge(game.review_count, is_recent(game));
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
                 <small><span class=\"row-secondary\" data-score-meta>{meta_html}</span><span class=\"row-dot row-secondary\"> · </span><span class=\"row-count\">{count}</span>{secondary}</small></span></span>\
               {score_html}{mean_html}{provisional}\
             </a>",
            slug = esc(&public_slug(&game.slug, &game.title)),
            secondary = secondary,
            title = esc(&game.title),
            title_attr = esc(&game.title),
            count = esc(&count),
            meta_html = meta_html,
            provisional = provisional,
            score_html = score_figure(
                score_value,
                "score score-compact",
                if sort == "mean" { "data-score-display hidden" } else { "data-score-display" },
            ),
            mean_html = score_figure(
                mean_value,
                "score score-compact",
                if sort == "mean" { "data-mean-display" } else { "data-mean-display hidden" },
            ),
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
        "<form class=\"filters\" id=\"game-filters\" data-sort=\"{sort}\" data-dir=\"{dir}\" aria-label=\"Filters\" role=\"search\">",
        sort = esc(sort),
        dir = default_sort_dir(sort),
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
        let dir_attr = if sort == key {
            format!(" data-dir=\"{}\"", default_sort_dir(key))
        } else {
            String::new()
        };
        html.push_str(&format!(
            "<button type=\"button\" data-sort=\"{key}\"{current_attr}{dir_attr}>{label}</button>",
            key = esc(key),
            current_attr = current_attr,
            dir_attr = dir_attr,
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
    let crumbs = breadcrumb(&[
        Crumb::new("Home", "/", false),
        Crumb::new("Outlets", "/outlets/", true),
    ]);
    if outlets.is_empty() {
        return format!(
            "{crumbs}<h1 class=\"page\">Outlets</h1>\
             <p class=\"lead\">Each review site&apos;s scoring habit—mean, spread, and the reviews behind them.</p>\
             <p class=\"empty\">No outlets yet.</p>"
        );
    }
    let mut html = format!(
        "{crumbs}<h1 class=\"page\">Outlets</h1>\
         <p class=\"lead\">Each review site has a raw habit ($\\mu_j$, $\\sigma_j$) and a shrunk habit ($\\mu'_j$, $\\sigma'_j$) pulled toward the catalog. Open one to see every review that built those numbers.</p>\
         <form class=\"outlet-search\" id=\"outlet-search\" role=\"search\">\
           <label>Search\
             <input type=\"search\" name=\"q\" placeholder=\"Search outlets\" autocomplete=\"off\" spellcheck=\"false\">\
           </label>\
         </form>\
         <p class=\"filter-meta\" id=\"outlet-search-meta\" hidden></p>",
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
        let (count, score) = if outlet.n == 0 {
            (
                format!("{} reviews, none scored", outlet.reviews.len()),
                "—".to_string(),
            )
        } else {
            let count = if outlet.n == 1 {
                "1 review".to_string()
            } else {
                format!("{} reviews", outlet.n)
            };
            (
                format!(
                    "{count} · μ {mu} · σ {sigma} · μ′ {mu_p} · σ′ {sigma_p}",
                    mu = fmt_num(outlet.mu),
                    sigma = fmt_num(outlet.sigma),
                    mu_p = fmt_num(outlet.mu_prime),
                    sigma_p = fmt_num(outlet.sigma_prime),
                ),
                fmt_num(outlet.mu_prime),
            )
        };
        html.push_str(&format!(
            "<a class=\"row\" href=\"/outlet/{slug}/\" \
                data-title=\"{name_attr}\" data-reviews=\"{n}\" data-mu=\"{mu_n}\" data-mu-prime=\"{mu_p_n}\">\
               <span><strong>{name}</strong><small>{count}</small></span>\
               <span class=\"score\">{score}</span>\
             </a>",
            slug = esc(&outlet.slug),
            name = esc(&outlet.name),
            name_attr = esc(&outlet.name),
            count = count,
            n = outlet.n,
            score = score,
            mu_n = outlet.mu,
            mu_p_n = outlet.mu_prime,
        ));
    }
    html.push_str("</div>");
    html.push_str("<p class=\"empty\" id=\"outlet-empty\" hidden>No outlets match that search.</p>");
    html
}

fn linked_game_title(slug: &str, title: &str, published: &BTreeMap<String, String>) -> String {
    match published.get(slug) {
        Some(public) => format!(
            "<a href=\"/game/{slug}/\">{title}</a>",
            slug = esc(public),
            title = esc(title),
        ),
        None => esc(title).to_string(),
    }
}

fn outlet_page(outlet: &OutletView, sort: &str, published: &BTreeMap<String, String>) -> String {
    let unscored_outlet = outlet.slug == "kotaku" || outlet.n == 0;
    let count = if unscored_outlet {
        let notes = outlet.reviews.len();
        format!(
            "{notes} reviews, none scored. Listed for the excerpt and the link"
        )
    } else if outlet.n == 1 {
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
    for review in outlet.reviews.iter().filter(|review| {
        !unscored_outlet && review.raw_score.is_some_and(|score| score.is_finite())
    }) {
        let raw = presented_raw(
            &outlet.slug,
            review.raw_score,
            review.scale_max,
            review.raw_score_norm,
        );
        let mapped = crate::engine::map_z(review.z_score, outlet.mu_global);
        let excerpt = review
            .excerpt
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(clip_excerpt)
            .filter(|text| !text.is_empty())
            .map(|text| format!("<p class=\"excerpt\">{}</p>", esc(&text)))
            .unwrap_or_default();
        let source = original_source(review.review_url.as_deref());
        reviews.push_str(&format!(
            "<div class=\"review\" data-title=\"{game_attr}\" data-score=\"{score_n}\" data-z=\"{z_n}\">\
               <span><strong>{game}</strong>\
                 <small>{raw} → {norm} · \
                   <span data-z-meta>Z {z}</span>\
                   <span data-score-meta hidden>Score {mapped_sub}</span>\
                 </small>\
               </span>\
               <span data-score-display class=\"mapped\" title=\"This review&apos;s contribution to the game score\">Score {mapped_fig}</span>\
               <span data-z-display class=\"z\" hidden>Z {z}</span>\
               {excerpt}{source}\
             </div>",
            game = linked_game_title(&review.game_slug, &review.game_title, published),
            game_attr = esc(&review.game_title),
            raw = esc(&raw),
            norm = score_figure(review.raw_score_norm, "score-compact score-meta-num", ""),
            z = fmt_z(review.z_score),
            mapped_fig = score_figure(mapped, "score-compact", ""),
            mapped_sub = score_figure(mapped, "score-compact score-meta-num", ""),
            score_n = mapped,
            z_n = review.z_score,
            excerpt = excerpt,
            source = source,
        ));
    }
    reviews.push_str("</div></section>");
    if unscored_outlet {
        reviews.clear();
        let mut notes = String::new();
        for review in &outlet.reviews {
            notes.push_str(&unscored_card(
                &linked_game_title(&review.game_slug, &review.game_title, published),
                review.excerpt.as_deref(),
                review.review_url.as_deref(),
                "",
            ));
        }
        reviews.push_str(&unscored_section(&notes));
    } else {
        let mut notes = String::new();
        for review in outlet.reviews.iter().filter(|review| {
            !review.raw_score.is_some_and(|score| score.is_finite())
        }) {
            notes.push_str(&unscored_card(
                &linked_game_title(&review.game_slug, &review.game_title, published),
                review.excerpt.as_deref(),
                review.review_url.as_deref(),
                "",
            ));
        }
        reviews.push_str(&unscored_section(&notes));
    }
    let stats = if unscored_outlet {
        String::new()
    } else {
        format!(
            "<p class=\"lead\">μ<sub>j</sub> and σ<sub>j</sub> are this outlet&apos;s raw mean and spread. μ′<sub>j</sub> and σ′<sub>j</sub> are the shrunk values used when scoring a game—pulled toward the catalog so a thin history cannot dominate. See <a href=\"/score/\">how the score works</a>.</p>\
             <div class=\"stats\">\
               <div class=\"stat\"><span class=\"label\">Raw mean μ<sub>j</sub></span><span class=\"value\">{mu}</span></div>\
               <div class=\"stat\"><span class=\"label\">Raw spread σ<sub>j</sub></span><span class=\"value\">{sigma}</span></div>\
               <div class=\"stat\"><span class=\"label\">Shrunk mean μ′<sub>j</sub></span><span class=\"value\">{mu_p}</span></div>\
               <div class=\"stat\"><span class=\"label\">Shrunk spread σ′<sub>j</sub></span><span class=\"value\">{sigma_p}</span></div>\
             </div>",
            mu = fmt_num(outlet.mu),
            sigma = fmt_num(outlet.sigma),
            mu_p = fmt_num(outlet.mu_prime),
            sigma_p = fmt_num(outlet.sigma_prime),
        )
    };
    let baseline = if unscored_outlet {
        String::new()
    } else {
        format!(
            " Catalog baseline μ_global {mu_g}, σ_global {sigma_g}.",
            mu_g = fmt_num(outlet.mu_global),
            sigma_g = fmt_num(outlet.sigma_global),
        )
    };
    let crumbs = breadcrumb(&[
        Crumb::new("Home", "/", false),
        Crumb::new("Outlets", "/outlets/", false),
        Crumb::new(&outlet.name, &format!("/outlet/{}/", outlet.slug), true),
    ]);
    format!(
        "{crumbs}<h1 class=\"page\">{name}</h1>\
         <p class=\"muted\">{count}.{baseline}</p>\
         {stats}\
         {reviews}",
        crumbs = crumbs,
        name = esc(&outlet.name),
        count = count,
        baseline = baseline,
        stats = stats,
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
    let alt = format!("Cover of {title}");
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

struct Crumb {
    name: String,
    path: String,
    current: bool,
}

impl Crumb {
    fn new(name: &str, path: &str, current: bool) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            current,
        }
    }
}

struct BestCatalog {
    hubs: BTreeSet<String>,
    years: BTreeSet<(String, i32)>,
}

struct PageCtx<'a> {
    games: &'a [GameView],
    catalog: &'a BestCatalog,
    slugs: &'a BTreeMap<String, String>,
}

fn breadcrumb(crumbs: &[Crumb]) -> String {
    let mut html = String::from("<nav aria-label=\"Breadcrumb\"><ol class=\"crumbs\">");
    for crumb in crumbs {
        html.push_str("<li>");
        if !html.ends_with("<ol class=\"crumbs\"><li>") {
            html.push_str("<span class=\"sep\" aria-hidden=\"true\">/</span>");
        }
        if crumb.current {
            html.push_str(&format!(
                "<span aria-current=\"page\">{}</span>",
                esc(&crumb.name)
            ));
        } else {
            html.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                esc(&crumb.path),
                esc(&crumb.name)
            ));
        }
        html.push_str("</li>");
    }
    html.push_str("</ol></nav>");
    let mut items = Vec::new();
    for (index, crumb) in crumbs.iter().enumerate() {
        items.push(serde_json::json!({
            "@type": "ListItem",
            "position": index + 1,
            "name": crumb.name,
            "item": absolute_url(&crumb.path),
        }));
    }
    let value = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": items,
    });
    let raw = serde_json::to_string(&value).unwrap_or_else(|_| "{}".into());
    html.push_str(&format!(
        "<script type=\"application/ld+json\">{}</script>",
        raw.replace('<', "\\u003c")
    ));
    html
}

fn absolute_url(path: &str) -> String {
    if path == "/" {
        return format!("{SITE_ORIGIN}/");
    }
    let path = if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    };
    format!("{SITE_ORIGIN}{path}")
}

fn best_platform_keys() -> &'static [&'static str] {
    &[
        "ps5", "ps4", "ps3", "ps2", "ps1", "psp", "vita", "switch", "switch-2", "3ds", "ds",
        "wii", "wii-u", "gamecube", "n64", "pc", "xbox-series", "xbox-one", "xbox-360", "xbox",
        "ios",
    ]
}

/// A recent release stays out of /best/ until it has 10 reviews. Older games keep the 5-review lists.
fn best_eligible(game: &GameView) -> bool {
    if is_recent(game) {
        game.review_count >= 10
    } else {
        game.review_count >= 5
    }
}

fn scored_on<'a>(games: &'a [GameView], key: &str, year: i32) -> Vec<&'a GameView> {
    let mut picked: Vec<&GameView> = games
        .iter()
        .filter(|game| best_eligible(game) && released_on(game, key, year))
        .collect();
    picked.sort_by(|a, b| {
        b.agg_score
            .partial_cmp(&a.agg_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    picked
}

fn has_legacy_release(games: &[GameView], key: &str, year: i32) -> bool {
    games.iter().any(|game| {
        game.release_year == Some(year)
            && game
                .platforms
                .iter()
                .any(|platform| platform_key(platform) == key)
    })
}

fn best_catalog(games: &[GameView]) -> BestCatalog {
    let mut hubs = BTreeSet::new();
    let mut years = BTreeSet::new();
    let year_list = collect_years(games);
    let platform_keys = collect_platforms(games);
    for (key, _) in platform_keys
        .iter()
        .filter(|(key, _)| best_platform_keys().contains(&key.as_str()))
    {
        let mut linked = false;
        let mut legacy = false;
        for year in &year_list {
            if scored_on(games, key, *year).len() < 5 {
                if has_legacy_release(games, key, *year) {
                    legacy = true;
                }
                continue;
            }
            linked = true;
            years.insert((key.clone(), *year));
        }
        if linked || legacy {
            hubs.insert(key.clone());
        }
    }
    BestCatalog { hubs, years }
}

/// Same assignment `render_all_pages` uses, so a link never points at a slug redirect.
fn published_slugs(games: &[GameView]) -> BTreeMap<String, String> {
    let mut order: Vec<&GameView> = games.iter().collect();
    order.sort_by(|a, b| {
        b.agg_score
            .partial_cmp(&a.agg_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.cmp(&b.title))
    });
    let mut used = BTreeSet::new();
    let mut map = BTreeMap::new();
    for game in order {
        let mut slug = public_slug(&game.slug, &game.title);
        if !used.insert(slug.clone()) {
            slug = game.slug.clone();
        }
        map.insert(game.slug.clone(), slug);
    }
    map
}

fn published_slug(game: &GameView, ctx: &PageCtx<'_>) -> String {
    ctx.slugs
        .get(&game.slug)
        .cloned()
        .unwrap_or_else(|| public_slug(&game.slug, &game.title))
}

fn release_keys(game: &GameView) -> Vec<String> {
    let mut keys = catalog_port_keys(game);
    for (key, _) in &game.platform_years {
        if !keys.contains(key) {
            keys.push(key.clone());
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

/// Year `released_on` would accept for this platform.
fn platform_release_year(game: &GameView, platform: &str) -> Option<i32> {
    if !game.platform_years.is_empty() {
        return game
            .platform_years
            .iter()
            .find(|(key, _)| key == platform)
            .map(|(_, year)| *year);
    }
    let year = game.release_year?;
    if year >= platform_launch_year(platform)
        && game
            .platforms
            .iter()
            .any(|item| platform_key(item) == platform)
    {
        Some(year)
    } else {
        None
    }
}

/// Oldest platform that already has a `/best/{key}/` hub.
/// A year list beats a hub-only crumb. The platform key breaks the rest.
fn primary_release(game: &GameView, catalog: &BestCatalog) -> Option<(String, i32, bool)> {
    let mut candidates = Vec::new();
    for key in release_keys(game) {
        if !catalog.hubs.contains(&key) {
            continue;
        }
        let Some(year) = platform_release_year(game, &key) else {
            continue;
        };
        let year_page = catalog.years.contains(&(key.clone(), year));
        candidates.push((year_page, year, key));
    }
    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    let (year_page, year, key) = candidates.into_iter().next()?;
    Some((key, year, year_page))
}

fn game_crumbs(game: &GameView, ctx: &PageCtx<'_>) -> String {
    let slug = published_slug(game, ctx);
    let mut crumbs = vec![Crumb::new("Home", "/", false)];
    if let Some((key, year, year_page)) = primary_release(game, ctx.catalog) {
        let hub = format!("/best/{key}/");
        crumbs.push(Crumb::new(&platform_label(&key), &hub, false));
        let year_path = if year_page {
            format!("/best/{key}/{year}/")
        } else {
            hub
        };
        crumbs.push(Crumb::new(&year.to_string(), &year_path, false));
    }
    crumbs.push(Crumb::new(&game.title, &format!("/game/{slug}/"), true));
    breadcrumb(&crumbs)
}

fn more_from_html(game: &GameView, ctx: &PageCtx<'_>) -> String {
    let mut html = String::new();
    for key in release_keys(game) {
        let Some(year) = platform_release_year(game, &key) else {
            continue;
        };
        if !ctx.catalog.years.contains(&(key.clone(), year)) {
            continue;
        }
        let label = platform_label(&key);
        let path = format!("/best/{key}/{year}/");
        html.push_str(&format!(
            "<section class=\"related\"><h2>More from {year} on {label}</h2><div>\
               <a class=\"row\" href=\"{path}\"><span><strong>Best {label} games of {year}</strong></span></a>\
             </div></section>",
            label = esc(&label),
            path = esc(&path),
        ));
    }
    html
}

fn similar_games<'a>(game: &GameView, games: &'a [GameView]) -> Vec<&'a GameView> {
    let (Some(year), Some(score)) = (game.release_year, game.agg_score) else {
        return Vec::new();
    };
    if !score.is_finite() {
        return Vec::new();
    }
    let mut peers: Vec<&GameView> = games
        .iter()
        .filter(|peer| {
            peer.slug != game.slug
                && peer.release_year == Some(year)
                && peer.agg_score.is_some_and(|value| value.is_finite())
        })
        .collect();
    peers.sort_by(|a, b| {
        let left = (a.agg_score.unwrap_or(0.0) - score).abs();
        let right = (b.agg_score.unwrap_or(0.0) - score).abs();
        left.total_cmp(&right)
            .then_with(|| public_slug(&a.slug, &a.title).cmp(&public_slug(&b.slug, &b.title)))
            .then_with(|| a.slug.cmp(&b.slug))
    });
    peers.truncate(6);
    peers
}

fn similar_html(game: &GameView, ctx: &PageCtx<'_>) -> String {
    let peers = similar_games(game, ctx.games);
    if peers.is_empty() {
        return String::new();
    }
    let mut html = String::from("<section class=\"related\"><h2>Similar scores</h2><div>");
    for peer in peers {
        let slug = published_slug(peer, ctx);
        html.push_str(&format!(
            "<a class=\"row\" href=\"/game/{slug}/\"><span><strong>{title}</strong></span>{score}</a>",
            slug = esc(&slug),
            title = esc(&peer.title),
            score = score_figure(peer.agg_score.unwrap_or(0.0), "score score-compact", ""),
        ));
    }
    html.push_str("</div></section>");
    html
}

fn game_page(game: &GameView, sort: &str, ctx: &PageCtx<'_>) -> String {
    let scored = game.reviews.iter().filter(|review| counts_for_score(review)).count();
    let count = if scored == 1 {
        "1 review".to_string()
    } else {
        format!("{scored} reviews")
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
    let catalog_ports = catalog_port_keys(game);
    let tagged: Vec<(&ReviewView, Vec<String>)> = game
        .reviews
        .iter()
        .filter(|review| is_public_outlet(&review.outlet_slug))
        .map(|review| (review, review_port_keys(review, &catalog_ports)))
        .collect();
    let mut reviews = String::from("<section class=\"reviews\"><h2>Outlet reviews</h2>");
    if catalog_ports.len() > 1 {
        reviews.push_str(
            "<div class=\"port-filter\" id=\"port-filter\" role=\"group\" aria-label=\"Filter by platform\">",
        );
        let all_score = port_score(&tagged, game.mu_global, None);
        reviews.push_str(&port_chip("all", "All", &all_score, true));
        for key in &catalog_ports {
            let score = port_score(&tagged, game.mu_global, Some(key));
            reviews.push_str(&port_chip(key, &platform_label(key), &score, false));
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
    for (review, keys) in tagged.iter().filter(|(review, _)| counts_for_score(review)) {
        let raw = presented_raw(
            &review.outlet_slug,
            review.raw_score,
            review.scale_max,
            review.raw_score_norm,
        );
        let mapped = crate::engine::map_z(review.z_score, game.mu_global);
        let excerpt = review
            .excerpt
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(clip_excerpt)
            .filter(|text| !text.is_empty())
            .map(|text| format!("<p class=\"excerpt\">{}</p>", esc(&text)))
            .unwrap_or_default();
        let source = original_source(review.review_url.as_deref());
        let plats = keys.join(" ");
        reviews.push_str(&format!(
            "<div class=\"review\" data-title=\"{outlet_attr}\" data-score=\"{score_n}\" data-z=\"{z_n}\" \
               data-mapped=\"{mapped_n}\" data-raw-norm=\"{norm_n}\" data-platforms=\"{plats}\">\
               <span><strong><a href=\"/outlet/{outlet_slug}/\">{outlet}</a></strong>\
                 <small>{raw} → {norm} · \
                   <span data-z-meta>Z {z}</span>\
                   <span data-score-meta hidden>Score {mapped_sub}</span>\
                 </small>\
               </span>\
               <span data-score-display class=\"mapped\" title=\"This outlet&apos;s contribution to the game score\">Score {mapped_fig}</span>\
               <span data-z-display class=\"z\" hidden>Z {z}</span>\
               {excerpt}{source}\
             </div>",
            outlet_slug = esc(&review.outlet_slug),
            outlet = esc(&review.outlet_name),
            outlet_attr = esc(&review.outlet_name),
            raw = esc(&raw),
            norm = score_figure(review.raw_score_norm, "score-compact score-meta-num", ""),
            z = fmt_z(review.z_score),
            mapped_fig = score_figure(mapped, "score-compact", ""),
            mapped_sub = score_figure(mapped, "score-compact score-meta-num", ""),
            score_n = mapped,
            z_n = review.z_score,
            mapped_n = mapped,
            norm_n = review.raw_score_norm,
            plats = esc(&plats),
            excerpt = excerpt,
            source = source,
        ));
    }
    reviews.push_str("</div></section>");
    let mut notes = String::new();
    for (review, keys) in tagged.iter().filter(|(review, _)| !counts_for_score(review)) {
        notes.push_str(&unscored_card(
            &format!(
                "<a href=\"/outlet/{slug}/\">{name}</a>",
                slug = esc(&review.outlet_slug),
                name = esc(&review.outlet_name),
            ),
            review.excerpt.as_deref(),
            review.review_url.as_deref(),
            &keys.join(" "),
        ));
    }
    reviews.push_str(&unscored_section(&notes));
    let crumbs = game_crumbs(game, ctx);
    let more = more_from_html(game, ctx);
    let similar = similar_html(game, ctx);
    format!(
        "{crumbs}<div class=\"game-hero\">{cover}<div class=\"text\">\
           <h1 class=\"page\">{title}</h1>\
           <div class=\"score-hero\" aria-label=\"SourceCritic score {score}\">\
             {score_html}\
             <span class=\"score-hero-copy\">\
               <span class=\"score-hero-label\">SourceCritic score</span>\
               <span class=\"score-hero-meta\" id=\"score-hero-meta\" data-mean=\"{mean_n}\" data-count=\"{scored}\" data-provisional=\"{provisional_flag}\">{hero_meta}</span>\
             </span>\
           </div>\
           {buy}\
           {synopsis}\
           {meta}\
           <p class=\"lead\">The published score is adjusted by each outlet&apos;s history. See <a href=\"/score/\">how the score works</a>.</p>\
         </div></div>\
         {reviews}\
         {more}\
         {similar}\
         {ad}",
        crumbs = crumbs,
        title = esc(&game.title),
        score_html = score_figure(
            game.agg_score.unwrap_or(0.0),
            "score-hero-value score-compact",
            &format!(
                "id=\"score-hero-value\" data-score=\"{}\"",
                game.agg_score.unwrap_or(0.0)
            ),
        ),
        mean_n = game.simple_mean.unwrap_or(0.0),
        scored = scored,
        provisional_flag = if scored < 5 { "1" } else { "0" },
        hero_meta = format!(
            "Simple mean {mean} · {count}{provisional}",
            mean = score_figure(
                game.simple_mean.unwrap_or(0.0),
                "score-compact score-meta-num",
                ""
            ),
            count = count,
            provisional = early_badge(scored as i64, is_recent(game)),
        ),
        buy = buy_block(&game.title, &game.platforms),
        ad = ad_unit("ad-game rect", "280px"),
        score = fmt_score(game.agg_score.unwrap_or(0.0)),
        synopsis = synopsis,
        meta = meta,
        cover = cover,
        reviews = reviews,
        more = more,
        similar = similar,
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

fn is_hub_url(url: &str) -> bool {
    let Some(after_scheme) = url.split("://").nth(1) else {
        return true;
    };
    let cut = after_scheme.find(['?', '#']).unwrap_or(after_scheme.len());
    let path = &after_scheme[..cut];
    let query = if cut < after_scheme.len() { &after_scheme[cut + 1..] } else { "" };
    let mut parts: Vec<String> = path
        .split('/')
        .filter(|part| !part.is_empty())
        .skip(1)
        .map(|part| part.to_ascii_lowercase())
        .collect();
    const LOCALES: &[&str] = &["en", "uk", "au", "us", "br", "fr", "de", "es", "jp"];
    while parts.first().is_some_and(|part| LOCALES.contains(&part.as_str())) {
        parts.remove(0);
    }
    if parts.is_empty() {
        return query.is_empty();
    }
    const SECTIONS: &[&str] = &[
        "xbox",
        "playstation",
        "pc",
        "nintendo",
        "switch",
        "ps5",
        "ps4",
        "ps3",
        "reviews",
        "review",
        "gaming",
        "games",
        "video-games",
        "news",
        "404",
        "review-guidelines",
        "casino-sites",
        "actualite.html",
    ];
    let last = parts[parts.len() - 1].as_str();
    if last.contains("suspendedpage")
        || last == "404"
        || last == "review-guidelines"
        || last == "casino-sites"
        || last == "actualite.html"
    {
        return true;
    }
    parts.iter().all(|part| SECTIONS.contains(&part.as_str()))
}

fn verifiable_url(url: Option<&str>) -> bool {
    url.is_some_and(|url| {
        (url.starts_with("https://") || url.starts_with("http://"))
            && !url.contains("savethechildren.org")
            && !is_hub_url(url)
    })
}

fn original_source(url: Option<&str>) -> String {
    match url.filter(|url| verifiable_url(Some(url))) {
        Some(url) => format!(
            "<p class=\"source\"><a href=\"{url}\" rel=\"noopener noreferrer\" target=\"_blank\">Read original review</a></p>",
            url = esc(url),
        ),
        None => "<p class=\"source unavailable\">Original inaccessible</p>".to_string(),
    }
}

fn unscored_card(
    title_html: &str,
    excerpt: Option<&str>,
    url: Option<&str>,
    platforms: &str,
) -> String {
    let excerpt = excerpt
        .filter(|text| !text.is_empty())
        .map(clip_excerpt)
        .filter(|text| !text.is_empty())
        .map(|text| format!("<p class=\"excerpt\">{}</p>", esc(&text)))
        .unwrap_or_default();
    let (status, source) = if verifiable_url(url) {
        ("No score", original_source(url))
    } else {
        ("Original inaccessible", String::new())
    };
    format!(
        "<div class=\"unscored\" data-platforms=\"{platforms}\"><span><strong>{title_html}</strong><small>{status}</small></span>{excerpt}{source}</div>",
        platforms = esc(platforms),
    )
}

fn unscored_section(cards: &str) -> String {
    if cards.is_empty() {
        return String::new();
    }
    format!(
        "<section class=\"reviews\" id=\"unscored-reviews\"><h2>Reviews without a score</h2>\
         <p class=\"muted\">These stay out of the score because no numeric grade was published.</p>\
         {cards}</section>"
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
    let Some(client) = configured_id("adsense_client") else {
        return String::new();
    };
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
        client = esc(&client),
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
    let header = site_header(section, path);
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
    let ga = configured_id("ga4_id").unwrap_or_default();
    let (gtm_boot, gtm_head, gtm_body) = match gtm_container_id() {
        Some(id) => (
            "gtag('js', new Date());dataLayer.push({'gtm.start':new Date().getTime(),event:'gtm.js'});"
                .to_string(),
            format!(
                "<script async src=\"https://www.googletagmanager.com/gtm.js?id={id}\"></script>"
            ),
            format!(
                "<noscript><iframe src=\"https://www.googletagmanager.com/ns.html?id={id}\" height=\"0\" width=\"0\" style=\"display:none;visibility:hidden\"></iframe></noscript>"
            ),
        ),
        None => (String::new(), String::new(), String::new()),
    };
    Html(format!(
        "<!DOCTYPE html>\
         <html lang=\"en\">\
         <head>\
           <meta charset=\"utf-8\">\
           <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
           <title>{title}</title>\
           <meta name=\"description\" content=\"{description}\">\
           <link rel=\"canonical\" href=\"{canonical}\">\
           <link rel=\"icon\" href=\"/favicon.ico\" sizes=\"32x32\">\
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
             {gtm_boot}\
           </script>\
           {gtm_head}\
           <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\
           <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\
           <link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&family=IBM+Plex+Serif:wght@600&display=swap\" rel=\"stylesheet\">\
           {katex}\
           <style>{CSS}</style>\
           <script>{SITE_JS}</script>\
         </head>\
         <body data-ga=\"{ga}\">\
           {gtm_body}\
           <div class=\"page-shell\">\
             <main>\
               {header}\
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
        gtm_boot = gtm_boot,
        gtm_head = gtm_head,
        gtm_body = gtm_body,
        ga = esc(&ga),
        header = header,
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
    games.retain(|game| match withhold_page(game) {
        Some(reason) => {
            eprintln!("skipped: {} - {reason}", game.title);
            false
        }
        None => true,
    });
    publish_best_nav(&games);
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
            "Critic scores for {filtered_total} games, adjusted for each outlet's scoring habit. Page {page} of {page_count}."
        );
        pages.push((
            file,
            document(
                Section::Games,
                "SourceCritic: critic scores and reviews",
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

    let catalog = best_catalog(&games);
    let slugs = published_slugs(&games);
    let ctx = PageCtx {
        games: &games,
        catalog: &catalog,
        slugs: &slugs,
    };
    for game in &games {
        let mut detailed = db
            .get_game(&game.slug)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing game {}", game.slug))?;
        sort_game_reviews(&mut detailed.reviews, "name", detailed.mu_global);
        let slug = published_slug(&detailed, &ctx);
        let path = format!("/game/{slug}/");
        pages.push((
            format!("game/{slug}/index.html"),
            document(
                Section::Other,
                &format!("{}: critic score and reviews", detailed.title),
                &game_page(&detailed, "name", &ctx),
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
                &outlet_page(&detailed, "name", &slugs),
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
        if withhold_page(game).is_some() {
            continue;
        }
        items.push(serde_json::json!({
            "slug": public_slug(&game.slug, &game.title),
            "title": game.title,
            "score": game.agg_score.unwrap_or(0.0),
            "mean": game.simple_mean.unwrap_or(0.0),
            "reviews": game.review_count,
            "year": game.release_year,
            "released": game.released_on,
            "platforms": platforms,
            "platform_labels": labels,
            "cover": cover,
            "provisional": game.review_count < 5,
            "early": is_recent(game) && game.review_count < 10,
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

/// A review counts when the outlet published a number. Kotaku does not.
/// A missing link stays in the score; the page just omits the link.
fn counts_for_score(review: &crate::db::ReviewView) -> bool {
    is_public_outlet(&review.outlet_slug)
        && review.outlet_slug != "kotaku"
        && review.raw_score.is_some_and(|score| score.is_finite())
        && review
            .review_url
            .as_deref()
            .is_none_or(|url| !url.contains("savethechildren.org"))
}

fn scored_norms(game: &GameView) -> Vec<f64> {
    game.reviews
        .iter()
        .filter(|review| counts_for_score(review))
        .map(|review| review.raw_score_norm)
        .collect()
}

fn clip_excerpt(text: &str) -> String {
    let raw = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if raw.is_empty() {
        return String::new();
    }
    let mut kept: Vec<&str> = Vec::new();
    for sentence in raw.split_inclusive(['.', '!', '?']) {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }
        if kept.len() >= 2 {
            break;
        }
        let candidate = if kept.is_empty() {
            sentence.to_string()
        } else {
            format!("{} {}", kept.join(" "), sentence)
        };
        if candidate.chars().count() > 300 && !kept.is_empty() {
            break;
        }
        kept.push(sentence);
        if candidate.chars().count() >= 300 {
            break;
        }
    }
    let mut out = kept.join(" ");
    if out.chars().count() > 300 {
        let window: String = out.chars().take(300).collect();
        if let Some(end) = window.rfind(['.', '!', '?']) {
            out = window[..=end].trim().to_string();
        } else if let Some(end) = window.rfind(' ') {
            out = window[..end].trim().to_string();
        } else {
            out = window;
        }
    }
    if out != raw && !out.ends_with('…') {
        out = out.trim_end_matches(['.', '!', '?', ' ']).to_string();
        out.push('…');
    }
    out
}

pub(crate) fn refresh_target(html: &str) -> Option<String> {
    let marker = "http-equiv=\"refresh\" content=\"0; url=";
    let rest = html.split(marker).nth(1)?;
    let target = rest.split('"').next()?;
    if target.starts_with('/') {
        Some(target.replace("&amp;", "&"))
    } else {
        None
    }
}

fn redirect_page(target: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <link rel=\"canonical\" href=\"{origin}{target}\">\
         <meta name=\"robots\" content=\"noindex\">\
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
    let scored = game.reviews.iter().filter(|review| counts_for_score(review)).count();
    let description = format!(
        "{}{year} has a SourceCritic score of {score} from {scored} critic reviews.{platform_bit}",
        game.title
    );
    let image = Some(format!("{SITE_ORIGIN}/og/{slug}.jpg"));
    Seo {
        description,
        image,
        json_ld: game_json_ld(game, slug),
        extra_head: String::new(),
    }
}

fn game_json_ld(game: &GameView, slug: &str) -> String {
    let mut reviews = Vec::new();
    let scored: Vec<_> = game
        .reviews
        .iter()
        .filter(|review| counts_for_score(review))
        .collect();
    for review in &scored {
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
    if !scored.is_empty() {
        value["aggregateRating"] = serde_json::json!({
            "@type": "AggregateRating",
            "ratingValue": fmt_score(game.agg_score.unwrap_or(0.0)),
            "bestRating": "100",
            "worstRating": "0",
            "ratingCount": scored.len()
        });
    }
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".into())
}

fn fmt_score(value: f64) -> String {
    format!("{}", value.round() as i64)
}

/// Scale the outlet actually publishes on. Aggregator rows were stored as 0–100.
fn native_scale(slug: &str) -> Option<f64> {
    if slug == "ign" || slug.starts_with("ign-") {
        return Some(10.0);
    }
    if slug == "eurogamer" || slug.starts_with("eurogamer-") {
        return Some(5.0);
    }
    if slug == "gamespot" || slug.starts_with("gamespot-") {
        return Some(10.0);
    }
    Some(match slug {
        "gamesradar" | "vg247" | "guardian" | "giant-bomb" | "easy-allies" | "usgamer"
        | "telegraph" | "gamepro" | "australian-gamepro" => 5.0,
        "jeuxvideo-com" => 20.0,
        "weekly-famitsu" => 40.0,
        "edge-magazine" | "game-informer" | "egm" | "metro-gamecentral" | "thesixthaxis"
        | "worth-playing" | "cogconnected" | "gamingtrend" | "hardcore-gamer" | "shacknews"
        | "dualshockers" | "god-is-a-geek" | "gamingbolt" | "game-revolution"
        | "nintendoworldreport" | "playstation-lifestyle" | "videogamer" | "ztgd"
        | "gaming-nexus" | "noobfeed" | "gamespew" | "techraptor" | "monstervine"
        | "noisy-pixel" | "checkpoint-gaming" | "impulsegamer" | "digital-chumps"
        | "gaming-age" | "cubed3" | "playstation-universe" | "video-chums" | "atomix"
        | "areajugones" | "hobby-consolas" | "meristation" | "vandal" | "3djuegos"
        | "multiplayer-it" | "everyeye-it" | "spaziogames" | "gamer-nl" | "xgn"
        | "the-games-machine" | "gameblast" | "psx-brasil" | "levelup" | "pure-xbox"
        | "rpg-site" | "attack-of-the-fanboy" | "stevivor" | "vgc" | "polygon" | "destructoid"
        | "nintendo-life" | "push-square" | "gamekult" => 10.0,
        _ => return None,
    })
}

fn format_on_scale(value: f64, scale: f64) -> String {
    let number = if scale >= 100.0 {
        format!("{}", value.round() as i64)
    } else {
        let rounded = (value * 10.0).round() / 10.0;
        if (rounded - rounded.round()).abs() < 0.001 {
            format!("{}", rounded.round() as i64)
        } else {
            format!("{rounded:.1}")
        }
    };
    let ceiling = if (scale - scale.round()).abs() < 0.001 {
        format!("{}", scale.round() as i64)
    } else {
        format!("{scale}")
    };
    format!("{number}/{ceiling}")
}

/// `9/10` or `4/5` when the outlet's scale is known. A row already stored on
/// that scale is left as written. A flattened aggregator `90/100` is converted.
fn presented_raw(slug: &str, raw: Option<f64>, scale: Option<f64>, norm: f64) -> String {
    if let (Some(raw), Some(scale)) = (raw, scale) {
        let flattened = scale >= 99.5 && (raw - norm).abs() < 0.051;
        if !flattened {
            return format_on_scale(raw, scale);
        }
    }
    if let Some(scale) = native_scale(slug) {
        if scale < 99.5 {
            return format_on_scale(norm * scale / 100.0, scale);
        }
    }
    format_on_scale(raw.unwrap_or(norm), scale.unwrap_or(100.0))
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
    let rounded = (value * 10.0).round() / 10.0;
    format!("{rounded:.1}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Decision: the resting figure is the rounded integer. Hover and keyboard
/// focus reveal two decimal places. Used for the list headline, the list
/// subtitle, the game-page score and simple mean, and each review's score.
fn score_parts(value: f64) -> (String, String, String) {
    let rounded = (value * 100.0).round() / 100.0;
    let text = format!("{rounded:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    match text.split_once('.') {
        Some((whole, frac)) => (whole.to_string(), format!(".{frac}"), text),
        None => (text.clone(), String::new(), text),
    }
}

fn score_figure(value: f64, class: &str, attrs: &str) -> String {
    let (whole, frac, title) = score_parts(value);
    let inner = if frac.is_empty() {
        format!("<span class=\"whole\">{whole}</span>", whole = esc(&whole))
    } else {
        format!(
            "<span class=\"whole\">{whole}</span><span class=\"frac\">{frac}</span>",
            whole = esc(&whole),
            frac = esc(&frac),
        )
    };
    let attrs = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {attrs}")
    };
    format!(
        "<span class=\"{class}\"{attrs} title=\"{title}\">{inner}</span>",
        title = esc(&title),
    )
}

fn colored_figure(value: f64, score: f64, mean: f64) -> String {
    let figure = score_figure(value, "score-compact score-meta-num", "");
    if !score.is_finite() || !mean.is_finite() || (score - mean).abs() < f64::EPSILON {
        return figure;
    }
    let class = if score > mean { "cmp-low" } else { "cmp-high" };
    format!("<span class=\"{class}\">{figure}</span>")
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
             <p class=\"lead\">SourceCritic is a static site. This notice covers the LGPD (Brazil) and the GDPR (EEA, UK, and Switzerland).</p>\
             <p>We collect the least we can. The pages themselves do not ask for an account. If you subscribe to the newsletter, the email address goes to the provider named in the form (Buttondown, MailerLite, or Beehiiv) and is used only to send the weekly releases and scores. If you email us, we keep that message to reply.</p>\
             <p>Google Tag Manager loads on each page. Advertising (Google AdSense) and measurement (Google Analytics) stay off until you choose. Visitors in the EEA, the UK, and Switzerland see Google's certified Privacy &amp; messaging CMP. Everyone else sees our banner. Consent Mode v2 defaults to denied for ad_storage, ad_user_data, ad_personalization, and analytics_storage. Accepting allows personalized ads and analytics. Rejecting outside those regions still shows non-personalized ads: ad_storage is granted, ad_user_data and ad_personalization stay denied, and AdSense is asked for non-personalized ads. Analytics stays off.</p>\
             <p>Purpose: show critic scores, fund hosting with ads, and understand which pages are used. Legal bases: consent for cookies, and legitimate interest for the publishing of publicly available review scores.</p>\
             <p>You can ask for access, correction, deletion, or portability of personal data we hold, and you can withdraw cookie consent by clearing site data. Contact <a href=\"mailto:joeltondbrandao@freedxm.com\">joeltondbrandao@freedxm.com</a>.</p>\
             <p>Reviews belong to the outlets that wrote them. SourceCritic shows the score, a short excerpt of one or two sentences, and a link to the original. We do not republish the full text.</p>"
        )
    } else {
        String::from(
            "<h1 class=\"page\">Terms of use</h1>\
             <p class=\"lead\">Using SourceCritic means you accept these terms.</p>\
             <p>Scores are an editorial transformation of published critic reviews. They are not an official score from any outlet, Metacritic, or OpenCritic. The method is described on <a href=\"/score/\">how the score works</a> and <a href=\"/about/\">about</a>.</p>\
             <p>Review text, scores, and names belong to the outlets. We display the score, a short excerpt, and a link. Do not treat an excerpt as the full review. Where to buy buttons may earn a commission at no extra cost to you; prices are not shown until a store feed is connected.</p>\
             <p>The site is provided as is. We do not promise that every score, platform, or year is complete. For privacy requests, use the address on <a href=\"/privacy/\">privacy</a>.</p>"
        )
    }
}

fn buy_block(title: &str, platforms: &[String]) -> String {
    let value = site_config();
    let Some(stores) = value.get("stores").and_then(|item| item.as_array()) else {
        return String::new();
    };
    let has_pc = platforms.iter().any(|platform| platform_key(platform) == "pc");
    let has_console = platforms
        .iter()
        .any(|platform| {
            let key = platform_key(platform);
            !key.is_empty() && key != "pc"
        });
    if !has_pc && !has_console {
        return String::new();
    }
    let query = urlencoding_min(title);
    let mut buttons = String::new();
    for store in stores {
        let kinds: Vec<&str> = store
            .get("kinds")
            .and_then(|item| item.as_array())
            .map(|items| items.iter().filter_map(|item| item.as_str()).collect())
            .unwrap_or_default();
        let fits = (has_pc && kinds.contains(&"pc")) || (has_console && kinds.contains(&"console"));
        if !fits {
            continue;
        }
        let id_key = store.get("id_key").and_then(|item| item.as_str()).unwrap_or("");
        let id = configured_id(id_key);
        let name = store.get("name").and_then(|item| item.as_str()).unwrap_or("Store");
        let template = store.get("url").and_then(|item| item.as_str()).unwrap_or("");
        if template.is_empty() || template.contains("YOUR_") {
            continue;
        }
        let url = store_url(template, &query, id.as_deref());
        if url.contains("YOUR_") || url.contains("{id}") {
            continue;
        }
        buttons.push_str(&format!(
            "<a href=\"{url}\" rel=\"sponsored nofollow noopener\" target=\"_blank\">{name}<small data-price-brl></small></a>",
            url = esc(&url),
            name = esc(name),
        ));
    }
    if buttons.is_empty() {
        return String::new();
    }
    format!(
        "<section class=\"buy-block\"><h2>Where to buy</h2>\
         <div class=\"buy-actions\">{buttons}</div>\
         <p class=\"buy-note\">Affiliate links: we may earn a commission at no cost to you.</p></section>"
    )
}

/// Search URL for a store. A placeholder affiliate id is left off the link.
fn store_url(template: &str, query: &str, id: Option<&str>) -> String {
    let (base, query_string) = template.split_once('?').unwrap_or((template, ""));
    let mut params = Vec::new();
    if !query_string.is_empty() {
        for part in query_string.split('&') {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            if value.contains("{id}") {
                if let Some(id) = id {
                    params.push(format!("{key}={id}"));
                }
                continue;
            }
            params.push(part.replace("{query}", query));
        }
    }
    let path = base.replace("{query}", query);
    if params.is_empty() {
        path
    } else {
        format!("{path}?{}", params.join("&"))
    }
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

fn platform_launch_year(key: &str) -> i32 {
    match key {
        "ps5" | "xbox-series" => 2020,
        "ps4" | "xbox-one" => 2013,
        "switch" => 2017,
        "switch-2" => 2025,
        "ps3" | "xbox-360" | "wii" => 2006,
        "wii-u" | "vita" => 2012,
        "3ds" => 2011,
        "ds" | "psp" => 2004,
        "gamecube" | "xbox" => 2001,
        "n64" => 1996,
        "ps2" => 2000,
        "ps1" => 1994,
        "ios" => 2008,
        _ => 0,
    }
}

/// A game belongs on /best/{platform}/{year}/ only when it launched there that year.
fn released_on(game: &GameView, platform: &str, year: i32) -> bool {
    if !game.platform_years.is_empty() {
        return game
            .platform_years
            .iter()
            .any(|(key, released)| key == platform && *released == year);
    }
    game.release_year == Some(year)
        && year >= platform_launch_year(platform)
        && game
            .platforms
            .iter()
            .any(|item| platform_key(item) == platform)
}

fn habit_row(outlet: &crate::db::OutletView) -> String {
    let reviews = if outlet.n == 1 {
        "1 review".to_string()
    } else {
        format!("{} reviews", outlet.n)
    };
    format!(
        "<a class=\"row\" href=\"/outlet/{slug}/\"><span><strong>{name}</strong><small>{reviews} · μ′ {mu}</small></span><span class=\"score\">{mu}</span></a>",
        slug = esc(&outlet.slug),
        name = esc(&outlet.name),
        reviews = reviews,
        mu = fmt_num(outlet.mu_prime),
    )
}

fn discovery_pages(games: &[GameView], outlets: &[crate::db::OutletView]) -> Vec<(String, String)> {
    let mut pages = Vec::new();
    let mut divisive: Vec<&GameView> = games
        .iter()
        .filter(|game| game.review_count >= 10)
        .collect();
    divisive.sort_by(|a, b| {
        let a_std = population_std(&scored_norms(a));
        let b_std = population_std(&scored_norms(b));
        b_std.partial_cmp(&a_std).unwrap_or(std::cmp::Ordering::Equal)
    });
    divisive.truncate(40);
    let lead_std = divisive.first().map(|game| {
        population_std(&scored_norms(game))
    });
    let intro = match (divisive.first(), lead_std) {
        (Some(game), Some(std)) => format!(
            "{} games with at least 10 scored reviews, ordered by how far the outlets sit from each other. {} leads, with a standard deviation of {:.1} points.",
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
        true,
        "",
    ));

    let mut gaps: Vec<&GameView> = games
        .iter()
        .filter(|game| game.review_count >= 10)
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
        false,
        "",
    ));

    let mut generous: Vec<&crate::db::OutletView> = outlets.iter().filter(|outlet| outlet.n >= 20).collect();
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
        _ => "Outlet habits appear once a publication has at least 20 scored reviews.".into(),
    };
    let habits = breadcrumb(&[
        Crumb::new("Home", "/", false),
        Crumb::new("Outlets", "/outlets/", false),
        Crumb::new("Outlet habits", "/outlets/ranking/", true),
    ]);
    let mut body = format!(
        "{habits}<h1 class=\"page\">Outlet habits</h1><p class=\"lead\">{rank_intro}</p><h2>Most generous</h2><div id=\"outlet-list\">"
    );
    for outlet in &generous {
        body.push_str(&habit_row(outlet));
    }
    body.push_str("</div><h2>Harshest</h2><div>");
    for outlet in &harsh {
        body.push_str(&habit_row(outlet));
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

    let best_home = breadcrumb(&[
        Crumb::new("Home", "/", false),
        Crumb::new("Best of", "/best/", true),
    ]);
    let mut best_index = format!(
        "{best_home}<h1 class=\"page\">Best of</h1><p class=\"lead\">Highest SourceCritic scores for a platform and a year, among games with at least 5 reviews.</p><div>",
    );
    let mut best_count = 0usize;
    let platform_keys = collect_platforms(games);
    let year_list = collect_years(games);
    for (key, label) in platform_keys
        .iter()
        .filter(|(key, _)| best_platform_keys().contains(&key.as_str()))
    {
        let mut platform_years_linked = Vec::new();
        let mut emitted_legacy = false;
        for year in &year_list {
            let mut picked = scored_on(games, key, *year);
            if picked.len() < 5 {
                if has_legacy_release(games, key, *year) {
                    emitted_legacy = true;
                    let target = format!("/best/{key}/");
                    pages.push((
                        format!("best/{key}/{year}/index.html"),
                        redirect_page(&target),
                    ));
                }
                continue;
            }
            picked.truncate(25);
            let avg = picked.iter().filter_map(|game| game.agg_score).sum::<f64>() / picked.len() as f64;
            let leader = &picked[0];
            let intro = format!(
                "Best {label} games released in {year}. The top {n} average {avg}. {title} leads with {score}.",
                n = picked.len(),
                avg = fmt_score(avg),
                title = esc(&leader.title),
                score = fmt_score(leader.agg_score.unwrap_or(0.0)),
            );
            let path = format!("/best/{key}/{year}/");
            let crumbs = breadcrumb(&[
                Crumb::new("Home", "/", false),
                Crumb::new("Best of", "/best/", false),
                Crumb::new(label, &format!("/best/{key}/"), false),
                Crumb::new(&year.to_string(), &path, true),
            ]);
            pages.push(list_page(
                &format!("best/{key}/{year}/index.html"),
                &path,
                &format!("Best {label} games of {year} | SourceCritic"),
                &format!("Best {label} games of {year}"),
                &intro,
                &picked,
                false,
                &crumbs,
            ));
            platform_years_linked.push((*year, path.clone()));
            best_index.push_str(&format!(
                "<a class=\"row\" href=\"{path}\"><span><strong>Best {label} games of {year}</strong></span></a>"
            ));
            best_count += 1;
        }
        if !platform_years_linked.is_empty() || emitted_legacy {
            let crumbs = breadcrumb(&[
                Crumb::new("Home", "/", false),
                Crumb::new("Best of", "/best/", false),
                Crumb::new(label, &format!("/best/{key}/"), true),
            ]);
            let mut body = format!(
                "{crumbs}<h1 class=\"page\">Best {label} games</h1><p class=\"lead\">Years with at least five scored games that launched on {label}.</p><div>"
            );
            if platform_years_linked.is_empty() {
                body.push_str("<p class=\"empty\">No year has five scored games yet.</p>");
            }
            for (year, path) in &platform_years_linked {
                body.push_str(&format!(
                    "<a class=\"row\" href=\"{path}\"><span><strong>{label} {year}</strong></span></a>"
                ));
            }
            body.push_str("</div>");
            pages.push((
                format!("best/{key}/index.html"),
                document(
                    Section::Other,
                    &format!("Best {label} games | SourceCritic"),
                    &body,
                    Some(&format!("/best/{key}/")),
                    &Seo {
                        description: format!("Best {label} games by release year."),
                        ..Seo::default()
                    },
                )
                .0,
            ));
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
    show_spread: bool,
    crumbs: &str,
) -> (String, String) {
    let mut body = format!(
        "{crumbs}<h1 class=\"page\">{heading}</h1><p class=\"lead\">{intro}</p><div id=\"game-list\">",
        heading = esc(heading),
    );
    for game in games {
        let slug = public_slug(&game.slug, &game.title);
        let reviews = if game.review_count == 1 {
            "1 review".to_string()
        } else {
            format!("{} reviews", game.review_count)
        };
        // Most divisive is ranked by how far outlets sit from each other, so the
        // row shows that standard deviation and not the score-versus-mean gap.
        let (detail, figures) = if show_spread {
            let spread = population_std(&scored_norms(game));
            (
                esc(&reviews),
                format!(
                    "<span class=\"score-pair\">\
                       <span class=\"score-pair-item\">{html}<small>std</small></span>\
                     </span>",
                    html = score_figure(spread, "score-compact score-open", ""),
                ),
            )
        } else {
            let score = game.agg_score.unwrap_or(0.0);
            let mean = game.simple_mean.unwrap_or(0.0);
            let gap = score - mean;
            let tone = if gap > 0.05 {
                "cmp-low"
            } else if gap < -0.05 {
                "cmp-high"
            } else {
                ""
            };
            let paint = |html: String| -> String {
                if tone.is_empty() {
                    html
                } else {
                    format!("<span class=\"{tone}\">{html}</span>")
                }
            };
            (
                format!(
                    "{reviews} · gap {gap}",
                    reviews = esc(&reviews),
                    gap = paint(score_figure(gap, "score-compact score-meta-num", "")),
                ),
                format!(
                    "<span class=\"score-pair\">\
                       <span class=\"score-pair-item\">{score_html}<small>score</small></span>\
                       <span class=\"score-pair-item\">{mean_html}<small>mean</small></span>\
                     </span>",
                    score_html = paint(score_figure(score, "score-compact", "")),
                    mean_html = paint(score_figure(mean, "score-compact", "")),
                ),
            )
        };
        body.push_str(&format!(
            "<a class=\"row\" href=\"/game/{slug}/\">\
               <span class=\"title-block\"><strong>{title}</strong>\
                 <small>{detail}</small></span>\
               {figures}\
             </a>",
            slug = esc(&slug),
            title = esc(&game.title),
            detail = detail,
            figures = figures,
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

pub(crate) fn audit_pages(files: &[(String, String)]) -> Result<(), String> {
    const MOJIBAKE: &[&str] = &[
        "Ã©", "Ã¡", "Ã­", "Ã³", "Ãº", "Ã±", "Ã£", "Ã§", "â€™", "â€œ", "â€“", "â€”", "ï¿½",
    ];
    for (path, html) in files {
        for marker in MOJIBAKE {
            if html.contains(marker) {
                return Err(format!("{path} contains mojibake {marker}"));
            }
        }
        if html.contains("YOUR_") {
            return Err(format!("{path} publishes a placeholder id"));
        }
        if !(path.starts_with("game/") || path.starts_with("outlet/")) {
            continue;
        }
        for bad in ["savethechildren.org", "metacritic.com", "opencritic.com", "proofpoint.com", "anyforsoft.com"] {
            if html.contains(bad) {
                return Err(format!("{path} links to {bad}"));
            }
        }
        if !path.starts_with("game/") || refresh_target(html).is_some() {
            continue;
        }
        let listed = html.matches("<div class=\"review\"").count();
        let shown = hero_review_count(html).ok_or_else(|| format!("{path} has no review total"))?;
        if shown != listed {
            return Err(format!("{path} shows {shown} reviews but lists {listed}"));
        }
        if listed >= 1 {
            if !has_rating_count(html, listed) {
                return Err(format!("{path} ratingCount is not {listed}"));
            }
        } else if html.contains("aggregateRating") {
            return Err(format!("{path} has aggregateRating with {listed} reviews"));
        }
        if let Some(start) = html.find("<p class=\"excerpt\">") {
            let rest = &html[start..];
            if let Some(end) = rest.find("</p>") {
                let text = &rest["<p class=\"excerpt\">".len()..end];
                if text.chars().count() > 320 {
                    return Err(format!("{path} excerpt is longer than 300 characters"));
                }
            }
        }
    }

    let mut content = BTreeSet::new();
    let mut redirects = BTreeSet::new();
    for (path, html) in files {
        if refresh_target(html).is_some() {
            if !html.contains("name=\"robots\" content=\"noindex\"") {
                return Err(format!("{path} redirect is missing noindex"));
            }
            if html.contains("BreadcrumbList") {
                return Err(format!("{path} redirect publishes a breadcrumb"));
            }
            if let Some(url) = file_url(path) {
                redirects.insert(url);
            }
            continue;
        }
        if let Some(url) = file_url(path) {
            content.insert(url);
        }
        if needs_breadcrumb(path)
            && (!html.contains("aria-label=\"Breadcrumb\"") || !html.contains("BreadcrumbList"))
        {
            return Err(format!("{path} is missing a breadcrumb"));
        }
    }
    for (path, html) in files {
        if refresh_target(html).is_some() {
            continue;
        }
        for href in anchor_hrefs(&without_scripts(html)) {
            let Some(url) = normalize_href(&href) else {
                continue;
            };
            if redirects.contains(&url) {
                return Err(format!("{path} links to redirect {url}"));
            }
            if !content.contains(&url) {
                return Err(format!("{path} links to missing {url}"));
            }
        }
    }
    Ok(())
}

fn needs_breadcrumb(path: &str) -> bool {
    path.starts_with("game/")
        || path.starts_with("outlet/")
        || path == "outlets/index.html"
        || path == "outlets/ranking/index.html"
        || path.starts_with("best/")
}

fn file_url(path: &str) -> Option<String> {
    if path == "404.html" {
        return None;
    }
    if path == "index.html" {
        return Some("/".to_string());
    }
    path.strip_suffix("/index.html")
        .map(|dir| format!("/{dir}/"))
}

fn has_rating_count(html: &str, count: usize) -> bool {
    let needle = format!("\"ratingCount\":{count}");
    let mut rest = html;
    while let Some(index) = rest.find(&needle) {
        let after = index + needle.len();
        let next = rest[after..].chars().next();
        if !next.is_some_and(|ch| ch.is_ascii_digit()) {
            return true;
        }
        rest = &rest[after..];
    }
    false
}

fn without_scripts(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("</script>") else {
            break;
        };
        rest = &rest[start + end + "</script>".len()..];
    }
    out.push_str(rest);
    out
}

fn anchor_hrefs(html: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<a ") {
        rest = &rest[start + 3..];
        let Some(tag_end) = rest.find('>') else {
            break;
        };
        let tag = &rest[..tag_end];
        rest = &rest[tag_end + 1..];
        let Some(href_at) = tag.find("href=\"") else {
            continue;
        };
        let value = &tag[href_at + 6..];
        let Some(end) = value.find('"') else {
            continue;
        };
        hrefs.push(value[..end].to_string());
    }
    hrefs
}

fn normalize_href(href: &str) -> Option<String> {
    let href = href.split(['?', '#']).next().unwrap_or(href);
    if href.is_empty()
        || href.starts_with("mailto:")
        || href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("//")
    {
        return None;
    }
    if !href.starts_with('/') {
        return None;
    }
    if href == "/" {
        return Some("/".to_string());
    }
    if href.rsplit('/').next().is_some_and(|tail| tail.contains('.')) {
        return None;
    }
    if href.ends_with('/') {
        Some(href.to_string())
    } else {
        Some(format!("{href}/"))
    }
}

fn hero_review_count(html: &str) -> Option<usize> {
    // The script in the head also says "Simple mean" and "review", so the
    // published total is the hero's data-count, not the first digit in the page.
    let rest = html.split("id=\"score-hero-meta\"").nth(1)?;
    let tag = rest.split('>').next()?;
    let count = tag.split("data-count=\"").nth(1)?.split('"').next()?;
    count.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_releases_list_from_three_reviews() {
        let today = days_from_civil(2026, 11, 26);
        assert!(passes_min_on(4, Some("2026-11-10"), today, 10));
        assert!(!passes_min_on(2, Some("2026-11-10"), today, 10));
        assert!(!passes_min_on(9, Some("2004-04-01"), today, 10));
        assert!(passes_min_on(10, Some("2004-04-01"), today, 10));
        assert!(passes_min_on(5, Some("2004-04-01"), today, 0));
        assert_eq!(DEFAULT_MIN_REVIEWS, 0);
        let filters = GameFilters {
            platform: "all".into(),
            year: "all".into(),
            min_reviews: DEFAULT_MIN_REVIEWS,
        };
        let html = browse_controls(&filters, &[], &[], "score");
        assert!(html.contains("value=\"0\" selected>Any</option>"), "{html}");
        assert!(!filters_are_active(&filters));
        assert!(is_hub_url("https://easyallies.com"));
        assert!(is_hub_url("https://www.ign.com/xbox"));
        assert!(is_hub_url("https://www.ign.com/playstation"));
        assert!(!is_hub_url("https://www.ign.com/articles/bioshock-review"));
        assert!(!is_hub_url("https://ztgd.com/the-legend-of-zelda-breath-of-the-wild/review"));
        assert!(is_hub_url("https://www.cubed3.com/games/reviews"));
    }

    #[test]
    fn listing_badge_is_printed_once() {
        let mut game = bare_game("far-far-west", "Far Far West", 2024, &["pc"], Some(70.0));
        game.review_count = 4;
        let filters = GameFilters {
            platform: "all".into(),
            year: "all".into(),
            min_reviews: 0,
        };
        let html = game_list(&[game], "score", &filters, &[], &[], 1, 1, 1, 1);
        assert_eq!(html.matches("provisional score").count(), 1, "{html}");
    }

    #[test]
    fn mojibake_fails_the_build() {
        let files = vec![(
            "game/tiny-tinas-wonderlands/index.html".to_string(),
            "<p>clichÃ©d</p>".to_string(),
        )];
        let error = audit_pages(&files).unwrap_err();
        assert!(error.contains("mojibake"), "{error}");
    }

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
    fn review_total_comes_from_the_hero_not_the_script() {
        let html = r#"<script>heroMeta.innerHTML = "Simple mean " + count + " review"</script>
<span id="score-hero-meta" data-mean="80" data-count="11">Simple mean 80 · 11 reviews</span>"#;
        assert_eq!(hero_review_count(html), Some(11));
    }

    #[test]
    fn hover_score_keeps_the_integer_and_the_fraction() {
        let html = score_figure(82.37, "score score-compact", "data-score-display");
        assert!(html.contains(">82<"));
        assert!(html.contains(".37"));
        assert!(html.contains("score-compact"));
        let whole = score_figure(80.0, "score score-compact", "");
        assert!(!whole.contains("frac"));
    }

    #[test]
    fn playstation_slugs_keep_the_brand_spelling() {
        assert_eq!(platform_label(&platform_key("playstation")), "PlayStation");
        assert_eq!(platform_label(&platform_key("playstation-2")), "PlayStation 2");
        assert_eq!(platform_label(&platform_key("playstation-3")), "PlayStation 3");
        assert_eq!(platform_label(&platform_key("playstation-vita")), "PlayStation Vita");
        assert_eq!(platform_label(&platform_key("playstation-5")), "PlayStation 5");
    }

    #[test]
    fn refresh_target_reads_the_meta_url() {
        let html = redirect_page("/best/ps2/");
        assert_eq!(refresh_target(&html).as_deref(), Some("/best/ps2/"));
        assert!(html.contains("name=\"robots\" content=\"noindex\""));
        assert!(html.contains("rel=\"canonical\" href=\"https://sourcecritic.freedxm.com/best/ps2/\""));
        assert!(refresh_target("<p>no redirect</p>").is_none());
    }

    fn bare_game(slug: &str, title: &str, year: i32, platforms: &[&str], score: Option<f64>) -> GameView {
        GameView {
            slug: slug.into(),
            title: title.into(),
            cover_path: None,
            release_year: Some(year),
            released_on: None,
            platforms: platforms.iter().map(|platform| (*platform).to_string()).collect(),
            platform_years: Vec::new(),
            synopsis: None,
            agg_score: score,
            simple_mean: score,
            review_count: if score.is_some() { 5 } else { 0 },
            mu_global: 70.0,
            sigma_global: 10.0,
            reviews: Vec::new(),
        }
    }

    #[test]
    fn breadcrumb_urls_are_absolute_and_slashed() {
        let html = breadcrumb(&[
            Crumb::new("Home", "/", false),
            Crumb::new("Best of", "/best/", false),
            Crumb::new("PlayStation 5", "/best/ps5/", true),
        ]);
        assert!(html.contains("aria-label=\"Breadcrumb\""));
        assert!(html.contains("href=\"/best/\""));
        assert!(html.contains("https://sourcecritic.freedxm.com/"));
        assert!(html.contains("https://sourcecritic.freedxm.com/best/"));
        assert!(html.contains("https://sourcecritic.freedxm.com/best/ps5/"));
        assert!(html.contains("\"@type\":\"BreadcrumbList\""));
    }

    #[test]
    fn primary_release_is_the_oldest_hub() {
        let mut early = bare_game("early", "Early", 2018, &["PlayStation 4", "PlayStation 5"], Some(80.0));
        early.platform_years = vec![("ps4".into(), 2018), ("ps5".into(), 2024)];
        let catalog = BestCatalog {
            hubs: ["ps4".to_string(), "ps5".to_string()].into_iter().collect(),
            years: [("ps4".into(), 2018), ("ps5".into(), 2024)].into_iter().collect(),
        };
        assert_eq!(
            primary_release(&early, &catalog).as_ref().map(|(key, year, page)| (key.as_str(), *year, *page)),
            Some(("ps4", 2018, true))
        );
        let multi = bare_game("hades", "Hades", 2020, &["PC", "iOS", "PlayStation 5"], Some(90.0));
        let multi_catalog = BestCatalog {
            hubs: ["pc", "ios", "ps5"].into_iter().map(str::to_string).collect(),
            years: [("pc".into(), 2020), ("ps5".into(), 2020)].into_iter().collect(),
        };
        assert_eq!(
            primary_release(&multi, &multi_catalog).as_ref().map(|(key, year, page)| (key.as_str(), *year, *page)),
            Some(("pc", 2020, true))
        );
        let only_dreamcast = bare_game("dc", "Dream", 2000, &["Dreamcast"], Some(80.0));
        assert!(primary_release(&only_dreamcast, &catalog).is_none());
    }

    #[test]
    fn similar_scores_stay_in_the_year_and_stop_at_six() {
        let mut games = vec![bare_game("self", "Self", 2020, &["PC"], Some(80.0))];
        for index in 0..8 {
            games.push(bare_game(
                &format!("g{index}"),
                &format!("G{index}"),
                2020,
                &["PC"],
                Some(80.0 + index as f64),
            ));
        }
        games.push(bare_game("other-year", "Other", 2019, &["PC"], Some(80.0)));
        games.push(bare_game("unscored", "Unscored", 2020, &["PC"], None));
        let picked = similar_games(&games[0], &games);
        assert_eq!(picked.len(), 6);
        assert_eq!(picked[0].slug, "g0");
        assert!(picked.iter().all(|game| game.release_year == Some(2020)));
        assert!(picked.iter().all(|game| game.slug != "self"));
    }

    #[test]
    fn best_catalog_keeps_thin_years_off_the_list() {
        let mut games = Vec::new();
        for index in 0..4 {
            let mut game = bare_game(&format!("g{index}"), &format!("G{index}"), 2024, &["PlayStation 5"], Some(80.0));
            game.review_count = 5;
            games.push(game);
        }
        let thin = best_catalog(&games);
        assert!(thin.years.is_empty());
        assert!(thin.hubs.contains("ps5"));
        let mut fifth = bare_game("g4", "G4", 2024, &["PlayStation 5"], Some(90.0));
        fifth.review_count = 5;
        games.push(fifth);
        let ready = best_catalog(&games);
        assert!(ready.years.contains(&("ps5".to_string(), 2024)));
    }

    #[test]
    fn best_menu_lists_only_years_with_a_page() {
        let mut games = Vec::new();
        for index in 0..5 {
            let mut game = bare_game(
                &format!("g{index}"),
                &format!("G{index}"),
                2024,
                &["PlayStation 5"],
                Some(80.0),
            );
            game.review_count = 5;
            games.push(game);
        }
        let items = best_nav_items(&games);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "ps5");
        assert_eq!(items[0].years, vec![2024]);
        let html = best_menu_markup(&items, "/best/ps5/2024/");
        assert!(html.contains("class=\"nav-drop\""));
        assert!(html.contains("href=\"/best/ps5/\""));
        assert!(html.contains("href=\"/best/ps5/2024/\" aria-current=\"page\""));
        games.pop();
        let thin_items = best_nav_items(&games);
        assert!(thin_items.is_empty());
        let thin = best_menu_markup(&thin_items, "/");
        assert!(!thin.contains("nav-drop"));
        assert!(thin.contains("href=\"/best/\""));
    }

    #[test]
    fn rating_count_does_not_match_a_longer_number() {
        assert!(has_rating_count("{\"ratingCount\":5}", 5));
        assert!(!has_rating_count("{\"ratingCount\":50}", 5));
        assert!(has_rating_count("{\"ratingCount\":1}", 1));
    }

    #[test]
    fn a_named_port_does_not_keep_the_shorter_platform() {
        assert_eq!(
            mentioned_platforms("This Switch 2 port of Resident Evil 4 holds up."),
            vec!["switch-2".to_string()]
        );
        assert_eq!(
            mentioned_platforms("The PS5 version is the one to play."),
            vec!["ps5".to_string()]
        );
        assert_eq!(
            mentioned_platforms("https://example.com/reviews/playstation-5/hades"),
            vec!["ps5".to_string()]
        );
        assert!(mentioned_platforms("A landmark remake.").is_empty());
    }

    fn bare_review(excerpt: &str, url: &str, platforms: &[&str]) -> ReviewView {
        ReviewView {
            outlet_slug: "ign".into(),
            outlet_name: "IGN".into(),
            raw_score: Some(90.0),
            scale_max: Some(100.0),
            raw_score_norm: 90.0,
            z_score: 0.0,
            excerpt: Some(excerpt.into()),
            review_url: Some(url.into()),
            platforms: platforms.iter().map(|platform| (*platform).to_string()).collect(),
            outlet_n: 1,
            mu: 0.0,
            sigma: 0.0,
            mu_prime: 0.0,
            sigma_prime: 0.0,
        }
    }

    #[test]
    fn a_filtered_review_stays_hidden_despite_the_grid() {
        assert!(CSS.contains(".review[hidden]"));
        assert!(CSS.contains(".unscored[hidden]"));
    }

    #[test]
    fn breath_of_the_wild_ports_ignore_machines_the_game_never_shipped_on() {
        let catalog = vec!["switch".into(), "switch-2".into(), "wii-u".into()];
        let switch_2 = bare_review("The Switch 2 edition is the one to replay.", "", &[]);
        assert_eq!(review_port_keys(&switch_2, &catalog), vec!["switch-2".to_string()]);
        let mut unverifiable = bare_review("A landmark adventure.", "", &[]);
        assert!(counts_for_score(&unverifiable));
        assert!(!verifiable_url(unverifiable.review_url.as_deref()));
        unverifiable.review_url = None;
        assert!(counts_for_score(&unverifiable));
        unverifiable.review_url = Some("https://easyallies.com".into());
        assert!(counts_for_score(&unverifiable));
        assert!(!verifiable_url(unverifiable.review_url.as_deref()));
        unverifiable.review_url = Some("https://www.savethechildren.org/us/ways-to-help".into());
        assert!(!counts_for_score(&unverifiable));
        unverifiable.review_url = Some("https://ign.com/articles/zelda".into());
        assert!(counts_for_score(&unverifiable));
        let original = bare_review("A landmark adventure.", "https://ign.com/articles/zelda", &[]);
        assert_eq!(
            review_port_keys(&original, &catalog),
            vec!["switch".to_string(), "wii-u".to_string()]
        );
        let aside = bare_review(
            "It doesn't feature eye-popping graphics like games on PC and PS4.",
            "https://example.com/beyond-playstation/zelda-review",
            &[],
        );
        assert!(review_port_keys(&aside, &catalog).is_empty());
        let stored = bare_review("A landmark adventure.", "", &["ps1"]);
        assert!(review_port_keys(&stored, &catalog).is_empty());
    }

    #[test]
    fn aggregator_hundred_is_shown_on_the_outlet_scale() {
        assert_eq!(presented_raw("ign", Some(90.0), Some(100.0), 90.0), "9/10");
        assert_eq!(presented_raw("eurogamer", Some(80.0), Some(100.0), 80.0), "4/5");
        assert_eq!(presented_raw("gamesradar", Some(100.0), Some(100.0), 100.0), "5/5");
        assert_eq!(presented_raw("jeuxvideo-com", Some(80.0), Some(100.0), 80.0), "16/20");
        assert_eq!(presented_raw("weekly-famitsu", Some(90.0), Some(100.0), 90.0), "36/40");
        assert_eq!(presented_raw("pc-gamer", Some(85.0), Some(100.0), 85.0), "85/100");
        assert_eq!(presented_raw("ign", Some(8.5), Some(10.0), 85.0), "8.5/10");
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
