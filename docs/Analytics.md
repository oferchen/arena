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

## Events

### Gameplay

- `player_joined` - emitted when a player connects
- `player_jumped` - emitted when a player jumps
- `player_died` - emitted when a player dies

### Economy

- `item_purchased` - player purchases an item
- `currency_earned` - player gains currency
- `currency_spent` - player spends currency

### Performance

- `frame_dropped` - a frame took too long to render
- `high_latency` - network latency exceeded threshold
- `tick_overrun` - server tick exceeded budget
