# Selected generation-one provenance

Stage 7 starts from a narrow tree. No generation-one service directory or persistence model is copied.

| Stage 7 item | Generation-one evidence | What was retained | What was not retained |
|---|---|---|---|
| `AnxietyWatchApp/ClientRootView.swift` | `AnxietyWatch/App/ContentView.swift` | Native tab/navigation precedent | BLE overlays, local data services, demo routes, and analysis views |
| `AnxietyWatchWatch/QuickLogSkeletonView.swift` | `AnxietyWatch Watch App/AnxietyWatchApp.swift` and parity ledger | Watch owns a Quick Log surface with the 1–10 rating concept | SwiftData, sensor capture, complication feed, direct sync, and local analysis |
| `Packages/AWDomain/AnxietyRating.swift` | `clients/apple/GEN1-PARITY.yaml` (`journal.anxiety_rating`) | The validated 1–10 domain scale | Severity color as the only state signal and any local archive |

The disabled watch skeleton names its blocker. T-I16, not this task, implements durable WatchConnectivity
relay and enables capture.
