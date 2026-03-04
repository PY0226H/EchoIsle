function toNumber(value, fallback = 0) {
  const n = Number(value);
  return Number.isFinite(n) ? n : fallback;
}

export const DEFAULT_OBSERVABILITY_THRESHOLDS = {
  lowSuccessRateThreshold: 80,
  highRetryThreshold: 1,
  highCoalescedThreshold: 2,
  highDbLatencyThresholdMs: 1200,
  lowCacheHitRateThreshold: 20,
  minRequestForCacheHitCheck: 20,
};

function clampFloat(value, min, max, fallback) {
  const n = Number(value);
  if (!Number.isFinite(n)) {
    return fallback;
  }
  return Math.min(max, Math.max(min, n));
}

function clampInt(value, min, max, fallback) {
  const n = Number(value);
  if (!Number.isFinite(n)) {
    return fallback;
  }
  const v = Math.trunc(n);
  return Math.min(max, Math.max(min, v));
}

export function normalizeObservabilityThresholds(input = {}) {
  return {
    lowSuccessRateThreshold: clampFloat(
      input.lowSuccessRateThreshold,
      1,
      99.99,
      DEFAULT_OBSERVABILITY_THRESHOLDS.lowSuccessRateThreshold,
    ),
    highRetryThreshold: clampFloat(
      input.highRetryThreshold,
      0.1,
      10,
      DEFAULT_OBSERVABILITY_THRESHOLDS.highRetryThreshold,
    ),
    highCoalescedThreshold: clampFloat(
      input.highCoalescedThreshold,
      0.1,
      20,
      DEFAULT_OBSERVABILITY_THRESHOLDS.highCoalescedThreshold,
    ),
    highDbLatencyThresholdMs: clampInt(
      input.highDbLatencyThresholdMs,
      1,
      60_000,
      DEFAULT_OBSERVABILITY_THRESHOLDS.highDbLatencyThresholdMs,
    ),
    lowCacheHitRateThreshold: clampFloat(
      input.lowCacheHitRateThreshold,
      0,
      99.99,
      DEFAULT_OBSERVABILITY_THRESHOLDS.lowCacheHitRateThreshold,
    ),
    minRequestForCacheHitCheck: clampInt(
      input.minRequestForCacheHitCheck,
      1,
      1_000_000,
      DEFAULT_OBSERVABILITY_THRESHOLDS.minRequestForCacheHitCheck,
    ),
  };
}

function topRowsByMetric(rows, predicate, limit = 3) {
  return rows
    .filter((row) => predicate(row))
    .slice(0, limit)
    .map((row) => ({
      sessionId: String(row?.debateSessionId || '-'),
      source: String(row?.sourceEventType || '-'),
      label: `session#${String(row?.debateSessionId || '-')}/${String(row?.sourceEventType || '-')}`,
    }));
}

function pickSessionIds(rows = [], limit = 3) {
  const ids = [];
  rows.forEach((item) => {
    const normalized = normalizeObservabilitySessionId(item?.sessionId);
    if (!normalized) {
      return;
    }
    if (!ids.includes(normalized)) {
      ids.push(normalized);
    }
  });
  return ids.slice(0, limit);
}

export function buildJudgeObservabilityAnomalies(
  { rows = [], metrics = {} } = {},
  inputThresholds = {},
) {
  const {
    lowSuccessRateThreshold,
    highRetryThreshold,
    highCoalescedThreshold,
    highDbLatencyThresholdMs,
    lowCacheHitRateThreshold,
    minRequestForCacheHitCheck,
  } = normalizeObservabilityThresholds(inputThresholds);
  const normalizedRows = Array.isArray(rows) ? rows : [];
  const anomalies = [];

  if (normalizedRows.length === 0) {
    anomalies.push({
      level: 'warning',
      code: 'summary_empty',
      text: '当前窗口暂无裁判刷新汇总数据，请确认 analytics 事件链路是否正常上报。',
      action: 'refresh_summary',
      sessionIds: [],
    });
  }

  const lowSuccessRows = topRowsByMetric(
    normalizedRows,
    (row) => toNumber(row?.totalRuns, 0) >= 3 && toNumber(row?.successRate, 100) < lowSuccessRateThreshold,
  );
  if (lowSuccessRows.length > 0) {
    anomalies.push({
      level: 'danger',
      code: 'low_success_rate',
      text: `存在低成功率刷新链路：${lowSuccessRows.map((item) => item.label).join(', ')}`,
      action: 'review_sessions',
      sessionIds: pickSessionIds(lowSuccessRows),
    });
  }

  const highRetryRows = topRowsByMetric(
    normalizedRows,
    (row) => toNumber(row?.totalRuns, 0) >= 3 && toNumber(row?.avgRetryCount, 0) > highRetryThreshold,
  );
  if (highRetryRows.length > 0) {
    anomalies.push({
      level: 'warning',
      code: 'high_retry',
      text: `存在高重试链路：${highRetryRows.map((item) => item.label).join(', ')}`,
      action: 'review_sessions',
      sessionIds: pickSessionIds(highRetryRows),
    });
  }

  const highCoalescedRows = topRowsByMetric(
    normalizedRows,
    (row) => toNumber(row?.totalRuns, 0) >= 3
      && toNumber(row?.avgCoalescedEvents, 0) > highCoalescedThreshold,
  );
  if (highCoalescedRows.length > 0) {
    anomalies.push({
      level: 'warning',
      code: 'high_coalesced',
      text: `存在高合并事件链路：${highCoalescedRows.map((item) => item.label).join(', ')}`,
      action: 'review_sessions',
      sessionIds: pickSessionIds(highCoalescedRows),
    });
  }

  const dbErrorTotal = toNumber(metrics?.dbErrorTotal, 0);
  if (dbErrorTotal > 0) {
    anomalies.push({
      level: 'danger',
      code: 'db_errors',
      text: `summary 查询出现 DB 错误 ${dbErrorTotal} 次，请优先排查 analytics 查询链路。`,
      action: 'refresh_metrics',
      sessionIds: [],
    });
  }

  const avgDbLatencyMs = toNumber(metrics?.avgDbLatencyMs, 0);
  if (avgDbLatencyMs > highDbLatencyThresholdMs) {
    anomalies.push({
      level: 'warning',
      code: 'high_db_latency',
      text: `summary 平均 DB 延迟 ${avgDbLatencyMs.toFixed(2)}ms，已超过阈值 ${highDbLatencyThresholdMs}ms。`,
      action: 'refresh_metrics',
      sessionIds: [],
    });
  }

  const requestTotal = toNumber(metrics?.requestTotal, 0);
  const cacheHitRate = toNumber(metrics?.cacheHitRate, 0);
  if (requestTotal >= minRequestForCacheHitCheck && cacheHitRate < lowCacheHitRateThreshold) {
    anomalies.push({
      level: 'warning',
      code: 'low_cache_hit_rate',
      text: `summary 缓存命中率 ${cacheHitRate.toFixed(2)}% 偏低，请关注查询压力与缓存策略。`,
      action: 'refresh_summary',
      sessionIds: [],
    });
  }

  return anomalies;
}

export function normalizeObservabilitySessionId(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) {
    return 0;
  }
  const id = Math.trunc(n);
  if (id < 1 || id > Number.MAX_SAFE_INTEGER) {
    return 0;
  }
  return id;
}
