# Broker test seams explained

## Purpose

The BLE broker is both a transport daemon and a safety boundary. It owns the only
application-level BlueZ access, arbitrates leases between workers, enforces typed
capabilities, bounds operations, cancels work after deadlines, and reports adapter
generation changes. A useful test suite must therefore prove more than protocol
serialization. It must prove that the broker makes the correct ownership decision,
that cleanup is reliable, and that no worker can bypass the boundary.

The full broker acceptance contract remains:

```text
cargo llvm-cov --package vitals-ble-broker --all-features --all-targets \
  --fail-under-lines 100 \
  --fail-under-functions 100 \
  --fail-under-regions 100 \
  --summary-only
```

This document explains the seams needed to reach that contract without pretending
that a fake radio is evidence about a real BlueZ installation.

## What a test seam is

A seam is a deliberately small interface at which production code can be supplied
with a deterministic collaborator during a test. The production collaborator still
runs in the deployed daemon. The test collaborator exercises the same policy and
lifecycle code without requiring a physical radio, a particular `hciN` alias, or an
uncontrolled D-Bus daemon.

A good seam has four properties:

1. **The safety decision stays in production code.** Tests do not reimplement lease,
   capability, cancellation, or cleanup policy.
2. **The substitute is smaller than the real backend.** A fake owner returns named
   results and controlled streams; it does not pretend to implement BlueZ.
3. **The boundary is typed.** There is no generic D-Bus method, arbitrary object path,
   or unbounded byte escape hatch added merely to make a test pass.
4. **The real backend remains separately testable.** Linux/BlueZ integration tests
   still exercise adapter selection, error classification, and the actual API shape.

A seam is not a mock of the product. It is a testable boundary around a production
policy decision.

## The broker layers and their seams

### 1. Stable controller selection

`bluez.rs` has a portable selection layer that consumes controller evidence and a
stable selector. It does not need D-Bus to answer these questions:

- Does the selector contain a stable identity?
- Does exactly one inventory record match?
- Are two runtime aliases actually one physical controller?
- Is a missing or ambiguous controller rejected with the blocking count?

These are pure evidence tests. They use temporary inventory trees and synthetic
`ControllerEvidence` values. They must never use `hci0` as identity; a runtime name
may appear only as evidence returned after stable selection.

The Linux owner then has a narrow backend seam: it receives the already-resolved
controller and opens the selected adapter. The backend tests cover BlueZ/D-Bus error
classification and adapter-loss behavior separately from the portable selector
suite.

### 2. Lease scheduler and capability policy

The lease scheduler is already close to a pure seam. It accepts a request and a
caller-supplied monotonic tick, then returns a typed result:

- granted lease;
- queued request with a named blocker;
- policy rejection;
- expiry/release dispatch;
- generation revocation;
- cancellation of a queued request.

Tests must use fixed ticks, not wall-clock time. They should cover every intent,
priority ordering, FIFO tie-break, maximum hold, deadline, preemption request,
source restriction, disabled source, and generation transition.

The scheduler must be tested directly as policy and through the broker admission
path. The direct tests prove the state machine; the broker tests prove that a worker
cannot request a capability that was not negotiated and that the lease identity is
carried into every operation.

### 3. Typed worker protocol

The protocol seam is the byte boundary. The deterministic fake transport is useful
here because it validates the protocol state machine without opening a Unix socket
or BlueZ:

- handshake required before health or operations;
- handshake cannot be repeated;
- protocol version, source, selector, and capability bits are bounded;
- BP cannot negotiate userspace connection authority;
- every client and server message variant round-trips;
- malformed magic, version, kind, UTF-8, length, payload, and trailing bytes fail
  closed;
- queue capacity is enforced and reported.

These tests must exercise the decoder and encoder directly. They must not merely
assert that a higher-level client receives a value, because that can leave an
individual wire variant uncovered.

### 4. Unix client socket

`client.rs` has a socket seam using a private Unix listener in a Linux integration
test. The test peer speaks the real encoded protocol and drives the real
`BrokerClient`; it is not a replacement client implementation.

The scripted peer must cover:

- successful source handshake and generation assignment;
- lease grant, queue rejection, broker rejection, and unexpected response;
- every typed operation wrapper;
- opaque characteristic handles;
- frame, device-state, and disconnected events;
- operation deadlines followed by cancellation acknowledgement;
- cancellation rejection, unexpected cancellation response, and socket failure;
- malformed or oversized server frames;
- release success, release rejection, and mismatched release response.

This seam proves operation correlation, bounded writes, timeout behavior, and client
error redaction while leaving the production framing and cancellation code in the
execution path.

### 5. Server dispatch and owner seam

