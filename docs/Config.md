# Config

Arena server configuration is controlled via environment variables or CLI flags.

| Env var            | CLI flag         | Description                          | Default              |
| ------------------ | ---------------- | ------------------------------------ | -------------------- |
| `ARENA_BIND_ADDR`  | `--bind-addr`    | Address to bind the server to        | `0.0.0.0:3000`       |
| `ARENA_PUBLIC_URL` | `--public-url`   | Public URL of the server             | -                    |
| `ARENA_SHARD_HOST` | `--shard-host`   | Hostname for shard connections       | -                    |
| `SCYLLA_URI`       | `--database-url` | ScyllaDB connection string           | -                    |
| `ARENA_CSP`        | `--csp`          | Content Security Policy header value | `default-src 'self'` |
