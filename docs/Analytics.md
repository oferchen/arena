# Analytics

Arena records gameplay events in the database for later analysis. Events can
optionally be forwarded to PostHog or exported via OpenTelemetry metrics.

## Configuration

| Env var                         | CLI flag                    | Description                                    | Default |
| ------------------------------- | --------------------------- | ---------------------------------------------- | ------- |
| `ARENA_ANALYTICS_LOCAL`         | `--analytics-local`         | Store analytics events locally                 | `false` |
| `ARENA_POSTHOG_KEY`             | `--posthog-key`             | PostHog API key (optional sink)                | -       |
| `ARENA_POSTHOG_URL`             | `--posthog-url`             | PostHog endpoint URL                           | -       |
| `ARENA_ANALYTICS_OPT_OUT`       | `--analytics-opt-out`       | Disable analytics regardless of other settings | `false` |
| `ARENA_ANALYTICS_OTLP_ENDPOINT` | `--analytics-otlp-endpoint` | OTLP metrics export address                    | -       |

## Usage

Provide a PostHog key and run the server:

```bash
ARENA_ANALYTICS_LOCAL=true \
ARENA_POSTHOG_KEY=phc_yourkey \
ARENA_POSTHOG_URL=https://app.posthog.com/capture/ \
ARENA_ANALYTICS_OTLP_ENDPOINT=127.0.0.1:4317 \
cargo run -p server
```

Events are written to the `analytics_events` table. A background task
periodically aggregates them into `analytics_rollups` and, if configured,
forwards events to PostHog or emits OTLP metrics.

## Integration

Create an `Analytics` instance and dispatch events where appropriate:

```rust
use analytics::{Analytics, Event};

let db: sea_orm::DatabaseConnection = /* obtain from your app */;
let analytics = Analytics::new(true, Some(db), None, None);
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
