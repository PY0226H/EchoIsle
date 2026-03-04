#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT="$ROOT/scripts/release/appstore_preflight_check.sh"

if [[ ! -x "$SCRIPT" ]]; then
  chmod +x "$SCRIPT"
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

expect_fail() {
  local name="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "[FAIL] $name: expected failure but passed"
    exit 1
  fi
  echo "[PASS] $name"
}

expect_pass() {
  local name="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "[PASS] $name"
    return
  fi
  echo "[FAIL] $name: expected pass but failed"
  exit 1
}

VALID_CHAT="$TMP_DIR/chat.valid.yml"
VALID_TAURI="$TMP_DIR/app.valid.yml"
VALID_AI_ENV="$TMP_DIR/ai.valid.env"

cat >"$VALID_CHAT" <<'EOF'
payment:
  verify_mode: apple
  apple_verify_url_prod: https://buy.itunes.apple.com/verifyReceipt
  apple_verify_url_sandbox: https://sandbox.itunes.apple.com/verifyReceipt
EOF

cat >"$VALID_TAURI" <<'EOF'
iap:
  purchase_mode: native
  allowed_product_ids:
    - com.aicomm.coins.60
    - com.aicomm.coins.120
  native_bridge:
    bin: /usr/local/bin/iap-storekit-bridge
    args: []
EOF

cat >"$VALID_AI_ENV" <<'EOF'
AI_JUDGE_PROVIDER=openai
AI_JUDGE_OPENAI_FALLBACK_TO_MOCK=false
OPENAI_API_KEY=sk-test
EOF

expect_pass "valid production config should pass" \
  "$SCRIPT" \
  --runtime-env production \
  --chat-config "$VALID_CHAT" \
  --tauri-app-config "$VALID_TAURI" \
  --ai-judge-env "$VALID_AI_ENV"

BAD_CHAT_MOCK="$TMP_DIR/chat.mock.yml"
cat >"$BAD_CHAT_MOCK" <<'EOF'
payment:
  verify_mode: mock
  apple_verify_url_prod: https://buy.itunes.apple.com/verifyReceipt
  apple_verify_url_sandbox: https://sandbox.itunes.apple.com/verifyReceipt
EOF

expect_fail "payment mock in production should fail" \
  "$SCRIPT" \
  --runtime-env production \
  --chat-config "$BAD_CHAT_MOCK" \
  --tauri-app-config "$VALID_TAURI" \
  --ai-judge-env "$VALID_AI_ENV"

BAD_TAURI_SIMULATE="$TMP_DIR/app.simulate.yml"
cat >"$BAD_TAURI_SIMULATE" <<'EOF'
iap:
  purchase_mode: native
  allowed_product_ids:
    - com.aicomm.coins.60
  native_bridge:
    bin: /usr/local/bin/iap-storekit-bridge
    args:
      - --simulate
EOF

expect_fail "tauri simulate arg in production should fail" \
  "$SCRIPT" \
  --runtime-env production \
  --chat-config "$VALID_CHAT" \
  --tauri-app-config "$BAD_TAURI_SIMULATE" \
  --ai-judge-env "$VALID_AI_ENV"

BAD_AI_MOCK="$TMP_DIR/ai.mock.env"
cat >"$BAD_AI_MOCK" <<'EOF'
AI_JUDGE_PROVIDER=mock
AI_JUDGE_OPENAI_FALLBACK_TO_MOCK=false
OPENAI_API_KEY=
EOF

expect_fail "ai_judge provider mock in production should fail" \
  "$SCRIPT" \
  --runtime-env production \
  --chat-config "$VALID_CHAT" \
  --tauri-app-config "$VALID_TAURI" \
  --ai-judge-env "$BAD_AI_MOCK"

echo "all preflight tests passed"
