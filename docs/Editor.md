# Editor

This guide covers configuration and use of Arena's in-game level editor.

## Configuration

| Env var               | CLI flag          | Description                        | Default         |
| --------------------- | ----------------- | ---------------------------------- | --------------- |
| `ARENA_EDITOR`        | `--editor`        | Enable the editor on startup       | `false`         |
| `ARENA_EDITOR_ASSETS` | `--editor-assets` | Directory containing editor assets | `assets/editor` |

## Usage

Enable the editor and run the server:

```bash
ARENA_EDITOR=true cargo run -p server
```

Press `F1` in the client to toggle the editor UI. The editor supports multiple
interaction modes:

- **FirstPerson** – navigate the world while editing
- **Orthographic** – top‑down/ortho manipulation
- **PrefabPalette** – place prefabs from the palette
- **CsgBrush** – carve geometry with CSG brushes
- **SplineTool** – edit spline paths
- **Volume** – mark volume points
- **NavMesh** – bake and visualize navigation meshes
- **Validation** – run structural, gameplay and performance checks

Switch modes programmatically by updating `EditorClient::mode`. The client
tracks undo/redo history via `snapshot`, `undo` and `redo` helpers.

Levels are autosaved in the browser using OPFS with IndexedDB fallback via
`store_level_locally`/`load_level_locally`. Exporting a level writes a
deterministic TOML representation and hashed binaries to
`assets/levels/<level_id>/`.

The editor can play the current level in‑place using `play_in_editor`, which
invokes authoritative rules provided by a `platform_api::GameModule`.

## Integration

The editor systems are provided by the `editor` crate. Add `editor` as a
dependency and register `EditorPlugin` on both client and server:

```rust
use editor::EditorPlugin;

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(EditorPlugin)
    .run();
```

Assets saved by the editor can be loaded by other modules at runtime.
