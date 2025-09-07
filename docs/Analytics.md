# Analytics

Arena can record gameplay events for later analysis.

## Configuration

| Env var                    | CLI flag               | Description                     | Default |
| -------------------------- | ---------------------- | ------------------------------- | ------- |
| `ARENA_ANALYTICS_ENDPOINT` | `--analytics-endpoint` | HTTP endpoint to receive events | -       |
| `ARENA_ANALYTICS_BATCH`    | `--analytics-batch`    | Number of events per upload     | `20`    |
| `ARENA_ANALYTICS_ENABLED`  | `--analytics`          | Enable analytics collection     | `false` |

## Usage

Enable analytics and run the server:

```bash
ARENA_ANALYTICS_ENABLED=true \
ARENA_ANALYTICS_ENDPOINT=https://example.com/events \
cargo run -p server
```

Events are queued and sent in batches to the configured endpoint.

## Integration

Import the `analytics` crate and call `track_event` where appropriate:

```rust
use analytics::track_event;

track_event("player_jump", &["height", "2.3"]);
```

Attach `AnalyticsPlugin` to the server to automatically forward events.
