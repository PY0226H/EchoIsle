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

function normalizeTimestampMs(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) {
    return 0;
  }
  const ts = Math.trunc(n);
  return ts > 0 ? ts : 0;
}

export function buildObservabilityAnomalyStateKey(anomaly = {}) {
  const code = String(anomaly?.code || '').trim();
  const safeCode = code || 'unknown';
  const sessionIds = [];
  if (Array.isArray(anomaly?.sessionIds)) {
    anomaly.sessionIds.forEach((value) => {
      const sessionId = normalizeObservabilitySessionId(value);
      if (sessionId <= 0 || sessionIds.includes(sessionId)) {
        return;
      }
      sessionIds.push(sessionId);
    });
    sessionIds.sort((a, b) => a - b);
  }
  if (sessionIds.length === 0) {
    return safeCode;
  }
  return `${safeCode}:${sessionIds.join(',')}`;
}

export function normalizeObservabilityAnomalyStateMap(input = {}, nowMs = Date.now()) {
  const source = input && typeof input === 'object' ? input : {};
  const now = normalizeTimestampMs(nowMs) || Date.now();
  const normalized = {};
  Object.entries(source).forEach(([key, raw]) => {
    const stateKey = String(key || '').trim();
    if (!stateKey) {
      return;
    }
    const item = raw && typeof raw === 'object' ? raw : {};
    const acknowledgedAtMs = normalizeTimestampMs(item.acknowledgedAtMs);
    const suppressUntilRaw = normalizeTimestampMs(item.suppressUntilMs);
    const suppressUntilMs = suppressUntilRaw > now ? suppressUntilRaw : 0;
    if (!acknowledgedAtMs && !suppressUntilMs) {
      return;
    }
    normalized[stateKey] = {
      acknowledgedAtMs,
      suppressUntilMs,
    };
  });
  return normalized;
}

export function projectObservabilityAnomalies(anomalies = [], state = {}, nowMs = Date.now()) {
  const now = normalizeTimestampMs(nowMs) || Date.now();
  const normalizedState = normalizeObservabilityAnomalyStateMap(state, now);
  const source = Array.isArray(anomalies) ? anomalies : [];
  const all = source.map((anomaly) => {
    const key = buildObservabilityAnomalyStateKey(anomaly);
    const item = normalizedState[key] || {};
    const suppressUntilMs = normalizeTimestampMs(item.suppressUntilMs);
    const acknowledgedAtMs = normalizeTimestampMs(item.acknowledgedAtMs);
    const suppressed = suppressUntilMs > now;
    return {
      ...anomaly,
      stateKey: key,
      acknowledgedAtMs,
      suppressUntilMs,
      suppressed,
    };
  });
  const visible = all.filter((item) => !item.suppressed);
  return {
    all,
    visible,
    suppressedCount: all.length - visible.length,
    state: normalizedState,
  };
}

export const OBSERVABILITY_TREND_WINDOW_MS = 48 * 60 * 60 * 1000;
export const OBSERVABILITY_TREND_MAX_POINTS = 1000;

export function buildObservabilityAnomalyCodeStats(anomalies = []) {
  const source = Array.isArray(anomalies) ? anomalies : [];
  const counts = {};
  source.forEach((anomaly) => {
    const code = String(anomaly?.code || '').trim() || 'unknown';
    counts[code] = Number(counts[code] || 0) + 1;
  });
  const rows = Object.entries(counts)
    .map(([code, count]) => ({
      code,
      count: Number(count || 0),
    }))
    .sort((a, b) => {
      if (b.count !== a.count) {
        return b.count - a.count;
      }
      return a.code.localeCompare(b.code);
    });
  return {
    total: source.length,
    counts,
    rows,
  };
}

function normalizeTrendCounts(input = {}) {
  const source = input && typeof input === 'object' ? input : {};
  const counts = {};
  Object.entries(source).forEach(([codeRaw, valueRaw]) => {
    const code = String(codeRaw || '').trim();
    if (!code) {
      return;
    }
    const n = Number(valueRaw);
    if (!Number.isFinite(n)) {
      return;
    }
    const value = Math.max(0, Math.trunc(n));
    if (value > 0) {
      counts[code] = value;
    }
  });
  return counts;
}

