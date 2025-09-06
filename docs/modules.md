# Modules

Arena modules extend the core game with new mechanics, assets, and server logic. Modules are regular Rust crates that plug into the server during startup.

## Setup

1. Create a new crate under `crates/`:
   ```bash
   cargo new crates/my_module --lib
   ```
2. Add the crate to the workspace `Cargo.toml` and implement the `Module` trait exported by the core library.
3. Include any client-side assets in the module and rebuild the workspace with `cargo build`.
4. Review the [netcode design](netcode.md) to understand how modules communicate with clients.

## Usage

- Register the module with the server by calling its `register()` function from `server/src/main.rs` or the module loader.
- Rebuild and restart the server:
  ```bash
  cargo run -p server
  ```
- Clients automatically discover the module when they connect.

## Reference

- `Module` trait: defines `fn register(&mut World)` and `fn update(&mut World, dt: f32)` hooks.
- `ServerPlugin`: convenience wrapper for attaching modules at runtime.
- Modules may send custom messages through the network layer; see the [netcode guide](netcode.md) for message types.
- Deployment notes for modules are covered in the [operations guide](ops.md).
