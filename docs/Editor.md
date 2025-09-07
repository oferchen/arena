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

Press `F1` in the client to toggle the editor UI. Create or modify entities,
then save the scene to disk.

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
