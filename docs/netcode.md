# Netcode

Arena uses a lockstep protocol over WebSockets to keep clients and the server in sync.

## Setup

1. Start the server with networking enabled:
   ```bash
   cargo run -p server
   ```
2. Ensure the required headers are present so browsers can open a WebSocket:
   - `Cross-Origin-Opener-Policy: same-origin`
   - `Cross-Origin-Embedder-Policy: require-corp`
3. Clients connect to `ws://localhost:3000/ws` during startup. For deployment details see the [operations guide](ops.md).

## Usage

- Each tick the server broadcasts the authoritative state to all clients.
- Clients submit input frames, which the server validates and distributes.
- Messages are encoded with `bincode` and prefixed with a one-byte message ID.
- The protocol tolerates packet loss by resending missed state snapshots.
- Modules can define custom message IDs; see the [modules guide](modules.md) for extending the protocol.

## Reference

| Message ID | Description        |
| ---------- | ------------------ |
| `0x01`     | Client input frame |
| `0x02`     | State snapshot     |
| `0x03`     | Chat message       |

- Default port: `3000`
- Tick rate: `60` Hz
- Serialization: `bincode`
- Disconnect timeout: `5` seconds
