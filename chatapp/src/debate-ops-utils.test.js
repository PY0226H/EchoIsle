import assert from 'node:assert/strict';
import {
  buildQuickUpdateSessionPayload,
  nextQuickStatusActions,
  normalizeOpsSessionStatus,
} from './debate-ops-utils.js';

assert.equal(normalizeOpsSessionStatus(' running '), 'running');
assert.equal(normalizeOpsSessionStatus('INVALID'), 'scheduled');

assert.deepEqual(nextQuickStatusActions('scheduled'), ['open', 'canceled']);
assert.deepEqual(nextQuickStatusActions('open'), ['running', 'canceled']);
assert.deepEqual(nextQuickStatusActions('running'), ['judging', 'closed', 'canceled']);
assert.deepEqual(nextQuickStatusActions('judging'), ['closed', 'canceled']);
assert.deepEqual(nextQuickStatusActions('closed'), []);

const payload = buildQuickUpdateSessionPayload(
  {
    id: 8,
    maxParticipantsPerSide: 500,
    scheduledStartAt: '2026-02-26T01:00:00.000Z',
    endAt: '2026-02-26T02:00:00.000Z',
  },
  'running',
);
assert.deepEqual(payload, {
  sessionId: 8,
  status: 'running',
  scheduledStartAt: '2026-02-26T01:00:00.000Z',
  endAt: '2026-02-26T02:00:00.000Z',
  maxParticipantsPerSide: 500,
});

assert.throws(
  () => buildQuickUpdateSessionPayload({ id: 0 }, 'open'),
  /invalid session id/,
);

assert.throws(
  () =>
    buildQuickUpdateSessionPayload(
      {
        id: 1,
        maxParticipantsPerSide: 0,
        scheduledStartAt: '2026-02-26T01:00:00.000Z',
        endAt: '2026-02-26T02:00:00.000Z',
      },
      'open',
    ),
  /invalid maxParticipantsPerSide/,
);

assert.throws(
  () =>
    buildQuickUpdateSessionPayload(
      {
        id: 1,
        maxParticipantsPerSide: 200,
        scheduledStartAt: '',
        endAt: '2026-02-26T02:00:00.000Z',
      },
      'open',
    ),
  /missing session schedule/,
);

