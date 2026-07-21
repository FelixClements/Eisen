# Relay Transport Contract

## Scope

This document defines the volatile peer-to-peer relay transport used for direct device-to-device synchronization when cloud sync is unavailable or as a complement to it.

## Authentication and authorization

- **Service authentication**: the relay service authenticates clients via short-lived tokens similar to the cloud service, but relay tokens do not authorize vault operations.
- **Device authorization**: peers prove membership by presenting a signed manifest summary and a per-capability relay capability token.
- Relay tokens are vault/session-scoped, high-entropy, expiring, and exchanged only through authenticated enrollment or user-confirmed channels. They are never logged in URLs.

## WebSocket subprotocol

- Peers connect over WSS to the relay.
- Subprotocol: `eisen-relay-v1`.
- After TCP/TLS handshake, the client sends an `AUTH` frame with the relay capability.

## Frames

All frames are Canonical CBOR maps with a `type` field. Frame types:

| Type | Purpose |
|---|---|
| `AUTH` | Authenticate with relay capability and present device identity proof. |
| `HELLO` | Peer handshake: protocol version, identity/membership proof, manifest summary. |
| `INVENTORY` | Per-origin contiguous sequence ranges and snapshot coverage. |
| `RANGE_REQ` | Request a range of operations by origin and sequence. |
| `RANGE_RESP` | Response with a bounded batch of envelopes. |
| `ACK` | Durable-local-commit acknowledgement for received envelopes. |
| `ERROR` | Explicit error code and disconnection reason. |
| `PING` / `PONG` | Keepalive. |

## Bounded frames

- Maximum frame size: 64 KiB.
- `RANGE_RESP` frames contain at most 100 envelopes.
- `INVENTORY` frames list at most 1,000 ranges.
- Peers reject oversized frames with `frame_too_large` and disconnect.

## Manifest exchange and range inventory encoding

- `INVENTORY` encodes coverage as `[(origin_device_id, start_seq, end_seq, has_snapshot)]`.
- Ranges are per-origin and contiguous; gaps are represented as separate range tuples.
- Snapshot coverage is encoded as `(checkpoint_hlc, snapshot_digest)` if a snapshot is available.

## Acknowledgement meaning

- An `ACK` is sent only after the envelope has been durably committed locally (log + materialized view + cursor/outbox state).
- Receipt of an `ACK` does not mean the message is backed up or persisted by the relay; the relay is volatile.
- Peers retain envelopes in their local outbox until acknowledged by all connected peers or until cloud delivery confirms receipt.

## Reconnect rules

- On reconnect, a peer discards all volatile assumptions.
- It re-authenticates, re-exchanges `HELLO`, and sends a fresh `INVENTORY`.
- Reconciliation resumes from the new inventory; no relay cursor is shared with the cloud cursor.
- After a configurable idle timeout, the relay may disconnect the peer; reconnect follows the same rules.

## Resource controls

- Per-peer limits: 64 KiB frame size, 100 messages/second, 1,000 queued frames, 5-minute handshake timeout, 10-minute idle timeout.
- Slow peers are isolated and disconnected with `slow_peer`.
- Flood / oversized behavior triggers disconnection with `abuse`.

## Dual-transport dedupe

- A peer may use both cloud and relay transports simultaneously.
- The same immutable log and outbox are the source of truth for both.
- Cloud cursors and relay inventory are kept separate.
- Deduplication is by operation ID and digest across both transports.