The most important new seam is the server owner boundary. Before this seam, the
server stored a concrete `LinuxBluezAdapterOwner`, making dispatch tests dependent
on constructing a live BlueZ session and leaving most of `server.rs` uncovered.

The server should depend on a small typed `BrokerOwner` interface with these
operations:

- generation and lease policy operations;
- health;
- bounded discovery;
- kernel-device observation;
- connection and disconnect;
- exact service/characteristic resolution;
- typed notification subscription;
- bounded characteristic write.

The production implementation delegates each method to
`LinuxBluezAdapterOwner`. The test implementation owns only deterministic policy
state and returns a typed fake characteristic plus a finite notification stream.
It must not contain BlueZ logic.

With that seam, server tests can drive the actual dispatch functions and assert:

- valid handshake, health, lease, operation, and release sequences;
- source and selector mismatch rejection;
- capability missing rejection for every operation kind;
- invalid target address and mismatched lease address rejection;
- invalid service/characteristic selector rejection;
- unknown characteristic-handle rejection;
- successful observation, discovery, connect, resolve, subscribe, and write;
- operation-owner failures converted to named error responses;
- subscription frame forwarding and disconnected notification;
- duplicate operation IDs;
- cancellation of active and completed operations;
- client cleanup aborting operations, releasing leases, disconnecting eligible
  devices, and removing that client's handles;
- response-queue closure handling.

This is the seam that makes server coverage meaningful: the production server
control flow runs unchanged, while only the hardware-dependent owner result is
controlled.

### 6. BlueZ backend seam

The real Linux owner must not be replaced by a fake in the host build. Its tests are
Linux-only and should be divided into two classes:

1. **Pure backend decisions:** lease validation, intent checks, BP prohibition,
   operation classification, stable controller state, and BlueZ error-kind mapping.
2. **Linux integration:** a controlled BlueZ/D-Bus fixture or an approved Pi test
   that exercises adapter/session/device/GATT calls.

A D-Bus fixture is acceptable only when it tests the real `bluer` integration and its
interface contract. A fixture that merely returns values to a duplicate fake owner
would test the mock, not the broker. Physical Pi acceptance remains a separate gate
and must report hardware evidence independently of coverage.

### 7. Process bootstrap

The daemon `main` should remain a thin bootstrap around testable configuration and
runtime startup. Configuration parsing is tested directly for:

- missing selector;
- invalid selector;
- invalid allowed UID;
- default socket path;
- explicit socket path;
- valid stable selector.

The infinite listener loop is not made finite merely for coverage. Instead, its
setup failures and client isolation are exercised through testable helpers, and the
Linux integration gate verifies that the real process can start against the approved
adapter environment.

## What must not become a seam

The following are not acceptable shortcuts:

- replacing `bluer` with a fake on macOS and calling that a Linux build;
- adding a generic D-Bus operation to simplify tests;
- bypassing capability checks in test-only server code;
- injecting an `hciN` name where the production path requires stable identity;
- making cancellation a test-only behavior;
- excluding large production files from LLVM coverage because they are difficult;
- marking all hardware-dependent functions as uncovered without a documented
  integration strategy;
- fabricating physiological samples to exercise collector acceptance.

A test may use synthetic addresses, selectors, payloads, and error values, but it
must preserve the same typed boundaries and fail-closed decisions as production.

## Coverage accounting

Coverage is evaluated at three levels:

- **Lines:** every executable statement in the broker is executed;
- **Functions:** every production function, including named failure helpers, is
  entered by a deterministic or Linux integration test;
- **Regions:** both sides of each meaningful branch are exercised, including
  cancellation, cleanup, rejection, and error conversion.

A round-trip test can cover an encoder branch but leave decoder failures untouched.
A successful fake-owner test can cover dispatch but leave lease mismatch and cleanup
branches untouched. The coverage report, not the number of tests, identifies the
remaining work.

The staged testing plan is therefore:

1. first cover the highest-risk client, protocol, lease, and dispatch paths;
2. after each migration step, add the safety-critical tests needed before its
   commit or deployment;
3. inspect the LLVM text report for uncovered regions;
4. add or improve a seam rather than lowering the threshold;
5. run the exact 100% command as the final broker task.

## Relationship to OpenSpec

This document explains the implementation of the existing broker coverage and
ownership requirements. It does not weaken them. The OpenSpec change remains the
source of truth for scope, scenarios, task ordering, and exact verification
commands. If implementation experience reveals a requirement or ordering change,
that change must first be proposed as an OpenSpec amendment and reviewed before
implementation proceeds.

## Current status

The broker already has deterministic lease/protocol tests, Linux selector tests,
and Unix client tests. The owner seam is being added so the server's actual typed
dispatch and cleanup paths can be tested without claiming that a fake radio is a
physical BlueZ acceptance result. The final 100% LLVM gate remains mandatory.
