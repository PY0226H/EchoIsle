const STATUS_ORDER = ['scheduled', 'open', 'running', 'judging', 'closed', 'canceled'];

export function normalizeOpsSessionStatus(status) {
  const normalized = String(status || '').trim().toLowerCase();
  if (STATUS_ORDER.includes(normalized)) {
    return normalized;
  }
  return 'scheduled';
}

export function nextQuickStatusActions(currentStatus) {
  const status = normalizeOpsSessionStatus(currentStatus);
  if (status === 'scheduled') {
    return ['open', 'canceled'];
  }
  if (status === 'open') {
    return ['running', 'canceled'];
  }
  if (status === 'running') {
    return ['judging', 'closed', 'canceled'];
  }
  if (status === 'judging') {
    return ['closed', 'canceled'];
  }
  return [];
}

export function buildQuickUpdateSessionPayload(session, nextStatus) {
  const id = Number(session?.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error('invalid session id');
  }

  const targetStatus = normalizeOpsSessionStatus(nextStatus);
  const maxParticipantsPerSide = Number(session?.maxParticipantsPerSide);
  if (!Number.isFinite(maxParticipantsPerSide) || maxParticipantsPerSide <= 0) {
    throw new Error('invalid maxParticipantsPerSide');
  }

  const scheduledStartAt = String(session?.scheduledStartAt || '').trim();
  const endAt = String(session?.endAt || '').trim();
  if (!scheduledStartAt || !endAt) {
    throw new Error('missing session schedule');
  }

  return {
    sessionId: id,
    status: targetStatus,
    scheduledStartAt,
    endAt,
    maxParticipantsPerSide,
  };
}

