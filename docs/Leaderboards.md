# Leaderboards

This guide describes how to configure and use Arena's leaderboard service.

## Configuration

| Env var                 | CLI flag            | Description                     | Default                   |
| ----------------------- | ------------------- | ------------------------------- | ------------------------- |
| `ARENA_LEADERBOARD_DB`  | `--leaderboard-db`  | Database URL for storing scores | `sqlite://leaderboard.db` |
| `ARENA_LEADERBOARD_MAX` | `--leaderboard-max` | Maximum entries per leaderboard | `100`                     |

## Usage

Post scores via HTTP:

```bash
curl -X POST https://server/leaderboard -d '{ "player": "Alice", "score": 42 }'
```

Retrieve the top standings:

```bash
curl https://server/leaderboard/top
```

## Integration

The `leaderboard` crate exposes an API for submitting and querying scores.
Register `LeaderboardPlugin` on the server to persist results and on the
client to display standings.

```rust
use leaderboard::LeaderboardPlugin;

App::new().add_plugins(LeaderboardPlugin);
```
