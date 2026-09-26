# BLE antenna comparison

## Runs

- Baseline/current antenna: `baseline-current-antenna-20260917T015420Z.md`
- New antenna: `new-antenna-20260917T020455Z.md`

## Timing correction

The operator reported after both runs:

- The new-antenna run included approximately 20 seconds of stationary time before walking away.
- The baseline run ended immediately when the operator returned, without the planned 10-second recovery wait.

The logs do not contain explicit operator markers for leaving, turning around, or returning. Consequently, total logger duration cannot be used as a direct range or recovery comparison.

## Recorded observations

| Metric | Current antenna | New antenna |
|---|---:|---:|
| Logger elapsed range | 2–194 seconds | 2–134 seconds |
| Polar H10 target rows | 182 | 126 |
| Wellue target rows | 182 | 126 |
| H10 rows reporting connected=yes | 182/182 | 126/126 |
| Wellue rows reporting connected=yes | 182/182 | 126/126 |
| H10 fresh RSSI samples | 1 | 1 |
| H10 RSSI sample | -45 dBm | -46 dBm |
| Wellue fresh RSSI samples | 0 | 0 |
| Observed target disconnects | 0 | 0 |
| Observed reconnect events | 0 | 0 |

The raw logs contain timestamped heartbeat/target entries and explicitly preserve `NA` when BlueZ supplied no fresh RSSI. Some elapsed seconds have no heartbeat because the polling commands overran the one-second interval; those gaps are retained rather than fabricated.

## Interpretation

This pair of runs does **not** establish that the new antenna improved or worsened range:

1. The walking and return boundaries were not marked, and the two stationary timing windows differed.
2. Neither device reported a disconnect during either captured logger window, so there is no observed loss/recovery event to compare.
3. Wellue supplied no fresh RSSI values in either run.
4. The H10 RSSI comparison is one sample per run: -45 dBm versus -46 dBm. A one-dBm difference with one sample is not evidence of an antenna effect.
5. The longer baseline duration and shorter new-antenna duration cannot be interpreted as longer or shorter range because the operator timing and route phase boundaries differ.

## Conclusion

**Inconclusive.** These runs show that both target records remained connected for the portions captured, but they do not demonstrate an antenna benefit. There is no defensible basis from these data alone to choose the new antenna over the current one.

The next useful test should keep the same route and pace, mark the start of walking and return explicitly, use the same stationary duration at both ends, and capture controller-level RSSI or another fresh radio measurement for both targets. If no disconnect occurs, the result should be reported as “no loss observed” rather than “greater range proven.”

No credentials, PHI, network addresses, or device addresses are included in this comparison.
