import assert from 'node:assert/strict';
import {
  emptyOpsRbacMe,
  getOpsPermissionHint,
  normalizeOpsRbacMe,
  parseOpsPermissionDenied,
  resolveOpsErrorText,
} from './ops-permission-utils.js';

assert.deepEqual(emptyOpsRbacMe(), {
  userId: 0,
  wsId: 0,
  isOwner: false,
  role: null,
  permissions: {
    debateManage: false,
    judgeReview: false,
    judgeRejudge: false,
    roleManage: false,
  },
});

assert.deepEqual(
  normalizeOpsRbacMe({
    userId: 2,
    wsId: 1,
    isOwner: false,
    role: 'ops_viewer',
    permissions: {
      debateManage: false,
      judgeReview: true,
      judgeRejudge: false,
      roleManage: false,
    },
  }),
  {
    userId: 2,
    wsId: 1,
    isOwner: false,
    role: 'ops_viewer',
    permissions: {
      debateManage: false,
      judgeReview: true,
      judgeRejudge: false,
      roleManage: false,
    },
  },
);

assert.deepEqual(
  parseOpsPermissionDenied('ops_permission_denied:judge_rejudge:ops role ops_viewer cannot access this operation'),
  {
    permission: 'judge_rejudge',
    reason: 'ops role ops_viewer cannot access this operation',
  },
);
assert.equal(parseOpsPermissionDenied('random_error_text'), null);

assert.equal(
  getOpsPermissionHint('role_manage'),
  '仅 workspace owner 可以管理 Ops 角色',
);
assert.equal(
  getOpsPermissionHint('unknown_permission'),
  '当前账号没有执行该操作的权限',
);

assert.equal(
  resolveOpsErrorText(
    {
      response: {
        data: {
          error:
            'ops_permission_denied:debate_manage:ops role ops_viewer cannot access this operation',
        },
      },
    },
    'fallback',
  ),
  '当前账号没有“场次管理”权限（需 ops_admin）（ops role ops_viewer cannot access this operation）',
);

assert.equal(
  resolveOpsErrorText({ message: 'plain error message' }, 'fallback'),
  'plain error message',
);
assert.equal(resolveOpsErrorText({}, 'fallback'), 'fallback');
