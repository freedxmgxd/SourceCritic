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

.row:first-of-type { border-top: 1px solid var(--line); }
.row:hover { background: linear-gradient(90deg, transparent, var(--surface-hover) 40%, transparent); }
.row .meta { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; }
.row .meta > span { min-width: 0; }
.row strong { display: block; font-size: 1.05rem; }
.row small { display: block; margin-top: 2px; color: var(--muted); font-size: 0.9rem; }

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

.filters label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--muted);
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.filters select {
  font: inherit;
  text-transform: none;
  letter-spacing: 0;
  color: var(--ink);
  border: 1px solid var(--line);
  border-radius: 10px;
  background: white;
  padding: 8px 10px;
  cursor: pointer;
}

.filters select:focus-visible {
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
  min-height: 0;
  overflow: hidden;
  border-radius: 10px;
  background: transparent;
  border: 0;
}

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
  .site-ad-rail {
    display: none !important;
  }
}

@media (max-width: 720px) {
  main { padding: 22px 18px 32px; }
  header { flex-direction: column; align-items: flex-start; }
  .stats { grid-template-columns: 1fr 1fr; }
  .filters { grid-template-columns: 1fr 1fr; }
  .browse-toolbar { align-items: flex-start; }
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
/// AdSense publisher client (`ca-pub-…`). Loaded only when the side rail shows (once/hour).
const ADSENSE_CLIENT: Option<&str> = Some("ca-pub-5490722539415533");
/// Side-rail display unit only. Fills the right column; main column width is unchanged.
/// Env `SOURCECRITIC_AD_HTML` overrides. At most once per browser hour.
const AD_HTML: Option<&str> = Some(
    r#"<ins class="adsbygoogle"
     style="display:block;width:100%;min-height:600px"
     data-ad-client="ca-pub-5490722539415533"
     data-ad-slot="8380670310"
     data-ad-format="vertical"
     data-full-width-responsive="false"></ins>"#,
);
/// How long before the side ad may show again for the same visitor (ms).
const AD_COOLDOWN_MS: u64 = 60 * 60 * 1000;
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
        meta.textContent = showMean
          ? ("score " + (row.getAttribute("data-score-label") || ""))
          : ("simple mean " + (row.getAttribute("data-mean-label") || ""));
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
  function filtersActive(form) {
    var platform = (form.platform && form.platform.value) || "all";
    var year = (form.year && form.year.value) || "all";
    var minReviews = Number((form.min_reviews && form.min_reviews.value) || 0);
    return platform !== "all" || year !== "all" || minReviews > 0;
  }
  function applyGameFilters() {
    var list = document.getElementById("game-list");
    var form = document.getElementById("game-filters");
    var meta = document.getElementById("filter-meta");
    var reset = document.getElementById("filter-reset");
    if (!list || !form) return;
    var platform = (form.platform && form.platform.value) || "all";
    var year = (form.year && form.year.value) || "all";
    var minReviews = Number((form.min_reviews && form.min_reviews.value) || 0);
    var total = Number(list.getAttribute("data-total") || 0);
    var shown = 0;
    Array.prototype.forEach.call(list.querySelectorAll(".row"), function (row) {
      var ok = true;
      if (platform !== "all") {
        var plats = (row.getAttribute("data-platforms") || "").split(/\s+/);
        ok = plats.indexOf(platform) !== -1;
      }
      if (ok && year !== "all") ok = row.getAttribute("data-year") === year;
      if (ok && minReviews > 0) ok = num(row, "data-reviews") >= minReviews;
      row.hidden = !ok;
      if (ok) shown += 1;
    });
    if (meta) meta.textContent = "Showing " + shown + " of " + total + " games.";
    var empty = document.getElementById("game-empty");
    if (empty) empty.hidden = shown !== 0 || total === 0;
    if (reset) reset.hidden = !filtersActive(form);
  }
  function resetGameFilters() {
    var form = document.getElementById("game-filters");
    if (!form) return;
    if (form.platform) form.platform.value = "all";
    if (form.year) form.year.value = "all";
    if (form.min_reviews) form.min_reviews.value = "0";
    applyGameFilters();
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
      sortChildren(list, btn.getAttribute("data-sort"), btn.getAttribute("data-sort"));
      if (listId === "game-list") applyGameFilters();
    });
    sortChildren(list, defaultKey, defaultKey);
    if (listId === "game-list") applyGameFilters();
  }
  document.addEventListener("DOMContentLoaded", function () {
    var form = document.getElementById("game-filters");
    if (form) {
      form.addEventListener("submit", function (ev) { ev.preventDefault(); applyGameFilters(); });
      form.addEventListener("change", applyGameFilters);
    }
    var reset = document.getElementById("filter-reset");
    if (reset) reset.addEventListener("click", resetGameFilters);
    bindSort("game-list", "score");
    bindSort("outlet-list", "reviews");
    bindSort("game-reviews", "name");
    bindSort("outlet-reviews", "name");
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
    gateSideAd();
  });
  function gateSideAd() {
    var aside = document.getElementById("site-ad");
    var src = document.getElementById("site-ad-src");
    if (!aside || !src) return;
    var cooldown = Number(aside.getAttribute("data-cooldown-ms") || 3600000);
    var client = aside.getAttribute("data-ad-client") || "";
    var key = "sc_ad_shown_at";
    var now = Date.now();
    var last = Number(localStorage.getItem(key) || 0);
    if (last && now - last < cooldown) {
      aside.remove();
      return;
    }
    var slot = aside.querySelector(".site-ad-slot");
    if (!slot) {
      aside.remove();
      return;
    }
    slot.innerHTML = src.textContent || "";
    src.remove();
    aside.hidden = false;
    localStorage.setItem(key, String(now));
    function fillUnit() {
      try {
        window.adsbygoogle = window.adsbygoogle || [];
        // Prefer the manual rail only — do not request page-level / Auto ads.
        window.adsbygoogle.push({
          google_ad_client: client,
          enable_page_level_ads: false
        });
        if (slot.querySelector(".adsbygoogle")) {
          window.adsbygoogle.push({});
        }
      } catch (e) {}
    }
    if (!client) {
      fillUnit();
      return;
    }
    if (window.__scAdsScriptLoading) {
      window.__scAdsScriptLoading.then(fillUnit);
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
    window.__scAdsScriptLoading.then(fillUnit);
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
    #[serde(default)]
    min_reviews: Option<i64>,
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
    let mut games = {
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
    let platforms = collect_platforms(&games);
    let years = collect_years(&games);
    let total = games.len();
    games.retain(|game| game_matches(game, &filters));
    sort_games(&mut games, sort);
    ok(
        Section::Games,
        "Games",
        &game_list(&games, sort, &filters, &platforms, &years, total),
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
            Ok(game) => game,
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
    let min_reviews = query.min_reviews.unwrap_or(0).max(0);
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
        "xbox series x" | "xbox series x/s" | "xbox series s" | "xbox-series-x" | "xbxs" => {
            "xbox-series".into()
        }
        "xbox one" | "xbox-one" => "xbox-one".into(),
        "nintendo switch 2" | "switch 2" | "nintendo-switch-2" => "switch-2".into(),
        "nintendo switch" | "switch" | "nintendo-switch" => "switch".into(),
        "pc" | "windows pc" | "windows" => "pc".into(),
        "ios" | "iphone/ipad" => "ios".into(),
        "android" => "android".into(),
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
        "xbox-series" => "Xbox Series X/S".into(),
        "xbox-one" => "Xbox One".into(),
        "switch-2" => "Nintendo Switch 2".into(),
        "switch" => "Nintendo Switch".into(),
        "pc" => "PC".into(),
        "ios" => "iOS".into(),
        "android" => "Android".into(),
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

fn game_list(
    games: &[GameView],
    sort: &str,
    filters: &GameFilters,
    platforms: &[(String, String)],
    years: &[i32],
    total_before_filter: usize,
) -> String {
    let mut html = String::from(
        "<h1 class=\"page\">Games</h1>\
         <p class=\"lead\">Outlet scores, adjusted so generous critics do not inflate a game and harsh critics do not drag it down.</p>",
    );
    html.push_str(&browse_controls(filters, platforms, years, sort));
    html.push_str(&format!(
        "<p class=\"filter-meta\" id=\"filter-meta\">Showing {shown} of {total} games.</p>",
        shown = games.len(),
        total = total_before_filter,
    ));
    if total_before_filter == 0 {
        html.push_str("<p class=\"empty\">No games yet.</p>");
        return html;
    }
    html.push_str(&format!(
        "<p class=\"empty\" id=\"game-empty\" hidden>No games match these filters.</p>\
         <div id=\"game-list\" data-total=\"{total}\">",
        total = total_before_filter,
    ));
    for game in games {
        let count = if game.review_count == 1 {
            "1 review".to_string()
        } else {
            format!("{} reviews", game.review_count)
        };
        let cover = cover_img(game.cover_path.as_deref(), false);
        let score_value = game.agg_score.unwrap_or(0.0);
        let mean_value = game.simple_mean.unwrap_or(0.0);
        let score_label = fmt_num(score_value);
        let mean_label = fmt_num(mean_value);
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
        html.push_str(&format!(
            "<a class=\"row\" href=\"/game/{slug}\" \
                data-score=\"{score_n}\" data-mean=\"{mean_n}\" data-reviews=\"{reviews}\" \
                data-title=\"{title_attr}\" data-year=\"{year}\" data-platforms=\"{plats}\" \
                data-score-label=\"{score_label}\" data-mean-label=\"{mean_label}\">\
               <span class=\"meta\">{cover}<span><strong>{title}</strong>\
                 <small><span data-score-meta>simple mean {mean_label}</span> · {count}{bits_tail}</small></span></span>\
               <span data-score-display>{score_html}</span>\
               <span data-mean-display hidden>{mean_html}</span>\
             </a>",
            slug = esc(&game.slug),
            title = esc(&game.title),
            title_attr = esc(&game.title),
            count = esc(&count),
            bits_tail = bits_tail,
            score_html = score_compact_html(score_value),
            mean_html = score_compact_html(mean_value),
            score_n = score_value,
            mean_n = mean_value,
            reviews = game.review_count,
            year = esc(&year_attr),
            plats = esc(&platform_keys.join(" ")),
            score_label = esc(&score_label),
            mean_label = esc(&mean_label),
            cover = cover,
        ));
    }
    html.push_str("</div>");
    html
}

fn filter_bar(
    filters: &GameFilters,
    platforms: &[(String, String)],
    years: &[i32],
) -> String {
    let mut html = String::from(
        "<form class=\"filters\" id=\"game-filters\" aria-label=\"Filters\">",
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
    filters.platform != "all" || filters.year != "all" || filters.min_reviews > 0
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
        filters = filter_bar(filters, platforms, years),
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
            "<a class=\"row\" href=\"/outlet/{slug}\" \
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
            _ => format!("{}", fmt_num(review.raw_score_norm)),
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
               <span><strong><a href=\"/game/{game_slug}\">{game}</a></strong>\
                 <small>{raw} → {norm} · \
                   <span data-z-meta>Z {z}</span>\
                   <span data-score-meta hidden>Score {mapped}</span>\
                 </small>\
               </span>\
               <span data-score-display class=\"mapped\" title=\"This review&apos;s contribution to the game score\">Score {mapped}</span>\
               <span data-z-display class=\"z\" hidden>Z {z}</span>\
               {excerpt}{source}\
             </div>",
            game_slug = esc(&review.game_slug),
            game = esc(&review.game_title),
            game_attr = esc(&review.game_title),
            raw = esc(&raw),
            norm = fmt_num(review.raw_score_norm),
            z = fmt_num(review.z_score),
            mapped = fmt_num(mapped),
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
         <p class=\"lead\">μ<sub>j</sub> and σ<sub>j</sub> are this outlet&apos;s raw mean and spread. μ′<sub>j</sub> and σ′<sub>j</sub> are the shrunk values used when scoring a game—pulled toward the catalog so a thin history cannot dominate. See <a href=\"/score\">how the score works</a>.</p>\
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

fn cover_img(path: Option<&str>, large: bool) -> String {
    let Some(path) = path.filter(|p| p.starts_with("/covers/")) else {
        return String::new();
    };
    let class = if large { "cover-lg" } else { "cover" };
    format!(
        "<img class=\"{class}\" src=\"{src}\" alt=\"\" width=\"{w}\" height=\"{h}\" loading=\"lazy\">",
        src = esc(path),
        w = if large { 140 } else { 56 },
        h = if large { 198 } else { 78 },
    )
}

fn game_page(game: &GameView, sort: &str) -> String {
    let count = if game.review_count == 1 {
        "1 review".to_string()
    } else {
        format!("{} reviews", game.review_count)
    };
    let cover = cover_img(game.cover_path.as_deref(), true);
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
    reviews.push_str(&sort_bar(
        &format!("/game/{}", game.slug),
        sort,
        "name",
        &[("name", "Outlet"), ("score", "Score"), ("z", "Z")],
    ));
    reviews.push_str("<div id=\"game-reviews\">");
    for review in &game.reviews {
        let raw = match (review.raw_score, review.scale_max) {
            (Some(score), Some(scale)) => format!("{score}/{scale}"),
            _ => format!("{}", fmt_num(review.raw_score_norm)),
        };
        let mapped = crate::engine::map_z(review.z_score, game.mu_global);
        let excerpt = review
            .excerpt
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(|text| format!("<p class=\"excerpt\">{}</p>", esc(text)))
            .unwrap_or_default();
        let source = match review.review_url.as_deref().filter(|url| {
            url.starts_with("https://") || url.starts_with("http://")
        }) {
            Some(url) => format!(
                "<p class=\"source\"><a href=\"{url}\" rel=\"noopener noreferrer\" target=\"_blank\">Read original review</a></p>",
                url = esc(url),
            ),
            None => String::new(),
        };
        reviews.push_str(&format!(
            "<div class=\"review\" data-title=\"{outlet_attr}\" data-score=\"{score_n}\" data-z=\"{z_n}\">\
               <span><strong><a href=\"/outlet/{outlet_slug}\">{outlet}</a></strong>\
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
            norm = fmt_num(review.raw_score_norm),
            z = fmt_num(review.z_score),
            mapped = fmt_num(mapped),
            score_n = mapped,
            z_n = review.z_score,
            excerpt = excerpt,
            source = source,
        ));
    }
    reviews.push_str("</div></section>");
    format!(
        "<div class=\"game-hero\">{cover}<div class=\"text\">\
           <h1 class=\"page\">{title}</h1>\
           <div class=\"score-hero\" aria-label=\"SourceCritic score {score}\">\
             <span class=\"score-hero-value\">{score}</span>\
             <span class=\"score-hero-copy\">\
               <span class=\"score-hero-label\">SourceCritic score</span>\
               <span class=\"score-hero-meta\">Simple mean {mean} · {count}</span>\
             </span>\
           </div>\
           {synopsis}\
           {meta}\
           <p class=\"lead\">The published score is adjusted by each outlet&apos;s history. See <a href=\"/score\">how the score works</a>.</p>\
         </div></div>\
         {reviews}",
        title = esc(&game.title),
        score = fmt_num(game.agg_score.unwrap_or(0.0)),
        mean = fmt_num(game.simple_mean.unwrap_or(0.0)),
        count = esc(&count),
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
           <p class=\"lead\">Short answers. For the math, see <a href=\"/score\">how the score works</a>.</p>\
           <div class=\"faq\">\
             <details open>\
               <summary>What is SourceCritic?</summary>\
               <p>SourceCritic aggregates critic reviews into a single 0–100 score. Unlike a plain average, each outlet is judged against <em>its own</em> scoring habit, so a soft 70 and a harsh 70 are not treated as the same signal.</p>\
             </details>\
             <details>\
               <summary>How is the score calculated?</summary>\
               <p>Every review is normalized to 0–100, compared to that outlet&apos;s history as a $z$-score, mapped back to 0–100, then averaged one vote per outlet. The full walkthrough is on <a href=\"/score\">The score</a>.</p>\
             </details>\
             <details>\
               <summary>Where do the reviews come from?</summary>\
               <p>Scores and excerpts are ingested from published critic outlets in the catalog. Each game page links back to the original review when a source URL is available.</p>\
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
         </section>\
         <section class=\"about-block\" id=\"donate\" aria-labelledby=\"donate-heading\">\
           <h2 id=\"donate-heading\">Donations</h2>\
           {donate_block}\
         </section>",
        contact_faq = contact_faq,
        contact_block = contact_block,
        donate_block = donate_block,
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
    (StatusCode::OK, document(section, title, body, None))
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
        ),
    )
}

fn ad_slot_html() -> String {
    let from_env = std::env::var("SOURCECRITIC_AD_HTML")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let markup = from_env.or_else(|| AD_HTML.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
    let client = ADSENSE_CLIENT.unwrap_or("");
    match markup {
        Some(inner) => format!(
            "<aside class=\"site-ad-rail\" id=\"site-ad\" hidden \
               data-cooldown-ms=\"{cooldown}\" data-ad-client=\"{client}\" aria-label=\"Advertisement\">\
               <span class=\"site-ad-label\">Ad</span>\
               <div class=\"site-ad-slot\"></div>\
               <script type=\"text/plain\" id=\"site-ad-src\">{inner}</script>\
             </aside>",
            cooldown = AD_COOLDOWN_MS,
            client = esc(client),
            // text/plain body must not close the script early; strip closers.
            inner = inner.replace("</script", "<\\/script"),
        ),
        None => String::new(),
    }
}

fn document(section: Section, title: &str, body: &str, path: Option<&str>) -> Html<String> {
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
    let canonical = match path {
        Some(p) if p == "/" || p.is_empty() => format!("{SITE_ORIGIN}/"),
        Some(p) => format!("{SITE_ORIGIN}{p}"),
        None => match section {
            Section::Games => format!("{SITE_ORIGIN}/"),
            Section::Outlets => format!("{SITE_ORIGIN}/outlets"),
            Section::Score => format!("{SITE_ORIGIN}/score"),
            Section::About => format!("{SITE_ORIGIN}/about"),
            Section::Other => format!("{SITE_ORIGIN}/"),
        },
    };
    Html(format!(
        "<!DOCTYPE html>\
         <html lang=\"en\">\
         <head>\
           <meta charset=\"utf-8\">\
           <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
           <title>{title} · SourceCritic</title>\
           <link rel=\"canonical\" href=\"{canonical}\">\
           <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\
           <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\
           <link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&family=IBM+Plex+Serif:wght@600&display=swap\" rel=\"stylesheet\">\
           {katex}\
           <style>{CSS}</style>\
           <script>{SITE_JS}</script>\
         </head>\
         <body>\
           <div class=\"page-shell\">\
             <main>\
               <header>\
                 <a class=\"brand\" href=\"/\">SourceCritic</a>\
                 <nav>\
                   <a href=\"/\"{games}>Games</a>\
                   <a href=\"/outlets\"{outlets}>Outlets</a>\
                   <a href=\"/score\"{score}>The score</a>\
                   <a href=\"/about\"{about}>About</a>\
                 </nav>\
               </header>\
               {body}\
             </main>\
             {ad}\
           </div>\
         </body>\
         </html>",
        title = esc(title),
        canonical = esc(&canonical),
        games = games,
        outlets = outlets,
        score = score,
        about = about,
        ad = ad,
    ))
}

/// Render every public HTML page for the static export.
/// Paths are relative (`index.html`, `game/{slug}/index.html`, …).
pub fn render_all_pages(db: &crate::db::Database) -> Result<Vec<(String, String)>, String> {
    let mut pages = Vec::new();

    let mut games = db.list_games().map_err(|e| e.to_string())?;
    let platforms = collect_platforms(&games);
    let years = collect_years(&games);
    let total = games.len();
    sort_games(&mut games, "score");
    let filters = GameFilters {
        platform: "all".into(),
        year: "all".into(),
        min_reviews: 0,
    };
    pages.push((
        "index.html".into(),
        document(
            Section::Games,
            "Games",
            &game_list(&games, "score", &filters, &platforms, &years, total),
            Some("/"),
        )
        .0,
    ));

    for game in &games {
        let mut detailed = db
            .get_game(&game.slug)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing game {}", game.slug))?;
        sort_game_reviews(&mut detailed.reviews, "name", detailed.mu_global);
        let path = format!("/game/{}", detailed.slug);
        pages.push((
            format!("game/{}/index.html", detailed.slug),
            document(
                Section::Other,
                &detailed.title,
                &game_page(&detailed, "name"),
                Some(&path),
            )
            .0,
        ));
    }

    let mut outlets = db.list_outlets().map_err(|e| e.to_string())?;
    sort_outlets(&mut outlets, "reviews");
    pages.push((
        "outlets/index.html".into(),
        document(
            Section::Outlets,
            "Outlets",
            &outlet_list(&outlets, "reviews"),
            Some("/outlets"),
        )
        .0,
    ));

    for outlet in &outlets {
        let mut detailed = db
            .get_outlet(&outlet.slug)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing outlet {}", outlet.slug))?;
        sort_outlet_reviews(&mut detailed.reviews, "name", detailed.mu_global);
        let path = format!("/outlet/{}", detailed.slug);
        pages.push((
            format!("outlet/{}/index.html", detailed.slug),
            document(
                Section::Outlets,
                &detailed.name,
                &outlet_page(&detailed, "name"),
                Some(&path),
            )
            .0,
        ));
    }

    pages.push((
        "score/index.html".into(),
        document(Section::Score, "The score", &method_page(), Some("/score")).0,
    ));
    pages.push((
        "about/index.html".into(),
        document(Section::About, "About", &about_page(), Some("/about")).0,
    ));
    pages.push((
        "404.html".into(),
        document(
            Section::Other,
            "Not found",
            "<h1 class=\"page\">Page not found.</h1><p class=\"lead\"><a href=\"/\">Back to games</a></p>",
            Some("/"),
        )
        .0,
    ));

    Ok(pages)
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

fn fmt_num(value: f64) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    format!("{rounded:.4}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn score_compact_html(value: f64) -> String {
    let full = fmt_num(value);
    let (whole, frac) = match full.split_once('.') {
        Some((whole, frac)) => (whole.to_string(), format!(".{frac}")),
        None => (full, String::new()),
    };
    if frac.is_empty() {
        return format!("<span class=\"score\">{whole}</span>", whole = esc(&whole));
    }
    format!(
        "<span class=\"score score-compact\" title=\"{full}\">\
           <span class=\"whole\">{whole}</span><span class=\"frac\">{frac}</span>\
         </span>",
        full = esc(&format!("{whole}{frac}")),
        whole = esc(&whole),
        frac = esc(&frac),
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_titles() {
        assert_eq!(slugify("Hades II"), "hades-ii");
        assert_eq!(slugify("Ação"), "acao");
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
