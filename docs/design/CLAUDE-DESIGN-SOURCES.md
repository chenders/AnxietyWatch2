# Claude Design sources — provenance

The `claude-design/` subdirectories under `docs/design/*/` are **verbatim exports** from
claude.ai/design projects, retrieved 2026-08-05 (via the DesignSync API and the design UI's
folder download). They are the canonical design references; the sibling directories
(`pi-panel/direction-a-clinical/`, `web-dashboard/`) hold the hand-ported vanilla
implementations that actually run.

| Local path | Claude Design project | Project ID | Contents |
|---|---|---|---|
| `pi-panel/claude-design/` | "Bedside Status Panel Designs" | `b8c918e3-c9cb-4c96-859a-19e6adcfbee6` | Three 800×480 panel directions (Clinical / Penumbra / Semaphore): a rationale doc + a `*Panel.dc.html` component each, with the five-state PanelState fixtures |
| `web-dashboard/claude-design/` | "Design fixes: hatch and theme" | `6bfab67a-4990-4c2c-ac4e-43fe46373569` | Three dashboard directions (A Clinical / B Night Report / C Terminal) + two complete standalone builds (`standalone-source.html` = A, `standalone-source-b.html` = B) |
| `admin-console/claude-design/` | "Three design directions" | `b5785f63-9247-41c9-a460-f79dfa9603eb` | Admin console: 3 directions (a "Service Mode" / b "Preflight" / c "Flowpath") × 17 files each (15 HTML pages + console.css/app.js), all vanilla HTML/CSS/JS, no dependencies |
| `progress-map/claude-design/` | "AnxietyWatch2 Progress Map" | `7729e2af-8148-4c2b-90fe-e11cd0c6306f` | Dependency-ordered platform-migration progress map (stages, gates, task table, hardware) |

## Notes

- **`.dc.html` files** are Claude Design's component format: an `<x-dc>` template plus a
  `DCLogic` class, rendered by `support.js` (the dc-runtime) *which expects `window.React`*.
  They preview inside claude.ai but are **not directly servable** — port to vanilla (as
  `direction-a-clinical/index.html` did) before wiring to real data. The admin-console
  export is already vanilla and opens straight from disk.
- **Panel status**: all three panel directions are already ported into
  `pi-panel/direction-a-clinical/index.html` as swipeable views (PR #12, #15), live-driven by
  `status.json` on the Pi (`?live`), selectable via swipe / `?view=` / localStorage. The
  design project's later "contrast fix" edits brought the design files in line with values
  the port already used (verified against the token table 2026-08-05).
- **Superseded projects** (not retrieved): "Panel measurement fixes" `3ef45d93` (earlier
  panel round: Clinical/Aperture/Ledger), "Raspberry Pi Medical Monitor Progress"
  `0e5d635b` (earlier progress map), "Anxiety Watch design exploration" `7f85eedc`.
- Two >256 KiB bundled exports ("AnxietyWatch - Clinical Dashboard.html" / "Night Report")
  were dropped: the DesignSync read cap truncates them, and they are exactly the standalone
  sources + inlined runtime, so nothing is lost.
