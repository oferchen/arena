# Purchases

Arena includes a lightweight payment system backed by Stripe Checkout. The
server exposes a catalog of stock keeping units (SKUs), endpoints to initiate a
purchase, and a webhook to grant entitlements when Stripe reports a completed
checkout session.

## Catalog

Available items are returned from the `/store` endpoint.

```bash
curl http://localhost:3000/store
```

## Purchase Flow

1. The client posts the desired SKU to `/purchase/start`:
   ```bash
   curl -X POST http://localhost:3000/purchase/start \
        -d '{"user":"alice","sku":"duck_hunt"}' \
        -H 'Content-Type: application/json'
   ```
   The server responds with a Stripe Checkout session identifier.
2. The client completes payment using Stripe's SDK.
3. Stripe notifies the server via the `checkout.session.completed` webhook at
   `/stripe/webhook`.
4. The server grants the entitlement, persists it to `entitlements.json`, and
   emits analytics events for the successful purchase and entitlement grant.

## Entitlements

Clients can query granted entitlements using `/entitlements/<user>` and gate
features locally based on the response.

```bash
curl http://localhost:3000/entitlements/alice
```

Analytics events are also emitted for store views and purchase initiation.
