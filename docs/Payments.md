# Payments

The payments service provides a simple SKU catalog and integrates with Stripe
Checkout. Configure the service with the following environment variables:

- `STRIPE_PUBLIC_KEY` – publishable key for Checkout sessions.
- `STRIPE_SECRET_KEY` – secret API key used by the server.
- `WEBHOOK_SECRET` – secret used to verify webhook callbacks.

Server routes:

- `POST /payments/checkout` – initiate a new purchase, returning a Checkout URL.
- `POST /payments/webhook` – Stripe webhook that grants entitlements after
  confirmation.

Entitlements are persisted via an in-memory store by default. Replace the
implementation in `crates/payments` to hook up a database in production.
