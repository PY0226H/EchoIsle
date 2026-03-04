import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildJudgeObservabilityAnomalies,
  normalizeObservabilitySessionId,
} from './judge-observability-utils.js';

test('normalizeObservabilitySessionId should normalize valid id', () => {
  assert.equal(normalizeObservabilitySessionId('42'), 42);
  assert.equal(normalizeObservabilitySessionId(0), 0);
  assert.equal(normalizeObservabilitySessionId('x'), 0);
});

test('buildJudgeObservabilityAnomalies should report summary empty', () => {
  const ret = buildJudgeObservabilityAnomalies({ rows: [], metrics: {} });
  assert.equal(ret.some((item) => item.code === 'summary_empty'), true);
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
