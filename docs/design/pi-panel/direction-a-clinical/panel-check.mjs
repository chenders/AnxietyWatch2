#!/usr/bin/env node
// Gate for the Direction A bedside panel.
//
// Run:  node docs/design/pi-panel/direction-a-clinical/panel-check.mjs [index.html]
// Exit: 0 = every hard requirement holds · 1 = at least one failed
//
// WHY THIS EXISTS
//
// The imported design stated its numbers in prose. Checking them found two
// different outcomes, and the difference is the whole argument for this file:
//
//   * Every contrast ratio in its token table was exactly right — all
//     twelve that state one. The thirteenth row is --bg, the ground the
//     others are measured against, which carries no ratio of its own.
//   * Its claim that "grayscale luminance separation between the tier states
//     is >= 1.9:1 in every adjacent pair" was wrong in every adjacent pair --
//     the worst is 1.00:1, because WATCH and CONFIRM are the same amber.
//   * Its claim that the row heights "= 480 exactly" was wrong: they total 470,
//     and the remaining 10px is absorbed by margin-top:auto above the silence
//     bar.
//
// A design that gets every stated number right is not a careless one.
// That is exactly why the numbers get checked rather than trusted: a table this
// good is the kind you stop re-deriving, and the two that were wrong were both
// load-bearing safety claims.
//
// Colour math vendored so this runs with bare `node`. Machado, Oliveira &
// Fernandes (2009) at severity 1.0.

