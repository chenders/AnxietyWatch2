# BLE broker telemetry

`vitals-ble-broker` publishes a Prometheus textfile snapshot every 10 seconds at
`/run/vitals/ble-broker.prom`. The path is configurable with
`VITALS_BLE_BROKER_METRICS_PATH`. The snapshot contains only low-cardinality
source and operation labels; it never contains device selectors, addresses,
GATT handles, reasons, or payloads.

## Client use and throughput

- `vitals_ble_broker_clients_connected_total{source=...}` and
  `vitals_ble_broker_clients_active{source=...}` show who is using the broker.
- `vitals_ble_broker_operations_total{source=...,operation=...}` counts typed
  operations. Success, failure, cancellation, and total duration counters are
  exported alongside it.
- `vitals_ble_broker_request_bytes_total{source=...}` reports encoded requests
  per source. The unlabelled metric is the aggregate. Response bytes and raw
  GATT notification payload bytes are exported separately.
- Lease grants, bounded queue attempts, releases, rejected handshakes, and
  active clients are recorded independently.

Useful rates are ordinary Prometheus rates, for example:

```promql
sum by (source) (rate(vitals_ble_broker_operations_total[10m]))
sum by (source) (rate(vitals_ble_broker_request_bytes_total[10m]))
```

## Resource comparison

The broker exports its observed process RSS and CPU jiffies. It also exports an
explicitly named **estimate** of the broker-only cost that would disappear if
clients performed the same typed work directly:

- `vitals_ble_broker_estimated_direct_mode_processes_removed`
- `vitals_ble_broker_estimated_direct_mode_rss_bytes_saved`
- `vitals_ble_broker_estimated_direct_mode_cpu_jiffies_saved`
- `vitals_ble_broker_estimated_direct_mode_ipc_bytes_avoided_total`

These are not claims about total host savings. Direct BlueZ clients could use
more memory or CPU through duplicated discovery, competing leases, or separate
connections. The metrics intentionally quantify only the removable broker
process and broker IPC overhead; the operation and notification counters are
the evidence needed to estimate any additional direct-mode duplication.
