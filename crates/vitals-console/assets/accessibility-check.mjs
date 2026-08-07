#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { runM1RegressionChecks } from "./m1-regression-check.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const read = name => readFileSync(join(root, name), "utf8");
const html = read("index.html");
const base = read("base.css");
const css = [base, read("dir-a.css"), read("dir-b.css"), read("dir-c.css")].join("\n");
const renderers = [read("dir-a.js"), read("dir-b.js"), read("dir-c.js")];

function luminance(hex) {
  const value = Number.parseInt(hex.slice(1), 16);
  const channels = [(value >> 16) & 255, (value >> 8) & 255, value & 255].map(value => {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}
function contrast(foreground, background) {
  const a = luminance(foreground);
  const b = luminance(background);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

// Explicitly enumerated shipped foreground/background and meaningful
// boundary/focus contexts. Text requires 4.5:1; component boundaries 3:1.
const pairs = [
  ["base text/body", "#f2f5f7", "#10151b", 4.5], ["header nav/header", "#d9e3eb", "#151c24", 4.5],
  ["brand/header", "#ffffff", "#151c24", 4.5], ["brand muted/header", "#aab8c5", "#151c24", 4.5],
  ["nav selected/selected fill", "#ffffff", "#2b3947", 4.5], ["control/control fill", "#ffffff", "#202b36", 4.5],
  ["eyebrow/body", "#b7c7d5", "#10151b", 4.5], ["card text/card", "#d1dbe3", "#192129", 4.5],
  ["card detail/card", "#9fb0bf", "#192129", 4.5], ["card timing/card", "#ffffff", "#192129", 4.5],
  ["live label/card", "#7ee4ad", "#192129", 4.5], ["failing label/card", "#ff9e8a", "#192129", 4.5],
  ["unknown label/card", "#ffda85", "#192129", 4.5], ["not-configured label/card", "#c4d0da", "#192129", 4.5],
  ["footer/footer", "#bdcad5", "#151c24", 4.5], ["overlay/overlay", "#fff6df", "#4d180d", 4.5],
  ["overlay button/button fill", "#351006", "#fff6df", 4.5], ["card boundary/card", "#68798a", "#192129", 3],
  ["control boundary/control fill", "#718296", "#202b36", 3], ["overlay boundary/overlay", "#ffbc58", "#4d180d", 3],
  ["dark focus/body", "#ffe08a", "#10151b", 3], ["dark focus/header", "#ffe08a", "#151c24", 3],
  ["dark focus/control", "#ffe08a", "#202b36", 3], ["dark focus/card", "#ffe08a", "#192129", 3],
  ["A accent/A body", "#6fd3ff", "#0d141a", 3],
  ["B body/B body", "#172029", "#f3f0e8", 4.5], ["B header/B header", "#edf3f5", "#172029", 4.5],
  ["B eyebrow/B body", "#52616d", "#f3f0e8", 4.5], ["B row text/white", "#394752", "#ffffff", 4.5],
  ["B number/white", "#65737d", "#ffffff", 4.5], ["B live/white", "#0c613c", "#ffffff", 4.5],
  ["B failing/white", "#9e2817", "#ffffff", 4.5], ["B unknown/white", "#76500b", "#ffffff", 4.5],
  ["B not-configured/white", "#46545e", "#ffffff", 4.5], ["B main focus/body", "#005fcc", "#f3f0e8", 3], ["B main focus/white", "#005fcc", "#ffffff", 3],
  ["B header focus/header", "#ffe08a", "#172029", 3], ["B header focus/control", "#ffe08a", "#202b36", 3],
  ["C text/C body", "#e8fffa", "#091817", 4.5], ["C cyan/C body", "#6ce2d1", "#091817", 4.5],
  ["C heading/C body", "#9bc7c0", "#091817", 4.5], ["C text/C card", "#c4ddd8", "#102522", 4.5],
  ["C live/C card", "#70eaae", "#102522", 4.5], ["C failing/C card", "#ff9a87", "#102522", 4.5],
  ["C unknown/C card", "#ffd57b", "#102522", 4.5], ["C not-configured/C card", "#b8cbc7", "#102522", 4.5],
  ["C card boundary/C card", "#4b7b74", "#102522", 3], ["C divider/C card", "#4b7b74", "#102522", 3],
];
for (const [name, foreground, background, minimum] of pairs) {
  const ratio = contrast(foreground, background);
  console.log(`${name}: ${foreground} on ${background} = ${ratio.toFixed(2)}:1`);
  assert.ok(ratio >= minimum, `${name} contrast ${ratio.toFixed(2)}:1 is below ${minimum}:1`);
}

assert.match(html, /<html lang="en">/);
assert.match(html, /<label for="layout-direction">/);
assert.match(html, /<select id="layout-direction"/);
assert.match(html, /<button type="submit">Apply<\/button>/);
assert.match(html, /<button id="reconnect" type="button">/);
assert.match(html, /<nav aria-label="Console screens">/);
assert.match(html, /aria-live="assertive"/);
assert.doesNotMatch(html + renderers.join("\n"), /<(div|span)[^>]+(?:onclick|role="button")/i);
for (const renderer of renderers) {
  assert.match(renderer, /<article class=/);
  assert.match(renderer, /aria-hidden="true"[^>]*>\$\{h\.glyph/);
  assert.match(renderer, /\$\{h\.esc\(item\.state\)\}/);
}
// Hue-independent state encoding: visible text + distinct glyphs, and
// NOT_CONFIGURED uses a dashed boundary while health states use solid ones.
const model = read("console-model.js");
assert.match(read("console.js"), /LIVE: "●", FAILING: "▲", UNKNOWN: "\?", NOT_CONFIGURED: "—"/);
assert.match(model, /NOT_CONFIGURED/);
assert.match(css, /\.state-not_configured\{[^}]*border-left-style:dashed/);
assert.match(read("dir-c.css"), /\.state-not_configured\{[^}]*border-top-style:dashed/);

for (const match of css.matchAll(/font-size\s*:\s*([^;}]+)/g)) {
  for (const size of match[1].matchAll(/([0-9]*\.?[0-9]+)(px|rem)/g)) {
    const pixels = Number(size[1]) * (size[2] === "rem" ? 16 : 1);
    assert.ok(pixels >= 14, `font-size ${size[0]} computes to ${pixels}px, below 14px`);
  }
}
assert.match(css, /font-variant-numeric:tabular-nums/);
assert.match(base, /:focus-visible\{outline:3px solid #ffe08a;outline-offset:3px\}/);
assert.match(base, /body\[data-dir=b\] main[^}]+outline-color:#005fcc/);
assert.match(base, /@media\(prefers-reduced-motion:reduce\)/);
assert.match(base, /animation-duration:\.01ms!important/);
assert.match(base, /transition-duration:\.01ms!important/);

console.log(`Accessibility checks passed: ${pairs.length} enumerated shipped token contexts; semantic labels, hue-independent glyph/text/boundary cues, 14px floor, per-theme focus, tabular numerics, and reduced motion verified.`);
runM1RegressionChecks();
