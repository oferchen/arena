# Operations

This guide covers deploying Arena and operating it in production environments.

## Setup

1. Install system dependencies:
   - Rust toolchain (`rustup`)
   - Node.js and npm
2. Build the workspace in release mode:
   ```bash
   cargo build --release
   npm install
   npm run build
   ```
3. Configure the server environment. Useful variables include:
   - `ARENA_PORT` – TCP port to listen on (default `3000`)
   - `ARENA_DATA_DIR` – path to persistent data
   - `ARENA_SMTP_*` – outgoing mail settings; see [Email configuration](Email.md)

## Usage

- Start the server:
  ```bash
  ARENA_PORT=3000 cargo run -p server --release
  ```
- Serve the `web/` directory with your preferred static file server.
- Monitor the process and restart on failure using a supervisor such as `systemd` or `pm2`.
- For multiplayer features such as WebRTC DataChannels ensure the required
  headers are set; see the [netcode guide](netcode.md).
- Modules can be added or removed without downtime; refer to the [modules
  guide](modules.md) for capability flags and packaging via
  `assets/modules/<id>/module.toml`.

## Reference

- Deployment artifacts are produced in `target/release/`.
- Logs are written to standard output and should be captured by your hosting platform.
- Recommended health check: `GET /healthz` expecting a `200` response.
- Backup the `ARENA_DATA_DIR` regularly to safeguard persistent state.
