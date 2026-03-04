import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  DEFAULT_OBSERVABILITY_THRESHOLDS,
  buildJudgeObservabilityAnomalies,
  normalizeObservabilitySessionId,
  normalizeObservabilityThresholds,
} from './judge-observability-utils.js';

test('normalizeObservabilitySessionId should normalize valid id', () => {
  assert.equal(normalizeObservabilitySessionId('42'), 42);
  assert.equal(normalizeObservabilitySessionId(0), 0);
  assert.equal(normalizeObservabilitySessionId('x'), 0);
});

test('buildJudgeObservabilityAnomalies should report summary empty', () => {
  const ret = buildJudgeObservabilityAnomalies({ rows: [], metrics: {} });
  assert.equal(ret.some((item) => item.code === 'summary_empty'), true);
  const anomaly = ret.find((item) => item.code === 'summary_empty');
  assert.equal(anomaly.action, 'refresh_summary');
});

test('buildJudgeObservabilityAnomalies should report low success and retries', () => {
  const ret = buildJudgeObservabilityAnomalies({
    rows: [
      {
        debateSessionId: '8',
        sourceEventType: 'DebateJudgeReportReady',
        totalRuns: 12,
        successRate: 66.5,
        avgRetryCount: 1.7,
        avgCoalescedEvents: 2.6,
      },
    ],
    metrics: {
      requestTotal: 50,
      cacheHitRate: 15,
      dbErrorTotal: 3,
      avgDbLatencyMs: 1550,
    },
  });

  const codes = ret.map((item) => item.code);
  assert.equal(codes.includes('low_success_rate'), true);
  assert.equal(codes.includes('high_retry'), true);
  assert.equal(codes.includes('high_coalesced'), true);
  assert.equal(codes.includes('db_errors'), true);
  assert.equal(codes.includes('high_db_latency'), true);
  assert.equal(codes.includes('low_cache_hit_rate'), true);
  const lowSuccess = ret.find((item) => item.code === 'low_success_rate');
  assert.deepEqual(lowSuccess.sessionIds, [8]);
  assert.equal(lowSuccess.action, 'review_sessions');
});

test('buildJudgeObservabilityAnomalies should return empty for healthy rows', () => {
  const ret = buildJudgeObservabilityAnomalies({
    rows: [
      {
        debateSessionId: '100',
        sourceEventType: 'DebateJudgeReportReady',
        totalRuns: 12,
        successRate: 99.2,
        avgRetryCount: 0.1,
        avgCoalescedEvents: 0.3,
      },
    ],
    metrics: {
      requestTotal: 100,
      cacheHitRate: 90,
      dbErrorTotal: 0,
      avgDbLatencyMs: 30,
    },
  });
  assert.deepEqual(ret, []);
});

test('normalizeObservabilityThresholds should fallback and clamp', () => {
  const ret = normalizeObservabilityThresholds({
    lowSuccessRateThreshold: 0,
    highRetryThreshold: 'x',
    highCoalescedThreshold: 99,
    highDbLatencyThresholdMs: -10,
    lowCacheHitRateThreshold: 1000,
    minRequestForCacheHitCheck: 0,
  });
  assert.deepEqual(ret, {
    lowSuccessRateThreshold: 1,
    highRetryThreshold: DEFAULT_OBSERVABILITY_THRESHOLDS.highRetryThreshold,
    highCoalescedThreshold: 20,
    highDbLatencyThresholdMs: 1,
    lowCacheHitRateThreshold: 99.99,
    minRequestForCacheHitCheck: 1,
  });
});
