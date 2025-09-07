# Purchases

Arena supports optional in-app purchases for cosmetics and modules.

## Configuration

| Env var                     | CLI flag                | Description                        | Default |
| --------------------------- | ----------------------- | ---------------------------------- | ------- |
| `ARENA_PURCHASE_VERIFY_URL` | `--purchase-verify-url` | Endpoint for receipt verification  | -       |
| `ARENA_PURCHASE_PUBLIC_KEY` | `--purchase-public-key` | Public key for validating receipts | -       |
| `ARENA_PURCHASE_TIMEOUT_MS` | `--purchase-timeout-ms` | Verification request timeout       | `5000`  |

## Usage

When a client completes a transaction, it sends the receipt to the server.
The server verifies the receipt and unlocks the item:

```bash
curl -X POST https://server/purchases/verify -d @receipt.json
```

## Integration

Purchase handling is implemented in the `platform-api` crate. Register
`PurchasesPlugin` on the server to accept verification requests and
update player inventory. The client invokes the platform SDK and forwards
receipts via the `platform-api` utilities.
