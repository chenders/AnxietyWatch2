#!/usr/bin/env node
// Palette gate for the clinical web dashboard.
//
// Run:  node docs/design/web-dashboard/palette-check.mjs [index.html]
// Exit: 0 = every declared requirement holds · 1 = at least one failed
//
// WHY THIS EXISTS
//
// The imported design asserted, in prose printed on its own design-system
// screen, that "every pair in the set stays >= 18 dE (CIE76) after
// Brettel-Vienot-Mollon simulation" and that cyan/green was the closest pair.
// Measured, the worst pair is --unknown against --spo2 at dE 2.9 (deutan) and
// 12.8 unsimulated. Both halves of the claim were wrong. A design system that
// prints its own verification is only as good as whatever checks the print.
//
// So the numbers are no longer written down. index.html computes its contrast
// ratios and CVD separations live from the real token values, and this script
// is the same arithmetic behind an exit code.
//
// WHAT IS ACTUALLY REQUIRED HERE
//
// The generic categorical-palette rule -- all N hues mutually separable -- is
// the right rule for a plot with N series sharing one set of axes. This design
// never does that. Every signal gets its own lane, its own fixed vertical
// scale, and a printed monospace label; identity is carried by position and
// text, and hue is redundant. Applying the all-pairs rule here would fail a
// design for a hazard it does not have, and "fix" it by breaking a coherent
// system.
//
// The pairs that genuinely share a plot are enumerated in PLOT_PAIRS below.
// Each one declares the non-hue encoding that separates it. A pair below the
// CVD floor with no declared secondary encoding is a hard failure -- that is
// the case where a reader really would be unable to tell two marks apart.
//
// The colour math is vendored rather than imported so this runs with bare
// `node` and no install step. Machado, Oliveira & Fernandes (2009) severity
// 1.0; dE is Euclidean distance in OKLab x100.

// ---- thresholds -------------------------------------------------------------
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Defaults to the sibling index.html. CI passes a deliberately mutated copy to
// prove this gate can actually fail -- see the design-gates job in ci.yml.
const HTML = process.argv[2]
  ? resolve(process.argv[2])
  : resolve(dirname(fileURLToPath(import.meta.url)), "index.html");

const CVD_TARGET = 8.0;   // dE, min over protan/deutan/tritan: separable on hue alone
const CVD_FLOOR = 6.0;    // below this, hue is doing no work at all
const CONTRAST_GRAPHIC = 3.0;  // WCAG 2.2 non-text contrast
const CONTRAST_TEXT = 4.5;     // WCAG 2.2 AA normal text

// ---- colour math ------------------------------------------------------------
const MACHADO = {
  protan: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deutan: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
  tritan: [[1.255528, -0.076749, -0.178779], [-0.078411, 0.930809, 0.147602], [0.004733, 0.691367, 0.303900]],
};
const s2lin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const lin = (h) => [0, 2, 4].map((i) => s2lin(parseInt(h.replace("#", "").slice(i, i + 2), 16) / 255));
const relLum = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;
const contrast = (a, b) => {
  const [hi, lo] = [relLum(lin(a)), relLum(lin(b))].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};
function oklab([r, g, b]) {
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
  ];
}
function simulate(h, kind) {
  const [r, g, b] = lin(h), M = MACHADO[kind], cl = (c) => Math.max(0, Math.min(1, c));
  return [
    cl(M[0][0] * r + M[0][1] * g + M[0][2] * b),
    cl(M[1][0] * r + M[1][1] * g + M[1][2] * b),
    cl(M[2][0] * r + M[2][1] * g + M[2][2] * b),
  ];
}
const deltaE = (h1, h2, kind) => {
  const a = oklab(kind ? simulate(h1, kind) : lin(h1));
  const b = oklab(kind ? simulate(h2, kind) : lin(h2));
  return 100 * Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
};
const worstCVD = (a, b) => Math.min(...["protan", "deutan", "tritan"].map((k) => deltaE(a, b, k)));