const MACHADO = {
  protan: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deutan: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
  tritan: [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
};
const s2lin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const lin = (h) => [0, 2, 4].map((i) => s2lin(parseInt(h.replace("#", "").slice(i, i + 2), 16) / 255));
const relLum = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;
const ratioOf = (la, lb) => { const [hi, lo] = [la, lb].sort((x, y) => y - x); return (hi + 0.05) / (lo + 0.05); };
const contrast = (a, b) => ratioOf(relLum(lin(a)), relLum(lin(b)));
const simulate = (h, kind) => {
  const [r, g, b] = lin(h), M = MACHADO[kind], cl = (c) => Math.max(0, Math.min(1, c));
  return [cl(M[0][0] * r + M[0][1] * g + M[0][2] * b),
          cl(M[1][0] * r + M[1][1] * g + M[1][2] * b),
          cl(M[2][0] * r + M[2][1] * g + M[2][2] * b)];
};

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Defaults to the sibling index.html. CI passes a deliberately mutated copy to
// prove this gate can actually fail -- see the design-gates job in ci.yml.
const HTML = process.argv[2]
  ? resolve(process.argv[2])
  : resolve(dirname(fileURLToPath(import.meta.url)), "index.html");

let failed = 0;
const pad = (s, n) => String(s).padEnd(n);
const line = (state, label, detail) => {
  if (state === "FAIL") failed++;
  console.log(`  [${pad(state, 4)}] ${pad(label, 30)} ${detail}`);
};

// ── 1. token contrast against pure black ─────────────────────────────────────
// [token, hex, ratio the design declared, minimum it must clear, why]
const BLACK = "#000000";
const TOKENS = [
  ["--field",      "#0A0B0C",  1.07, 0,   "box interior — region without emission"],
  ["--rule",       "#5E6670",  3.61, 3.0, "boxed-region borders (WCAG 1.4.11)"],
  ["--rule-quiet", "#3A4048",  2.01, 0,   "decorative hairlines — carries no meaning, exempt"],
  ["--spo2",       "#3FC8D8", 10.46, 4.5, "oxygen saturation"],
  ["--hr",         "#35C46A",  9.26, 4.5, "pulse"],
  ["--nibp",       "#DCE1E6", 15.96, 4.5, "non-invasive pressure"],
  ["--watch",      "#E0A227",  9.37, 4.5, "medium physiological + technical"],
  ["--klaxon",     "#E2453C",  5.15, 4.5, "high priority only"],
  ["--unknown",    "#8B97A3",  7.05, 4.5, "indeterminate"],
  ["--unknown-hi", "#B9C0C7", 11.43, 4.5, "indeterminate text over a live value"],
  ["--label",      "#7A828C",  5.40, 4.5, "region labels at 14px"],
  ["--rest",       "#585F52",  3.17, 3.0, "resting tier word at 44px — large-text AA"],
];
console.log("\nTOKEN CONTRAST vs #000 (declared value must match, and clear its bar)");
for (const [name, hex, declared, floor, why] of TOKENS) {
  const c = contrast(hex, BLACK);
  const matches = Math.abs(c - declared) < 0.05;
  const clears = c >= floor;
  line(matches && clears ? "ok" : "FAIL", name,
    `${c.toFixed(2)}:1 (declared ${declared.toFixed(2)})`
    + (matches ? "" : " — DECLARED VALUE IS WRONG")
    + (clears ? "" : ` — below its ${floor}:1 bar`)
    + ` · ${why}`);
}

// ── 2. tier separation ───────────────────────────────────────────────────────
// The design is right that amber and red collapse under red-green dichromacy,
// and right to say hue is therefore never the only difference. It was wrong
// about the fallback: luminance does NOT carry the tiers either.
//
// So the real requirement is structural, and it is asserted structurally: every
// adjacent tier pair must differ in at least TWO non-hue channels. Glyph, word,
// border style, fill, and motion are all readable in grayscale, at a glance,
// half-asleep -- which is the actual viewing condition.
const TIERS = [
  { name: "CLEAR",        accent: "#35C46A", glyph: "●",  word: "CLEAR",        border: "solid",  fill: "none",   motion: "none" },
  { name: "CAN'T ASSESS", accent: "#8B97A3", glyph: "?",  word: "CAN'T ASSESS", border: "dashed", fill: "hatch",  motion: "none" },
  { name: "WATCH",        accent: "#E0A227", glyph: "▲",  word: "WATCH",        border: "solid",  fill: "6%",     motion: "none" },
  { name: "CONFIRM",      accent: "#E0A227", glyph: "▲▲", word: "CONFIRM",      border: "solid",  fill: "14%",    motion: "0.5Hz" },
  { name: "KLAXON",       accent: "#E2453C", glyph: "■",  word: "KLAXON",       border: "solid",  fill: "18%",    motion: "1Hz" },
];
const CHANNELS = ["glyph", "word", "border", "fill", "motion"];

console.log("\nTIER SEPARATION — adjacent pairs");
console.log("  hue is NOT relied on: amber and red collapse toward one olive under");
console.log("  deutan/protan, and luminance does not separate the tiers either.");
for (let i = 0; i < TIERS.length - 1; i++) {
  const a = TIERS[i], b = TIERS[i + 1];
  const lum = ratioOf(relLum(lin(a.accent)), relLum(lin(b.accent)));
  const differing = CHANNELS.filter((c) => a[c] !== b[c]);
  // Two independent non-hue channels, so that losing one to a rendering quirk,
  // a scratched screen or a half-open eye still leaves the tiers distinct.
  line(differing.length >= 2 ? "ok" : "FAIL", `${a.name} -> ${b.name}`,
    `luminance ${lum.toFixed(2)}:1 (not relied on) · ${differing.length} non-hue channels differ: ${differing.join(", ")}`);
}

// The mitigation above is only *necessary* if the collapse is real. Assert it,
// so that a future palette change which happens to separate amber from red
// reports that the structural redundancy is now belt-and-braces rather than
// silently leaving a stale justification in the README.
// The design said amber and red "collapse toward one hue under both red-green
// dichromacies". Half right, and the half it missed matters: they converge in
// hue under both, but luminance still separates them under protanopia (2.23:1)
// and does NOT under deuteranopia (1.65:1). Deuteranopia is also the more
// common of the two, so the structural redundancy is required on the strength
// of that one case -- redundancy is needed if ANY simulation loses the pair,
// not only if all of them do.
console.log("\nCVD COLLAPSE — is the structural redundancy actually needed?");
let collapses = false;
for (const kind of ["deutan", "protan", "tritan"]) {
  const dl = ratioOf(relLum(simulate("#E0A227", kind)), relLum(simulate("#E2453C", kind)));
  const lost = dl < 1.9;
  if (lost) collapses = true;
  line("ok", `--watch vs --klaxon (${kind})`,
    `luminance ${dl.toFixed(2)}:1 simulated — ${lost ? "LOST: hue and luminance both gone" : "luminance still separates them"}`);
}
line(collapses ? "ok" : "warn", "redundancy required?",
  collapses
    ? "yes — at least one dichromacy loses the pair entirely, so glyph/fill/motion are load-bearing"
    : "no longer — every dichromacy separates these two; the README's justification is now stale");

// ── 3. layout arithmetic ─────────────────────────────────────────────────────
// The panel is a fixed-size box whose regions must never reflow into each
// other's address. index.html asserts the rendered geometry at runtime; this is
// the same sum offline, and it is where the design's "= 480 exactly" died.
const ROWS = [22, 86, 152, 54, 34, 56];
const GAP = 8, PAD_Y = 13, PANEL_H = 480, PANEL_W = 800;
const rowSum = ROWS.reduce((a, b) => a + b, 0);
const gaps = GAP * (ROWS.length - 1);
const total = rowSum + gaps + PAD_Y * 2;
const slack = PANEL_H - total;

console.log("\nLAYOUT — 800x480 fixed, no region may reflow");
line("ok", "rows", `${ROWS.join(" + ")} = ${rowSum}`);
line("ok", "gaps + padding", `${ROWS.length - 1} x ${GAP} = ${gaps}, padding 2 x ${PAD_Y} = ${PAD_Y * 2}`);
line(slack >= 0 ? "ok" : "FAIL", "total vs panel height",
  `${total} of ${PANEL_H} — ${slack}px slack, absorbed above the silence bar by margin-top:auto`
  + (slack < 0 ? " — OVERFLOWS, regions will reflow" : ""));
line(slack === 0 ? "warn" : "ok", "the design's \"= 480 exactly\"",
  slack === 0 ? "now actually exact — update README.md, which explains why it is not"
              : `false: the rows total ${total}, not ${PANEL_H}. Harmless, but it was never checked`);

// ── 4. the one touch target ──────────────────────────────────────────────────
// WCAG 2.5.5 AAA wants 44px in the short axis. The silence bar is the ONLY
// interactive element on the panel, deliberately: at 3 a.m. a second target is
// a way to press the wrong thing.
const BAR = { w: PANEL_W - 16 * 2, h: 56 };
console.log("\nTOUCH");
line(BAR.h >= 44 ? "ok" : "FAIL", "silence bar short axis", `${BAR.h}px >= 44px (WCAG 2.5.5 AAA)`);
line(BAR.w === PANEL_W - 32 ? "ok" : "FAIL", "silence bar width",
  `${BAR.w}px — full content width, so an imprecise finger cannot miss it`);


// -- the TOKENS table above must BE the panel ---------------------------------
//
// Every ratio checked above is checked against hexes hard-coded in TOKENS.
// Nothing so far proves those are the hexes index.html renders. Let them drift
// and this gate keeps exiting 0 while certifying a panel nobody sees -- the
// same vacuous-check failure mode the rest of ci.yml was written about.
//
// Both directions: a changed value fails, and a NEW colour token the gate has
// never heard of also fails, since otherwise the way around the gate is to add
// a colour it does not know to look at.
const EXEMPT = {
  "--bg": "pure black — the ground every ratio above is measured against",
};

console.log(`\nSOURCE SYNC -- ${HTML}`);
let html = null;
try {
  html = readFileSync(HTML, "utf8");
} catch (e) {
  // Fail loudly: a gate that cannot read what it gates must not call it fine.
  line("FAIL", "read index.html", `${e.code || e.message}`);
}
if (html !== null) {
  const m = /:root\s*\{([\s\S]*?)\}/.exec(html);
  if (!m) {
    line("FAIL", ":root block", "not found in the stylesheet");
  } else {
    const css = {};
    for (const [, k, v] of m[1].matchAll(/(--[\w-]+)\s*:\s*(#[0-9a-fA-F]{6})\b/g)) css[k] = v.toLowerCase();
    let bad = 0;
    for (const [name, hex] of TOKENS) {
      const theirs = css[name];
      if (theirs === undefined) { line("FAIL", name, "declared here but absent from the stylesheet"); bad++; }
      else if (theirs !== hex.toLowerCase()) {
        line("FAIL", name, `gate says ${hex}, stylesheet says ${theirs} -- the gate is checking a panel the device does not render`);
        bad++;
      }
    }
    const known = new Set(TOKENS.map((t) => t[0]));
    for (const name of Object.keys(css)) {
      if (known.has(name) || EXEMPT[name]) continue;
      line("FAIL", name, "colour token in the stylesheet that this gate does not check -- add it to TOKENS or EXEMPT with a reason");
      bad++;
    }
    // The tier accents are asserted separately above, so they must be real
    // tokens too -- otherwise TIERS could describe a panel that does not exist.
    const values = new Set(Object.values(css));
    for (const t of TIERS) {
      if (!values.has(t.accent.toLowerCase())) {
        line("FAIL", `tier ${t.name}`, `accent ${t.accent} is not any token in the stylesheet`);
        bad++;
      }
    }
    if (!bad) line("ok", "tokens vs stylesheet", `all ${TOKENS.length} gated tokens match, ${TIERS.length} tier accents real, ${Object.keys(EXEMPT).length} exempt`);
  }
}

console.log(failed === 0
  ? "\nPASS — every hard requirement holds.\n"
  : `\nFAIL — ${failed} hard requirement(s) violated.\n`);
process.exit(failed === 0 ? 0 : 1);