export function normalizeObservabilityAnomalyTrendHistory(
  input = [],
  nowMs = Date.now(),
  windowMs = OBSERVABILITY_TREND_WINDOW_MS,
  maxPoints = OBSERVABILITY_TREND_MAX_POINTS,
) {
  const source = Array.isArray(input) ? input : [];
  const now = normalizeTimestampMs(nowMs) || Date.now();
  const safeWindowMs = Math.max(1, Math.trunc(Number(windowMs) || OBSERVABILITY_TREND_WINDOW_MS));
  const safeMaxPoints = Math.max(1, Math.trunc(Number(maxPoints) || OBSERVABILITY_TREND_MAX_POINTS));
  const lowerBound = now - safeWindowMs;
  const items = [];
  source.forEach((entry) => {
    const row = entry && typeof entry === 'object' ? entry : {};
    const atMs = normalizeTimestampMs(row.atMs);
    if (!atMs || atMs < lowerBound || atMs > now) {
      return;
    }
    items.push({
      atMs,
      counts: normalizeTrendCounts(row.counts),
    });
  });
  items.sort((a, b) => a.atMs - b.atMs);
  if (items.length > safeMaxPoints) {
    return items.slice(items.length - safeMaxPoints);
  }
  return items;
}

function mergeTrendTotals(target = {}, counts = {}) {
  Object.entries(counts).forEach(([code, value]) => {
    const n = Number(value || 0);
    if (n <= 0) {
      return;
    }
    target[code] = Number(target[code] || 0) + n;
  });
}

export function appendObservabilityAnomalyTrendSnapshot(
  history = [],
  anomalies = [],
  nowMs = Date.now(),
) {
  const atMs = normalizeTimestampMs(nowMs) || Date.now();
  const normalizedHistory = normalizeObservabilityAnomalyTrendHistory(history, atMs);
  const stats = buildObservabilityAnomalyCodeStats(anomalies);
  const next = [
    ...normalizedHistory,
    {
      atMs,
      counts: stats.counts,
    },
  ];
  return normalizeObservabilityAnomalyTrendHistory(next, atMs);
}

export function summarizeObservabilityAnomalyTrend(history = [], nowMs = Date.now()) {
  const now = normalizeTimestampMs(nowMs) || Date.now();
  const normalizedHistory = normalizeObservabilityAnomalyTrendHistory(history, now);
  const recentStartMs = now - (24 * 60 * 60 * 1000);
  const previousStartMs = now - (48 * 60 * 60 * 1000);
  let recentSamples = 0;
  let previousSamples = 0;
  const recentTotals = {};
  const previousTotals = {};

  normalizedHistory.forEach((entry) => {
    const atMs = normalizeTimestampMs(entry?.atMs);
    if (atMs >= recentStartMs && atMs <= now) {
      recentSamples += 1;
      mergeTrendTotals(recentTotals, entry?.counts || {});
      return;
    }
    if (atMs >= previousStartMs && atMs < recentStartMs) {
      previousSamples += 1;
      mergeTrendTotals(previousTotals, entry?.counts || {});
    }
  });

  const codeSet = new Set([
    ...Object.keys(recentTotals),
    ...Object.keys(previousTotals),
  ]);
  const rows = Array.from(codeSet)
    .map((code) => {
      const recentTotal = Number(recentTotals[code] || 0);
      const previousTotal = Number(previousTotals[code] || 0);
      const recentAvg = recentSamples > 0 ? recentTotal / recentSamples : 0;
      const previousAvg = previousSamples > 0 ? previousTotal / previousSamples : 0;
      const delta = recentAvg - previousAvg;
      let trend = 'flat';
      if (delta > 0.001) {
        trend = 'up';
      } else if (delta < -0.001) {
        trend = 'down';
      }
      return {
        code,
        recentTotal,
        previousTotal,
        recentAvg,
        previousAvg,
        delta,
        trend,
      };
    })
    .sort((a, b) => {
      if (b.recentAvg !== a.recentAvg) {
        return b.recentAvg - a.recentAvg;
      }
      if (Math.abs(b.delta) !== Math.abs(a.delta)) {
        return Math.abs(b.delta) - Math.abs(a.delta);
      }
      return a.code.localeCompare(b.code);
    });

  const latestAtMs = normalizedHistory.length > 0
    ? normalizedHistory[normalizedHistory.length - 1].atMs
    : 0;

  return {
    latestAtMs,
    historyCount: normalizedHistory.length,
    recentSamples,
    previousSamples,
    rows,
  };
}