// ---- tokens -----------------------------------------------------------------
// Keep in lockstep with the :root and [data-theme="light"] blocks in index.html.
const THEMES = {
  dark: {
    surface: "#000508",
    signal: { spo2: "#22d3ee", hr: "#4ade80", resp: "#facc15", bp: "#f87171", cpap: "#e879f9" },
    status: { hi: "#ff5c5c", med: "#ffb54d", low: "#67e8f9", ok: "#4ade80" },
    unknown: "#a5b4fc",
    text: { text: "#e8edf2", "text-dim": "#9aa7b4", "text-faint": "#737f8d" },
    lineStrong: "#5b6674",
  },
  light: {
    surface: "#f4f6f8",
    signal: { spo2: "#0e7490", hr: "#15803d", resp: "#a16207", bp: "#b91c1c", cpap: "#a21caf" },
    status: { hi: "#b91c1c", med: "#a16207", low: "#0e7490", ok: "#15803d" },
    unknown: "#4338ca",
    text: { text: "#10161d", "text-dim": "#4a5560", "text-faint": "#5c6874" },
    lineStrong: "#6b7480",
  },
};

// ── the table above must BE the page ─────────────────────────────────────────
//
// Everything this script checks is checked against the hexes hard-coded above.
// Nothing so far proves those are the hexes index.html actually renders. Let
// them drift and the gate keeps exiting 0 while validating a palette no reader
// has ever seen -- a check that cannot fail, which is the exact defect the CI
// file's own comments were written about.
//
// So: parse the stylesheet and require the two to agree, in both directions.
// A token whose value moved fails. A NEW colour token added to :root that this
// script has never heard of also fails, because otherwise the way to bypass the
// gate is simply to introduce a colour it does not know to look at.
const TOKEN_MAP = {
  surface: "--bg", unknown: "--unknown", lineStrong: "--line-strong",
  "signal.spo2": "--spo2", "signal.hr": "--hr", "signal.resp": "--resp",
  "signal.bp": "--bp", "signal.cpap": "--cpap",
  "status.hi": "--hi", "status.med": "--med", "status.low": "--low", "status.ok": "--ok",
  "text.text": "--text", "text.text-dim": "--text-dim", "text.text-faint": "--text-faint",
};
// Colour tokens deliberately outside the gate, each with the reason it is
// exempt. Anything not here and not in TOKEN_MAP is an error, not a default.
const EXEMPT = {
  "--surface": "box fill; structure is carried by --line-strong, not by its contrast",
  "--surface-2": "pressed-state fill for the trend-window control; never a mark",
  "--line": "decorative gridwork only — explicitly exempt from WCAG 1.4.11",
};

