# Netcode

Arena uses a lockstep protocol that transports messages over WebSockets or WebRTC
DataChannels. The server drives a 60 Hz tick to keep clients in sync.

## Setup

Before starting, build the client and server as described in the
[README's building section](../README.md#building).

1. Start the server with networking enabled:
   ```bash
   cargo run -p server
   ```
2. Ensure the required headers are present so browsers can open a WebSocket:
   - `Cross-Origin-Opener-Policy: same-origin`
   - `Cross-Origin-Embedder-Policy: require-corp`
3. Clients connect to `ws://localhost:3000/ws` during startup. The server then
   negotiates WebRTC DataChannels when available. For deployment details see the
   [operations guide](ops.md).

## Usage

- Every 60 Hz tick the server broadcasts the authoritative state using
  delta-compressed snapshots.
- Clients submit input frames and run client-side prediction, reconciling when
  authoritative snapshots arrive.
- Messages are encoded with `bincode` and prefixed with a one-byte message ID.
- The transport layer supports WebSockets and WebRTC DataChannels and resends
  missed snapshots to tolerate packet loss.
- Modules can define custom message IDs; see the [modules guide](modules.md) for
  extending the protocol.

## Reference

| Message ID | Description        |
| ---------- | ------------------ |
| `0x01`     | Client input frame |
| `0x02`     | State snapshot     |
| `0x03`     | Chat message       |

- Default port: `3000`
- Tick rate: `60` Hz
- Snapshot compression: delta against last acknowledged state
- Serialization: `bincode`
- Disconnect timeout: `5` seconds
