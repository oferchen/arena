# Analytics

Arena can record gameplay events for later analysis.

## Configuration

| Env var                   | CLI flag              | Description                                    | Default |
| ------------------------- | --------------------- | ---------------------------------------------- | ------- |
| `ARENA_POSTHOG_KEY`       | `--posthog-key`       | PostHog API key (enables analytics)            | -       |
| `ARENA_ANALYTICS_OPT_OUT` | `--analytics-opt-out` | Disable analytics regardless of other settings | `false` |
| `ARENA_METRICS_ADDR`      | `--metrics-addr`      | OTLP metrics export address                    | -       |

## Usage

Provide a PostHog key and run the server:

```bash
ARENA_POSTHOG_KEY=phc_yourkey \
ARENA_METRICS_ADDR=127.0.0.1:4317 \
cargo run -p server
```

Events are retained in memory and optionally forwarded to PostHog or exported via OpenTelemetry metrics.

## Integration

Create an `Analytics` instance and dispatch events where appropriate:

```rust
use analytics::{Analytics, Event};

let analytics = Analytics::new(true, None, None);
analytics.dispatch(Event::PlayerJumped);
```

## Events

### Gameplay

- `player_joined` - emitted when a player connects
- `player_jumped` - emitted when a player jumps
- `player_died` - emitted when a player dies

### Economy

- `item_purchased` - player purchases an item
- `currency_earned` - player gains currency
- `currency_spent` - player spends currency
- `purchase_completed` - checkout finished successfully
  - `sku` - identifier of the purchased item
  - `user_id` - UUID of the purchasing user

### Performance

- `frame_dropped` - a frame took too long to render
- `high_latency` - network latency exceeded threshold
- `tick_overrun` - server tick exceeded budget