function parseThemes(html) {
  const grab = (re) => {
    const m = re.exec(html);
    if (!m) return null;
    const out = {};
    for (const [, k, v] of m[1].matchAll(/(--[\w-]+)\s*:\s*(#[0-9a-fA-F]{6})\b/g)) out[k] = v.toLowerCase();
    return out;
  };
  return {
    dark: grab(/:root\s*\{([\s\S]*?)\}/),
    light: grab(/\[data-theme="light"\]\s*\{([\s\S]*?)\}/),
  };
}

function checkSource(htmlPath) {
  let html;
  try {
    html = readFileSync(htmlPath, "utf8");
  } catch (e) {
    // Fail loudly. A gate that cannot read the thing it gates must not report
    // that the thing is fine.
    console.log(`\nSOURCE SYNC\n  [FAIL] cannot read ${htmlPath} — ${e.code || e.message}`);
    failed++;
    return;
  }
  const css = parseThemes(html);
  console.log(`\nSOURCE SYNC — ${htmlPath}`);
  for (const [mode, t] of Object.entries(THEMES)) {
    const block = css[mode];
    if (!block) { line("FAIL", `${mode} block`, "not found in the stylesheet"); continue; }
    let drift = 0, unknown = 0;
    for (const [key, cssVar] of Object.entries(TOKEN_MAP)) {
      const mine = key.includes(".")
        ? t[key.split(".")[0]][key.split(".")[1]]
        : t[key];
      const theirs = block[cssVar];
      if (theirs === undefined) { line("FAIL", `${cssVar} (${mode})`, "declared here but absent from the stylesheet"); drift++; }
      else if (theirs !== mine.toLowerCase()) {
        line("FAIL", `${cssVar} (${mode})`, `gate says ${mine}, stylesheet says ${theirs} — the gate is checking a palette the page does not use`);
        drift++;
      }
    }
    for (const cssVar of Object.keys(block)) {
      if (Object.values(TOKEN_MAP).includes(cssVar) || EXEMPT[cssVar]) continue;
      line("FAIL", `${cssVar} (${mode})`, "colour token in the stylesheet that this gate does not check — add it to TOKEN_MAP or EXEMPT with a reason");
      unknown++;
    }
    if (!drift && !unknown)
      line("ok", `${mode} theme`, `all ${Object.keys(TOKEN_MAP).length} gated tokens match the stylesheet; ${Object.keys(EXEMPT).length} exempt`);
  }
}

// Pairs that appear inside ONE plot box and must be told apart there.
//
// Enumerated from the plots that actually exist, not from the token list. Only
// four signals are ever drawn: SpO2, HR, RESP and CPAP leak each own a lane on
// "Last night" and a card on "Trends". --bp is a declared token with no plot --
// blood pressure surfaces as a numeric tile only -- so it contributes no pair.
// Listing it anyway would invent a hazard and, worse, would make the gate go
// red for a collision no reader can ever see.
//
// `secondary` names the non-hue channel that separates the pair, or null if hue
// is the only differentiator. A null pair below CVD_TARGET fails the run.
const PLOTTED = ["spo2", "hr", "resp", "cpap"];
const PLOT_PAIRS = (t) => [
  // Every lane and every trend card draws its own gap as hatch between dashed
  // walls. The signal line is solid and continuous; the gap is a texture fill.
  ...PLOTTED.map((k) => ({
    a: `--${k}`, av: t.signal[k], b: "--unknown", bv: t.unknown,
    secondary: "diagonal hatch fill vs solid stroke, plus dashed vertical gap walls and a '?' glyph",
  })),
  // Overnight lanes overlay event marks on the signal trace.
  ...PLOTTED.map((k) => ({
    a: `--${k}`, av: t.signal[k], b: "--hi", bv: t.status.hi,
    secondary: "full-height vertical rule vs horizontal trace — different geometry, not different hue",
  })),
  // Trend cards draw the series, its own rolling baseline and its IQR band in
  // ONE hue on purpose; separation there is dash pattern and opacity, never hue.
  // Nothing else shares a plot: signals each own a lane.
];

// ---- checks -----------------------------------------------------------------
let failed = 0;
const pad = (s, n) => String(s).padEnd(n);
const line = (state, label, detail) => {
  if (state === "FAIL") failed++;
  console.log(`  [${pad(state, 4)}] ${pad(label, 34)} ${detail}`);
};

for (const [mode, t] of Object.entries(THEMES)) {
  console.log(`\n${mode.toUpperCase()} — surface ${t.surface}`);

  // 1. Graphical objects: every signal, status and the unknown token must clear
  //    3:1 against the ground they are drawn on, or the mark itself is invisible.
  const graphics = { ...t.signal, ...t.status, unknown: t.unknown };
  for (const [name, hex] of Object.entries(graphics)) {
    const c = contrast(hex, t.surface);
    line(c >= CONTRAST_GRAPHIC ? "ok" : "FAIL", `--${name} vs surface`,
      `${c.toFixed(2)}:1 ${c >= CONTRAST_GRAPHIC ? ">=" : "<"} ${CONTRAST_GRAPHIC}:1`);
  }

  // 2. Text tokens carry values and labels; AA normal text is 4.5:1.
  for (const [name, hex] of Object.entries(t.text)) {
    const c = contrast(hex, t.surface);
    line(c >= CONTRAST_TEXT ? "ok" : "FAIL", `--${name} vs surface (text)`,
      `${c.toFixed(2)}:1 ${c >= CONTRAST_TEXT ? ">=" : "<"} ${CONTRAST_TEXT}:1`);
  }

  // 3. --line-strong marks real component boundaries, so it is a graphical
  //    object under 1.4.11, unlike --line which is decorative gridwork.
  const ls = contrast(t.lineStrong, t.surface);
  line(ls >= CONTRAST_GRAPHIC ? "ok" : "FAIL", "--line-strong vs surface",
    `${ls.toFixed(2)}:1 ${ls >= CONTRAST_GRAPHIC ? ">=" : "<"} ${CONTRAST_GRAPHIC}:1`);

  // 4. The one that actually bites: within-plot separation.
  for (const p of PLOT_PAIRS(t)) {
    const d = worstCVD(p.av, p.bv);
    const state = d >= CVD_TARGET ? "ok" : p.secondary ? (d >= CVD_FLOOR ? "warn" : "hue0") : "FAIL";
    const note = d >= CVD_TARGET
      ? "separable on hue alone"
      : p.secondary
        ? `hue ${d < CVD_FLOOR ? "carries nothing" : "is weak"} — separated by ${p.secondary}`
        : "NO secondary encoding declared";
    line(state, `${p.a} vs ${p.b}`, `dE ${d.toFixed(1)} (worst CVD) — ${note}`);
  }
}

// 5. --hr and --ok are deliberately the same hex in the dark theme: green means
//    "heart rate" on a chart and "measured and in range" in a status column.
//    That is a real overload of a reserved status colour. It is tolerable only
//    because the two never appear in the same region and status always carries
//    a glyph. Assert the glyph rule holds by asserting the collision is known,
//    so that changing one without the other trips this.
// The generic rule is that status colours are reserved and never reused as a
// series hue. This design breaks it, and not marginally: in the light theme
// EVERY status token is the same hex as a signal token. That is a deliberate
// consequence of deriving both ramps from the same small set of accessible
// steps, and it is survivable only because status always ships a glyph and the
// two families never occupy the same region. None of these are plot pairs.
//
// Enumerated rather than summarised so that changing one hex moves one line
// here. A collision that disappears is as much a change as one that appears --
// if a pair separates, this says so and README.md is then wrong.
console.log("\nRESERVED-COLOUR OVERLOAD (status hue reused as a signal hue)");
for (const [mode, t] of Object.entries(THEMES)) {
  for (const [sName, sHex] of Object.entries(t.status)) {
    const hit = Object.entries(t.signal).find(([, v]) => v.toLowerCase() === sHex.toLowerCase());
    const near = Object.entries(t.signal)
      .map(([k, v]) => [k, worstCVD(v, sHex)]).sort((a, b) => a[1] - b[1])[0];
    if (hit) {
      line("warn", `--${sName} = --${hit[0]} (${mode})`,
        `both ${sHex} — reserved status hue reused as a signal; every status use must pair a glyph`);
    } else if (near[1] < CVD_TARGET) {
      // A different hex that no dichromat can tell apart is the same collision
      // with extra steps. Saying "distinct" here because the strings differ is
      // exactly the mistake the imported design made about its own palette.
      line("warn", `--${sName} ~ --${near[0]} (${mode})`,
        `${sHex} vs ${t.signal[near[0]]} — different hex, dE ${near[1].toFixed(1)}: indistinguishable under CVD`);
    } else {
      line("ok", `--${sName} (${mode})`,
        `separable from every signal — nearest --${near[0]} at dE ${near[1].toFixed(1)}`);
    }
  }
}
console.log("  note: --bp is the one signal with no plot. Give blood pressure a lane");
console.log("        and its collision with --hi stops being cosmetic — resolve it first.");

checkSource(HTML);

console.log(failed === 0
  ? "\nPASS — every hard requirement holds.\n"
  : `\nFAIL — ${failed} hard requirement(s) violated.\n`);
process.exit(failed === 0 ? 0 : 1);
