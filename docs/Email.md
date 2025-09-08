# Email

This guide covers configuration of Arena's outgoing email system. Build the
project as outlined in the [README](../README.md#building) before configuring
email.

## Environment variables and CLI flags

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

## Authentication

Arena supports basic username/password authentication. Provide
`ARENA_SMTP_USER`/`--smtp-user` and `ARENA_SMTP_PASS`/`--smtp-pass` to
authenticate with your SMTP provider. If these are not set, Arena
connects without authentication.

## STARTTLS and SMTPS

Set `ARENA_SMTP_STARTTLS`/`--smtp-starttls` to control STARTTLS usage:

- `auto` – opportunistically upgrade to TLS if the server supports it
- `always` – require STARTTLS, failing if unsupported
- `never` – disable STARTTLS

If your provider uses SMTPS (implicit TLS, typically port `465`), enable
`ARENA_SMTP_SMTPS=true` or `--smtp-smtps`.

## Retry behaviour

Failed deliveries are retried up to five times with exponential
backoff starting at one second (1s, 2s, 4s, 8s). After the final
attempt a warning is logged.

## Test endpoint

POST `/admin/mail/test` sends a test message to the configured
`ARENA_SMTP_FROM` address. The endpoint responds with JSON indicating
whether the message was queued, for example:

```json
{ "queued": true }
```

A `queued` value of `false` means the message could not be queued.

## Sample configuration

```bash
export ARENA_SMTP_HOST=smtp.example.com
export ARENA_SMTP_PORT=587
export ARENA_SMTP_FROM=arena@example.com
export ARENA_SMTP_STARTTLS=always
export ARENA_SMTP_USER=mailuser
export ARENA_SMTP_PASS=secret
export ARENA_SMTP_TIMEOUT_MS=20000
cargo run -p server
```

## Troubleshooting

- **Connection refused** – verify `ARENA_SMTP_HOST` and `ARENA_SMTP_PORT`
  and that the server allows connections from your host.
- **TLS handshake failures** – ensure STARTTLS/SMTPS settings match the
  server's requirements and that system CA certificates are up to date.
- **Authentication required** – ensure `ARENA_SMTP_USER` and
  `ARENA_SMTP_PASS` are set correctly.
- **Persistent failures** – check server logs for retry warnings and use
  `/admin/mail/test` to verify connectivity.
