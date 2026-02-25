const DEFAULT_NOTIFY_BASE = 'http://localhost:6687/events';

function toWsProtocol(protocol) {
  if (protocol === 'https:') {
    return 'wss:';
  }
  if (protocol === 'http:') {
    return 'ws:';
  }
  if (protocol === 'wss:' || protocol === 'ws:') {
    return protocol;
  }
  return 'ws:';
}

function normalizeNotifyBasePath(pathname) {
  if (!pathname || pathname === '/') {
    return '';
  }
  const trimmed = pathname.endsWith('/') ? pathname.slice(0, -1) : pathname;
  if (trimmed.endsWith('/events')) {
    return trimmed.slice(0, -'/events'.length);
  }
  return trimmed;
}

export function buildDebateRoomWsUrl({ notifyBase, sessionId, notifyTicket }) {
  if (!sessionId) {
    throw new Error('sessionId is required');
  }
  if (!notifyTicket || !String(notifyTicket).trim()) {
    throw new Error('notifyTicket is required');
  }
  const base = String(notifyBase || DEFAULT_NOTIFY_BASE);
  const parsed = new URL(base);
  parsed.protocol = toWsProtocol(parsed.protocol);
  const basePath = normalizeNotifyBasePath(parsed.pathname);
  parsed.pathname = `${basePath}/ws/debate/${sessionId}`;
  parsed.search = '';
  parsed.searchParams.set('token', String(notifyTicket).trim());
  return parsed.toString();
}

export function parseDebateRoomWsMessage(raw) {
  if (typeof raw !== 'string' || raw.trim() === '') {
    return null;
  }
  try {
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return null;
    }
    const type = typeof parsed.type === 'string' ? parsed.type : '';
    if (!type) {
      return null;
    }
    return parsed;
  } catch (_) {
    return null;
  }
}

export function extractDebateRoomEvent(message, expectedEventName = '') {
  if (!message || message.type !== 'roomEvent') {
    return null;
  }
  if (typeof message.eventName !== 'string' || !message.eventName) {
    return null;
  }
  if (expectedEventName && message.eventName !== expectedEventName) {
    return null;
  }
  const payload = message.payload;
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return null;
  }
  return payload;
}
