import { invoke } from '@tauri-apps/api/core';

export function isTauriRuntime() {
  if (typeof window === 'undefined' || !window) {
    return false;
  }
  return Boolean(window.__TAURI_INTERNALS__ || window.__TAURI__);
}

export function normalizeIapPurchasePayload(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    throw new Error('invalid iap purchase payload');
  }
  const productId = String(payload.productId || payload.product_id || '').trim();
  const transactionId = String(payload.transactionId || payload.transaction_id || '').trim();
  const receiptData = String(payload.receiptData || payload.receipt_data || '').trim();
  const originalTransactionIdRaw = payload.originalTransactionId ?? payload.original_transaction_id;
  const source = String(payload.source || '').trim() || 'tauri';
  if (!productId || !transactionId || !receiptData) {
    throw new Error('iap purchase payload missing required fields');
  }
  return {
    productId,
    transactionId,
    originalTransactionId: originalTransactionIdRaw == null
      ? null
      : String(originalTransactionIdRaw).trim() || null,
    receiptData,
    source,
  };
}

export function parseNativeBridgeErrorMessage(message) {
  const raw = String(message || '').trim();
  const prefix = 'native_iap_bridge_error:';
  if (!raw.startsWith(prefix)) {
    return null;
  }
  const content = raw.slice(prefix.length);
  const sep = content.indexOf(':');
  if (sep <= 0) {
    return null;
  }
  const code = content.slice(0, sep).trim();
  const errorMessage = content.slice(sep + 1).trim();
  if (!code || !errorMessage) {
    return null;
  }
  return { code, message: errorMessage };
}

export function normalizeIapNativeBridgeDiagnostics(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    throw new Error('invalid iap native bridge diagnostics payload');
  }
  return {
    runtimeEnv: payload.runtimeEnv ?? payload.runtime_env ?? null,
    runtimeIsProduction: Boolean(payload.runtimeIsProduction ?? payload.runtime_is_production),
    purchaseMode: String(payload.purchaseMode ?? payload.purchase_mode ?? '').trim() || 'unknown',
    allowedProductIds: Array.isArray(payload.allowedProductIds ?? payload.allowed_product_ids)
      ? (payload.allowedProductIds ?? payload.allowed_product_ids).map((v) => String(v))
      : [],
    invalidAllowedProductIds: Array.isArray(
      payload.invalidAllowedProductIds ?? payload.invalid_allowed_product_ids,
    )
      ? (payload.invalidAllowedProductIds ?? payload.invalid_allowed_product_ids).map((v) => String(v))
      : [],
    allowedProductIdsConfigured: Boolean(
      payload.allowedProductIdsConfigured ?? payload.allowed_product_ids_configured,
    ),
    nativeBridgeBin: String(payload.nativeBridgeBin ?? payload.native_bridge_bin ?? '').trim(),
    nativeBridgeArgs: Array.isArray(payload.nativeBridgeArgs ?? payload.native_bridge_args)
      ? (payload.nativeBridgeArgs ?? payload.native_bridge_args).map((v) => String(v))
      : [],
    nativeBridgeBinIsAbsolute: Boolean(
      payload.nativeBridgeBinIsAbsolute ?? payload.native_bridge_bin_is_absolute,
    ),
    nativeBridgeBinExists: Boolean(payload.nativeBridgeBinExists ?? payload.native_bridge_bin_exists),
    nativeBridgeBinExecutable: Boolean(
      payload.nativeBridgeBinExecutable ?? payload.native_bridge_bin_executable,
    ),
    hasSimulateArg: Boolean(payload.hasSimulateArg ?? payload.has_simulate_arg),
    jsonOverridePresent: Boolean(payload.jsonOverridePresent ?? payload.json_override_present),
    productionPolicyOk: Boolean(payload.productionPolicyOk ?? payload.production_policy_ok),
    productionPolicyError: String(
      payload.productionPolicyError ?? payload.production_policy_error ?? '',
    ).trim() || null,
    readyForNativePurchase: Boolean(
      payload.readyForNativePurchase ?? payload.ready_for_native_purchase,
    ),
  };
}

export async function getIapNativeBridgeDiagnostics() {
  if (!isTauriRuntime()) {
    throw new Error('tauri runtime is unavailable');
  }
  const payload = await invoke('iap_get_native_bridge_diagnostics');
  return normalizeIapNativeBridgeDiagnostics(payload);
}

export async function purchaseIapViaTauri(productId) {
  if (!isTauriRuntime()) {
    throw new Error('tauri runtime is unavailable');
  }
  const normalizedProductId = String(productId || '').trim();
  if (!normalizedProductId) {
    throw new Error('productId is required');
  }
  try {
    const payload = await invoke('iap_purchase_product', { productId: normalizedProductId });
    return normalizeIapPurchasePayload(payload);
  } catch (error) {
    const message = String(error?.message || error || '').trim();
    const parsed = parseNativeBridgeErrorMessage(message);
    if (parsed) {
      const wrapped = new Error(parsed.message);
      wrapped.code = parsed.code;
      throw wrapped;
    }
    throw error;
  }
}
