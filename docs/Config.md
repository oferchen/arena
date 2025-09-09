# Configuration

Arena is configured through environment variables prefixed with `ARENA_` or
equivalent CLI flags. `ARENA_BIND_ADDR` or the `--bind-addr` flag must be
provided when launching the server. Arena uses PostgreSQL with SeaORM for data
persistence.

## Server

| Env var                 | CLI flag            | Description                                  | Default              |
| ----------------------- | ------------------- | -------------------------------------------- | -------------------- |
| `ARENA_BIND_ADDR`       | `--bind-addr`       | Address to bind the server to **(required)** | -                    |
| `ARENA_PUBLIC_BASE_URL` | `--public-base-url` | Public base URL of the server                | -                    |
| `ARENA_DB_URL`          | `--db-url`          | PostgreSQL database URL                      | -                    |
| `ARENA_CSP`             | `--csp`             | Content Security Policy header value         | `default-src 'self'` |

## RTC

| Env var                  | CLI flag             | Description                            | Default |
| ------------------------ | -------------------- | -------------------------------------- | ------- |
| `ARENA_SIGNALING_WS_URL` | `--signaling-ws-url` | WebSocket URL for the signaling server | -       |

## Analytics

| Env var                   | CLI flag              | Description                                    | Default |
| ------------------------- | --------------------- | ---------------------------------------------- | ------- |
| `ARENA_ANALYTICS_OPT_OUT` | `--analytics-opt-out` | Disable analytics regardless of other settings | `false` |
| `ARENA_POSTHOG_KEY`       | `--posthog-key`       | PostHog API key (enables analytics)            | -       |

## Metrics

| Env var              | CLI flag         | Description                 | Default |
| -------------------- | ---------------- | --------------------------- | ------- |
| `ARENA_METRICS_ADDR` | `--metrics-addr` | OTLP metrics export address | -       |

## Email

| Env var                 | CLI flag            | Description                               | Default |
| ----------------------- | ------------------- | ----------------------------------------- | ------- |
| `ARENA_SMTP_HOST`       | `--smtp-host`       | SMTP server hostname                      | -       |
| `ARENA_SMTP_PORT`       | `--smtp-port`       | SMTP server port                          | -       |
| `ARENA_SMTP_FROM`       | `--smtp-from`       | Sender address for all mail               | -       |
| `ARENA_SMTP_STARTTLS`   | `--smtp-starttls`   | STARTTLS mode (`auto`, `always`, `never`) | `auto`  |
| `ARENA_SMTP_SMTPS`      | `--smtp-smtps`      | Use SMTPS (implicit TLS)                  | `false` |
| `ARENA_SMTP_USER`       | `--smtp-user`       | SMTP username                             | -       |
| `ARENA_SMTP_PASS`       | `--smtp-pass`       | SMTP password                             | -       |
| `ARENA_SMTP_TIMEOUT_MS` | `--smtp-timeout-ms` | Connection timeout in milliseconds        | `10000` |

## Leaderboards

| Env var                 | CLI flag            | Description                              | Default   |
| ----------------------- | ------------------- | ---------------------------------------- | --------- |
| `ARENA_LEADERBOARD_MAX` | `--leaderboard-max` | Maximum entries mirrored per leaderboard | `100`     |
| `ARENA_REPLAYS_DIR`     | `--replays-dir`     | Directory where match replays are stored | `replays` |

## Editor

| Env var               | CLI flag          | Description                        | Default         |
| --------------------- | ----------------- | ---------------------------------- | --------------- |
| `ARENA_EDITOR`        | `--editor`        | Enable the editor on startup       | `false`         |
| `ARENA_EDITOR_ASSETS` | `--editor-assets` | Directory containing editor assets | `assets/editor` |

## Purchases

The purchases module does not define additional `ARENA_*` variables. See
[`Purchases`](Purchases.md) for details on entitlement flow and catalog usage.
