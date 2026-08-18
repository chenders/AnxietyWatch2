# Recovered operator reports — 2026-08-17 night session

The operator reported hearing aircraft in chat messages to the prior assistant
as well as pressing the bedroom-door Hue button. This file recovers those
report times from the pi session transcript metadata so they can be recorded
as durable corroboration evidence — **without ever duplicating the bridge
events, whose timestamps remain authoritative**.

## Timezone / conversion provenance

- Message times come from **session JSONL timestamp metadata** in
  `~/.pi/agent/sessions/--Users-chris-Source-AnxietyWatch2--/`
  `2026-08-17T00-23-06-944Z_01a00d19-c1c0-7920-a58f-6422eb9399c7.jsonl` —
  ISO-8601 **UTC (Z suffix), millisecond precision**, stamped by the transport
  at receipt. Never derived from prose or wall-clock guesses.
- Conversion: `datetime.fromisoformat(iso.replace("Z", "+00:00")).timestamp()`
  → UTC epoch seconds, exact (verified against the values below).
- Bridge press times come from the label store (`observed_at`, also encoded in
  each `event_id` as epoch-milliseconds) and are the Hue bridge's own event
  timestamps, NTP-disciplined by the capture service.

## Correlation table

| Operator message (UTC metadata) | epoch | Says | Bridge event | Press (UTC) | Gap |
|---|---|---|---|---|---|
| 02:58:03.813 | 1786935483.813 | "just heard a plane and clicked the button" | `hue-1786935446780-1` | 02:57:26.78 | +37.0 s |
| 03:04:07.366 | 1786935847.366 | "i pushed the button - its a faint sound somewhat in the distance but definitely notable" | `hue-1786935825979-1` | 03:03:45.98 | +21.4 s |
| 03:12:41.766 | 1786936361.766 | "i pressed the button first for me, then two times because this one is bothering my cat" | `hue-1786936343158-1` + `hue-1786936345840-2` | 03:12:23.16 / 03:12:25.84 | +18.6 s |
| 03:15:27.217 | 1786936527.217 | "the two presses also encompass an episode where i heard it first" — the semantic ruling: a double press includes a human-heard episode | (clarifies the two events above) | — | — |

Presses with **no matching chat message** (button-only, already durable):
`hue-1786937329736-1` (03:28:49), `hue-1786938466247-1` (03:47:46),
`hue-1786938905891-1` (03:55:05), `hue-1786939173123-1` (03:59:33),
`hue-1786939519667-1` (04:05:19), `hue-1786939908632-1` (04:11:48).

## The lost press — needs a real event, not a corroboration

At **04:09:49 UTC (epoch ≈ 1786939789)** the operator pressed the button and
the deployed capture recorded **no label row**: the bridge stream reconnects
every ~90 s (no keepalives) and the then-deployed parser counted only
`initial_press`, which fell in the reconnect blind window. Evidence: the
operator's report in-session, plus the capture state showing the bridge SSE
event id `1786939789:0` persisted with no corresponding gesture. The parser
now recovers unpaired releases (`PressExtractor`), so this class of loss ends
with the next deploy.

## Commands for the operator (run on the Pi after deploy — not run by the agent)

```sh
# The lost press: a REAL event with the exact bridge-observed second.
panel-aircraft-report --at 1786939789 \
  --source "operator report + bridge SSE id 1786939789:0; label row lost to initial_press-only parsing" \
  --note "pressed the button; press lost in reconnect blind window (see recovered-operator-reports-2026-08-17.md)"

# Corroborations: preserved words and timing, never second events.
panel-aircraft-report --corroborates hue-1786935446780-1 \
  --iso 2026-08-17T02:58:03.813Z --source "chat 01a00d19" \
  --note "just heard a plane and clicked the button"
panel-aircraft-report --corroborates hue-1786935825979-1 \
  --iso 2026-08-17T03:04:07.366Z --source "chat 01a00d19" \
  --note "faint sound somewhat in the distance but definitely notable"
panel-aircraft-report --corroborates hue-1786936343158-1 \
  --iso 2026-08-17T03:12:41.766Z --source "chat 01a00d19" \
  --note "pressed first for me, then two times because this one is bothering my cat"
panel-aircraft-report --corroborates hue-1786936345840-2 \
  --iso 2026-08-17T03:12:41.766Z --source "chat 01a00d19" \
  --note "double press = cat bothered; also encompasses the human-heard episode (operator ruling 03:15:27Z)"
```
