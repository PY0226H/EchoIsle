# iap-storekit-bridge

Command-line native bridge for Tauri `purchase_mode=native`.

## Purpose

`chatapp/src-tauri` calls an external command and expects JSON payload:

```json
{
  "productId": "com.acme.coins.100",
  "transactionId": "1234567890",
  "originalTransactionId": "1234567890",
  "receiptData": "<base64_receipt>",
  "source": "iap_storekit_bridge_storekit2"
}
```

This package provides that bridge with StoreKit2 purchase flow.

## Build

```bash
cd /Users/panyihang/Documents/aicomm/chatapp/native/iap-storekit-bridge
xcrun swift build -c release
```

Binary path:

`/Users/panyihang/Documents/aicomm/chatapp/native/iap-storekit-bridge/.build/release/iap-storekit-bridge`

Quick launcher (auto build when missing):

`/Users/panyihang/Documents/aicomm/chatapp/native/iap-storekit-bridge/run.sh`

## Run

Real purchase:

```bash
./.build/release/iap-storekit-bridge --product-id com.acme.coins.100
```

Simulated payload (for local debug / CI):

```bash
./.build/release/iap-storekit-bridge --product-id com.acme.coins.100 --simulate
```

Or by env:

```bash
AICOMM_IAP_SIMULATE=1 ./.build/release/iap-storekit-bridge --product-id com.acme.coins.100
```

## Receipt behavior

- If `AICOMM_IAP_RECEIPT_B64` is set and non-empty, bridge uses it directly.
- Otherwise bridge tries `Bundle.main.appStoreReceiptURL` and base64 encodes the file.

## Tauri config snippet

`app.yml`:

```yaml
iap:
  purchase_mode: "native"
  native_bridge:
    bin: "/Users/panyihang/Documents/aicomm/chatapp/native/iap-storekit-bridge/run.sh"
    args: []
```
