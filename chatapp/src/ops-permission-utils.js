const OPS_PERMISSION_HINTS = {
  debate_manage: '当前账号没有“场次管理”权限（需 ops_admin）',
  judge_review: '当前账号没有“判决审阅”权限（需 ops_viewer / ops_reviewer / ops_admin）',
  judge_rejudge: '当前账号没有“复核触发”权限（需 ops_reviewer / ops_admin）',
  role_manage: '仅 platform admin 可以管理 Ops 角色',
};

function normalizePermissionKey(permission) {
  const value = String(permission || '').trim();
  if (!value) {
    return '';
  }
  if (value === 'debate_manage' || value === 'debateManage') {
    return 'debateManage';
  }
  if (value === 'judge_review' || value === 'judgeReview') {
    return 'judgeReview';
  }
  if (value === 'judge_rejudge' || value === 'judgeRejudge') {
    return 'judgeRejudge';
  }
  if (value === 'role_manage' || value === 'roleManage') {
    return 'roleManage';
  }
  return '';
}

export function emptyOpsRbacMe() {
  return {
    userId: 0,
    isOwner: false,
    role: null,
    permissions: {
      debateManage: false,
      judgeReview: false,
      judgeRejudge: false,
      roleManage: false,
    },
  };
}

export function normalizeOpsRbacMe(payload) {
  const value = payload || {};
  const permissions = value.permissions || {};
  return {
    userId: Number(value.userId || 0),
    isOwner: !!value.isOwner,
    role: value.role == null ? null : String(value.role),
    permissions: {
      debateManage: !!permissions.debateManage,
      judgeReview: !!permissions.judgeReview,
      judgeRejudge: !!permissions.judgeRejudge,
      roleManage: !!permissions.roleManage,
    },
  };
}

export function parseOpsPermissionDenied(rawText) {
  if (!rawText || typeof rawText !== 'string') {
    return null;
  }
  const text = rawText.trim();
  const prefix = 'ops_permission_denied:';
  if (!text.startsWith(prefix)) {
    return null;
  }

  const firstSep = text.indexOf(':');
  const secondSep = text.indexOf(':', firstSep + 1);
  if (secondSep <= firstSep + 1 || secondSep + 1 >= text.length) {
    return null;
  }

  const permission = text.slice(firstSep + 1, secondSep).trim();
  const reason = text.slice(secondSep + 1).trim();
  if (!permission || !reason) {
    return null;
  }
  return {
    permission,
    reason,
  };
}

export function getOpsPermissionHint(permission) {
  return OPS_PERMISSION_HINTS[permission] || '当前账号没有执行该操作的权限';
}

export function hasOpsPermission(snapshot, permission) {
  const key = normalizePermissionKey(permission);
  if (!key) {
    return false;
  }
  return !!snapshot?.permissions?.[key];
}

export function hasAnyOpsPermission(snapshot) {
  return [
    'debateManage',
    'judgeReview',
    'judgeRejudge',
    'roleManage',
  ].some((key) => !!snapshot?.permissions?.[key]);
}

export function hasRequiredOpsPermissions(snapshot, permissions = []) {
  if (!Array.isArray(permissions) || permissions.length === 0) {
    return hasAnyOpsPermission(snapshot);
  }
  return permissions.every((item) => hasOpsPermission(snapshot, item));
}

export function resolveOpsErrorText(error, fallback = '操作失败') {
  const serverText = error?.response?.data?.error;
  if (typeof serverText === 'string' && serverText.trim()) {
    const denied = parseOpsPermissionDenied(serverText);
    if (denied) {
      return `${getOpsPermissionHint(denied.permission)}（${denied.reason}）`;
    }
    return serverText;
  }
  if (typeof error?.message === 'string' && error.message.trim()) {
    return error.message;
  }
  return fallback;
}
