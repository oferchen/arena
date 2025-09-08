# Leaderboards

This guide describes how to configure and use Arena's leaderboard service.

## Configuration

| Env var                 | CLI flag            | Description                          | Default              |
| ----------------------- | ------------------- | ------------------------------------ | -------------------- |
| `SCYLLA_URI`            | `--scylla-uri`      | Connection string for Scylla cluster | `127.0.0.1:9042`     |
| `ARENA_REDIS_URL`       | `--redis-url`       | Redis URL for leaderboard cache      | `redis://127.0.0.1/` |
| `ARENA_LEADERBOARD_MAX` | `--leaderboard-max` | Maximum entries per leaderboard      | `100`                |

## Usage

Post scores via HTTP:

```bash
curl -X POST https://server/leaderboard -d '{ "player": "Alice", "score": 42 }'
```

Retrieve the top standings for a given window (`daily`, `weekly`, or `all_time`):

```bash
curl https://server/leaderboard/top
```

## Integration

The `leaderboard` crate exposes an API for submitting and querying scores.
Scores are stored in Scylla and the top standings for each leaderboard window
(`daily`, `weekly`, and `all_time`) are mirrored in Redis for fast access.
Register `LeaderboardPlugin` on the server to persist results and on the
client to display standings.

```rust
use leaderboard::LeaderboardPlugin;

App::new().add_plugins(LeaderboardPlugin);
```
