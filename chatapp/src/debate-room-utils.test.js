import assert from 'node:assert/strict';
import {
  buildDebateRoomWsUrl,
  extractDebateRoomEvent,
  normalizeJudgeReportStatus,
  parseDebateRoomWsMessage,
  shouldPollJudgeReportStatus,
} from './debate-room-utils.js';

const wsUrl = buildDebateRoomWsUrl({
  notifyBase: 'http://localhost:6687/events',
  sessionId: 123,
  notifyTicket: 'abc',
});
assert.equal(wsUrl, 'ws://localhost:6687/ws/debate/123?token=abc');

const wsUrlWithPrefix = buildDebateRoomWsUrl({
  notifyBase: 'https://example.com/notify/events',
  sessionId: 88,
  notifyTicket: 't-1',
});
assert.equal(wsUrlWithPrefix, 'wss://example.com/notify/ws/debate/88?token=t-1');

assert.throws(
  () => buildDebateRoomWsUrl({ notifyBase: 'http://localhost:6687/events', sessionId: 1 }),
  /notifyTicket is required/,
);

const welcome = parseDebateRoomWsMessage(
  JSON.stringify({ type: 'welcome', sessionId: 12, userId: 7 }),
);
assert.deepEqual(welcome, { type: 'welcome', sessionId: 12, userId: 7 });

const roomEvent = parseDebateRoomWsMessage(
  JSON.stringify({
    type: 'roomEvent',
    eventName: 'DebateMessageCreated',
    payload: { event: 'DebateMessageCreated', messageId: 9, sessionId: 12 },
  }),
);
assert.equal(roomEvent?.type, 'roomEvent');
assert.deepEqual(extractDebateRoomEvent(roomEvent), {
  event: 'DebateMessageCreated',
  messageId: 9,
  sessionId: 12,
});
assert.deepEqual(extractDebateRoomEvent(roomEvent, 'DebateMessageCreated'), {
  event: 'DebateMessageCreated',
  messageId: 9,
  sessionId: 12,
});
assert.equal(extractDebateRoomEvent(roomEvent, 'DebateMessagePinned'), null);

assert.equal(parseDebateRoomWsMessage('{'), null);
assert.equal(parseDebateRoomWsMessage(''), null);
assert.equal(parseDebateRoomWsMessage('[]'), null);

assert.equal(normalizeJudgeReportStatus('ready'), 'ready');
assert.equal(normalizeJudgeReportStatus('PENDING'), 'pending');
assert.equal(normalizeJudgeReportStatus('unknown-status'), 'absent');
assert.equal(normalizeJudgeReportStatus(null), 'absent');
assert.equal(shouldPollJudgeReportStatus('pending'), true);
assert.equal(shouldPollJudgeReportStatus('ready'), false);
