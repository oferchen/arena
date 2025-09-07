CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  leaderboard_id TEXT NOT NULL,
  player_id TEXT NOT NULL,
  replay_path TEXT NOT NULL,
  created_at TEXT NOT NULL,
  flagged INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS scores (
  id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  player_id TEXT NOT NULL,
  points INTEGER NOT NULL,
  window TEXT NOT NULL,
  FOREIGN KEY(run_id) REFERENCES runs(id) ON DELETE CASCADE,
  PRIMARY KEY (id, window)
);
