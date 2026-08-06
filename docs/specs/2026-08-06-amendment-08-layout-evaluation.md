# Amendment 08 — Layout selection by live evaluation, not upfront choice

**Date:** 2026-08-06
**Status:** accepted (operator decision)
**Supersedes:** the implicit single-direction assumption in [`../design/SPEC-admin-console.md`](../design/SPEC-admin-console.md)
§3.1 — that one of `PROMPT-admin.md`'s three visual directions is chosen before M1 is built.
**Affects:** admin console M1 (+ one new post-evaluation task), the web dashboard's plan position,
the M1 task brief when it is written. M0 is untouched — it has no UI.

---

## 1. The decision

**No design direction is chosen from mockups.** Every candidate layout ships behind a runtime
"change layout" switcher and is evaluated *in operation, on real data*. The final determination is
made after living with them; a pruning task then deletes the losers. The wasted code this implies
is accepted explicitly — it is the price of not committing to a layout that was only ever seen
with demo data.

**Precedent.** The bedside panel already did exactly this: PR #12/#15 shipped all three panel
directions (Clinical / Penumbra / Semaphore) plus five experimental views as swipeable live views
over `status.json`, selected by swipe / `?view=` / localStorage. That mechanism is what makes a
real evaluation possible at all, and it is the model for both surfaces below.

## 2. Admin console — M1 renders all three directions

The three complete direction mockups live in
[`../design/admin-console/claude-design/`](../design/admin-console/claude-design/)
(`a/` Service Mode · `b/` Preflight · `c/` Flowpath — 15 HTML pages + one stylesheet + one script
each, vanilla, no dependencies). M1 changes from "build the console" to:

1. **One view-model per screen, three renderings.** Handlers produce direction-agnostic view-model
   structs; each direction is a template set plus its `console.css`. **No direction-specific data
   logic anywhere** — this constraint is what keeps the waste cheap and the eventual pruning a
   file deletion rather than a refactor.
2. **Switcher contract:** `?dir=a|b|c` overrides, a cookie persists the last choice, default is
   the persisted value. Same shape as the panel's `?view=` + localStorage.
3. **Scope guard:** M1's *content* scope (§3.1: Overview, Display path, Ingest, attempt history,
   left-join, self-freshness, host readouts) is unchanged. If tripling the template work threatens
   the milestone, dropping direction coverage (three → two) is the sanctioned cut — never dropping
   M1 content.
4. **New task (post-M1, unscheduled):** "Direction determination" — after real use, record the
   choice as a one-line decision in this file's index entry, delete the losing template sets and
   stylesheets, and remove the switcher.

## 3. Web dashboard — gets a plan home, and the same treatment

The dashboard previously existed only as `docs/design/web-dashboard/` with a README; no stage or
milestone owned its wiring. It is now a named deliverable, **gated on `vitalsd`'s query/health
path** (the same substrate the console's M0 produces):

1. Port directions **B (Night Report)** and **C (Terminal)** from
   [`../design/web-dashboard/claude-design/`](../design/web-dashboard/claude-design/) onto the
   *same* `model()` seam the ported direction A uses (`web-dashboard/index.html`) — one view
   model, three renderers, a layout switcher using the panel's `PANELS`-array pattern.
2. Wire `model()`'s internals to `vitalsd` once — the single seam feeds all three layouts, per
   the README's "Wiring it up" table (Now → Health + Subscribe · Last night → Query over one
   session · Trends → server-side per-night aggregation · System health → Health + backup state).
3. The README's two invariants bind all three ports: **a gap arrives as `null`** (nothing coerces
   missing to a number), and **trends aggregate server-side** (one point per night, never 30 ×
   30,000 samples over the wire).
4. Same post-evaluation pruning task as the console.

## 4. Bedside panel — no change

The switcher already shipped and is the precedent. The Stage 8 re-point from the stopgap
`status.json` to `vitalsd`'s health endpoint is unaffected.

## 5. Why this is the right trade

A layout for a safety-adjacent instrument is load-bearing UI: the failure mode of choosing from
mockups is discovering at 3 a.m., mid-incident, that the chosen layout buries the one number that
matters. The panel evaluation only produced a defensible choice because all three directions ran
on the real display with real data. The cost is bounded (template sets, not logic), the pruning is
mechanical, and the constraint in §2.1 prevents the temporary breadth from leaking into the data
layer.
