# Modules

Arena modules extend the core game with new mechanics, assets, and server logic.
Modules are regular Rust crates that plug into the server during startup.

## Setup

1. Start by copying the template module located at `assets/modules/null_module`.
   This provides a minimal `module.toml` descriptor with the `LOBBY_PAD` capability.
2. Create a new crate under `crates/`:
   ```bash
   cargo new crates/my_module --lib
   ```
3. Add the crate to the workspace `Cargo.toml` and implement the `GameModule` trait exported by the core library.
4. Include any client-side assets in the module and rebuild the workspace with `cargo build`.
5. Create a descriptor at `assets/modules/<id>/module.toml` containing metadata and capability flags for the module.
6. Review the [netcode design](netcode.md) to understand how modules communicate with clients.

## Usage

- Register the module with the server by calling its `register()` function from `server/src/main.rs` or the module loader.
- Set capability flags in `module.toml` to advertise features like networking or UI.
- Rebuild and restart the server:
  ```bash
  cargo run -p server
  ```
- Clients automatically discover the module when they connect.

### Example: Null Module

The `NullModule` demonstrates the minimal structure of a module. It provides empty
hooks and can be registered on the client:

```rust
use null_module::NullModule;
use engine::{AppExt, EnginePlugin};

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(EnginePlugin)
    .add_game_module::<NullModule>()
    .run();
```

## Reference

- `GameModule` trait: defines lifecycle hooks for initialization and per-tick updates.
- `module.toml`: metadata file located at `assets/modules/<id>/module.toml`.
- Capability flags: `netcode`, `ui`, and other features declared in `module.toml`.
- Modules may send custom messages through the network layer; see the [netcode guide](netcode.md) for message types.
- Deployment notes for modules are covered in the [operations guide](ops.md).
- Template module: `null_module` serves as a starting point for new modules.
- Example module: [Duck Hunt](DuckHunt.md) demonstrating networking and asset integration.
