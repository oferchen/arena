# Leaderboards

Arena's leaderboard service persists results in Scylla and mirrors the
top standings in Redis for fast reads. Scores are tracked in three windows:
**daily**, **weekly**, and **all_time**.

## Configuration

| Env var                 | CLI flag            | Description                                  | Default |
| ----------------------- | ------------------- | -------------------------------------------- | ------- |
| `ARENA_DB_URL`          | `--db-url`          | Scylla database URL                          | -       |
| `ARENA_REDIS_URL`       | `--redis-url`       | Redis URL for the top‑N cache **(required)** | -       |
| `ARENA_LEADERBOARD_MAX` | `--leaderboard-max` | Maximum entries mirrored per leaderboard     | `100`   |

Each score submission writes a run and windowed score to Scylla.
The highest `ARENA_LEADERBOARD_MAX` scores for each window are maintained
in Redis sorted sets for quick retrieval.

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
Register `LeaderboardPlugin` on the server to persist results and on the
client to display standings.

```rust
use leaderboard::LeaderboardPlugin;

App::new().add_plugins(LeaderboardPlugin);
```
