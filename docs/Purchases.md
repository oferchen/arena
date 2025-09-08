# Purchases

Arena uses an OTP-based entitlement system. After a user signs in with a one-time
password, the client can claim items directly from the server.

## Catalog

Available items are returned from the `/store` endpoint.

```bash
curl http://localhost:3000/store
```

## Claim Flow

1. The client authenticates via OTP and receives a session identifier.
2. The client posts the desired SKU to `/store/claim`:
   ```bash
   curl -X POST http://localhost:3000/store/claim \
        -H 'X-Session: <session-token>' \
        -H 'Content-Type: application/json' \
        -d '{"sku":"duck_hunt"}'
   ```
   The server verifies the session and records the entitlement in its
   Scylla-backed store.
3. The entitlement is persisted server-side and can be queried later.

## Entitlements

Clients can query granted entitlements using `/entitlements/<user>` and gate
features locally based on the response.
