function toNumber(value, fallback = 0) {
  const n = Number(value);
  return Number.isFinite(n) ? n : fallback;
}

function topRowsByMetric(rows, predicate, limit = 3) {
  return rows
    .filter((row) => predicate(row))
    .slice(0, limit)
    .map((row) => {
      const sessionId = String(row?.debateSessionId || '-');
      const source = String(row?.sourceEventType || '-');
      return `session#${sessionId}/${source}`;
    });
}

export function buildJudgeObservabilityAnomalies(
  { rows = [], metrics = {} } = {},
  {
    lowSuccessRateThreshold = 80,
    highRetryThreshold = 1,
    highCoalescedThreshold = 2,
    highDbLatencyThresholdMs = 1200,
    lowCacheHitRateThreshold = 20,
    minRequestForCacheHitCheck = 20,
  } = {},
) {
  const normalizedRows = Array.isArray(rows) ? rows : [];
  const anomalies = [];

  if (normalizedRows.length === 0) {
    anomalies.push({
      level: 'warning',
      code: 'summary_empty',
      text: '当前窗口暂无裁判刷新汇总数据，请确认 analytics 事件链路是否正常上报。',
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
      text: `存在低成功率刷新链路：${lowSuccessRows.join(', ')}`,
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
      text: `存在高重试链路：${highRetryRows.join(', ')}`,
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
      text: `存在高合并事件链路：${highCoalescedRows.join(', ')}`,
    });
  }

  const dbErrorTotal = toNumber(metrics?.dbErrorTotal, 0);
  if (dbErrorTotal > 0) {
    anomalies.push({
      level: 'danger',
      code: 'db_errors',
      text: `summary 查询出现 DB 错误 ${dbErrorTotal} 次，请优先排查 analytics 查询链路。`,
    });
  }

  const avgDbLatencyMs = toNumber(metrics?.avgDbLatencyMs, 0);
  if (avgDbLatencyMs > highDbLatencyThresholdMs) {
    anomalies.push({
      level: 'warning',
      code: 'high_db_latency',
      text: `summary 平均 DB 延迟 ${avgDbLatencyMs.toFixed(2)}ms，已超过阈值 ${highDbLatencyThresholdMs}ms。`,
    });
  }

  const requestTotal = toNumber(metrics?.requestTotal, 0);
  const cacheHitRate = toNumber(metrics?.cacheHitRate, 0);
  if (requestTotal >= minRequestForCacheHitCheck && cacheHitRate < lowCacheHitRateThreshold) {
    anomalies.push({
      level: 'warning',
      code: 'low_cache_hit_rate',
      text: `summary 缓存命中率 ${cacheHitRate.toFixed(2)}% 偏低，请关注查询压力与缓存策略。`,
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
