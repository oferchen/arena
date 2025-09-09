# Leaderboards

Arena's leaderboard service persists results in PostgreSQL using SeaORM as the
persistence layer. Scores are tracked in three windows: **daily**, **weekly**,
and **all_time**.

## Configuration

| Env var                 | CLI flag            | Description                              | Default |
| ----------------------- | ------------------- | ---------------------------------------- | ------- |
| `ARENA_DB_URL`          | `--db-url`          | PostgreSQL database URL                  | -       |
| `ARENA_LEADERBOARD_MAX` | `--leaderboard-max` | Maximum entries mirrored per leaderboard | `100`   |

Each score submission writes a run and windowed score to PostgreSQL via
SeaORM. The highest
`ARENA_LEADERBOARD_MAX` scores for each window are maintained for quick
retrieval.

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
