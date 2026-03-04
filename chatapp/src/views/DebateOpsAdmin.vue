<template>
  <div class="flex h-screen">
    <Sidebar />
    <div class="flex-1 overflow-y-auto bg-gray-50">
      <div class="max-w-6xl mx-auto p-6 space-y-5">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h1 class="text-2xl font-bold text-gray-900">Debate Ops Admin</h1>
            <p class="text-sm text-gray-600 mt-1">
              创建辩题、排期场次并管理定时窗口，保证“到点开放、过时收口”。
            </p>
          </div>
          <button
            @click="refreshData"
            :disabled="loading"
            class="px-4 py-2 rounded bg-blue-600 text-white text-sm disabled:opacity-50"
          >
            {{ loading ? '刷新中...' : '刷新' }}
          </button>
        </div>

        <div v-if="errorText" class="bg-red-50 text-red-700 border border-red-200 rounded p-3 text-sm">
          {{ errorText }}
        </div>

        <div class="bg-white border rounded-lg p-4 space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <div class="text-sm font-semibold text-gray-900">Ops RBAC 角色管理</div>
              <div class="text-xs text-gray-500 mt-1">仅 workspace owner 可授予/撤销角色。</div>
            </div>
            <button
              @click="refreshRoleAssignments"
              :disabled="roleLoading"
              class="px-3 py-1 rounded border text-xs bg-white hover:bg-gray-100 disabled:opacity-50"
            >
              {{ roleLoading ? '刷新中...' : '刷新角色列表' }}
            </button>
          </div>

          <div v-if="roleErrorText" class="bg-red-50 text-red-700 border border-red-200 rounded p-2 text-xs">
            {{ roleErrorText }}
          </div>

          <div class="grid grid-cols-1 md:grid-cols-3 gap-2">
            <select v-model="roleForm.userId" class="border rounded px-3 py-2 text-sm">
              <option value="">选择用户</option>
              <option v-for="user in workspaceUsers()" :key="user.id" :value="String(user.id)">
                {{ user.fullname }} (#{{ user.id }}) · {{ user.email }}
              </option>
            </select>
            <select v-model="roleForm.role" class="border rounded px-3 py-2 text-sm">
              <option value="ops_admin">ops_admin</option>
              <option value="ops_reviewer">ops_reviewer</option>
              <option value="ops_viewer">ops_viewer</option>
            </select>
            <button
              @click="upsertRoleAssignment"
              :disabled="roleLoading || !roleForm.userId"
              class="px-3 py-2 rounded bg-slate-700 text-white text-sm disabled:opacity-50"
            >
              {{ roleLoading ? '处理中...' : '授予/更新角色' }}
            </button>
          </div>

          <div class="overflow-x-auto">
            <table class="min-w-full text-xs">
              <thead>
                <tr class="text-left text-gray-500 border-b">
                  <th class="py-2 pr-3">User</th>
                  <th class="py-2 pr-3">Role</th>
                  <th class="py-2 pr-3">GrantedBy</th>
                  <th class="py-2 pr-3">UpdatedAt</th>
                  <th class="py-2 pr-3">Action</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in roleAssignments" :key="item.userId" class="border-b last:border-b-0">
                  <td class="py-2 pr-3 text-gray-900">{{ userLabel(item.userId) }}</td>
                  <td class="py-2 pr-3 text-gray-700">{{ roleLabel(item.role) }}</td>
                  <td class="py-2 pr-3 text-gray-700">{{ userLabel(item.grantedBy) }}</td>
                  <td class="py-2 pr-3 text-gray-700">{{ formatDateTime(item.updatedAt) }}</td>
                  <td class="py-2 pr-3">
                    <button
                      @click="revokeRoleAssignment(item.userId)"
                      :disabled="roleLoading"
                      class="px-2 py-1 rounded border border-rose-300 bg-rose-50 text-rose-700 hover:bg-rose-100 disabled:opacity-50"
                    >
                      撤销
                    </button>
                  </td>
                </tr>
                <tr v-if="roleAssignments.length === 0">
                  <td colspan="5" class="py-3 text-center text-gray-500">暂无已授予角色</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div class="bg-white border rounded-lg p-4 space-y-3">
            <div class="text-sm font-semibold text-gray-900">创建辩题</div>
            <input v-model="topicForm.title" class="w-full border rounded px-3 py-2 text-sm" placeholder="标题" />
            <textarea
              v-model="topicForm.description"
              rows="3"
              class="w-full border rounded px-3 py-2 text-sm"
              placeholder="辩题描述"
            />
            <div class="grid grid-cols-2 gap-2">
              <input v-model="topicForm.category" class="border rounded px-3 py-2 text-sm" placeholder="分类（如 game）" />
              <label class="inline-flex items-center gap-2 text-sm text-gray-700">
                <input v-model="topicForm.isActive" type="checkbox" class="rounded border-gray-300" />
                active
              </label>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <input v-model="topicForm.stancePro" class="border rounded px-3 py-2 text-sm" placeholder="正方立场" />
              <input v-model="topicForm.stanceCon" class="border rounded px-3 py-2 text-sm" placeholder="反方立场" />
            </div>
            <textarea
              v-model="topicForm.contextSeed"
              rows="2"
              class="w-full border rounded px-3 py-2 text-sm"
              placeholder="背景知识（可空）"
            />
            <button
              @click="createTopic"
              :disabled="createTopicLoading"
              class="px-3 py-2 rounded bg-emerald-600 text-white text-sm disabled:opacity-50"
            >
              {{ createTopicLoading ? '创建中...' : '创建辩题' }}
            </button>
          </div>

          <div class="bg-white border rounded-lg p-4 space-y-3">
            <div class="text-sm font-semibold text-gray-900">创建场次</div>
            <select v-model="sessionForm.topicId" class="w-full border rounded px-3 py-2 text-sm">
              <option value="">选择辩题</option>
              <option v-for="topic in topics" :key="topic.id" :value="String(topic.id)">
                {{ topic.title }} (#{{ topic.id }})
              </option>
            </select>
            <div class="grid grid-cols-2 gap-2">
              <select v-model="sessionForm.status" class="border rounded px-3 py-2 text-sm">
                <option value="scheduled">scheduled</option>
                <option value="open">open</option>
              </select>
              <input
                v-model.number="sessionForm.maxParticipantsPerSide"
                type="number"
                min="1"
                class="border rounded px-3 py-2 text-sm"
                placeholder="每侧人数上限"
              />
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
              <label class="text-xs text-gray-600">
                开始时间
                <input v-model="sessionForm.scheduledStartAtLocal" type="datetime-local" class="w-full border rounded px-3 py-2 text-sm mt-1" />
              </label>
              <label class="text-xs text-gray-600">
                结束时间
                <input v-model="sessionForm.endAtLocal" type="datetime-local" class="w-full border rounded px-3 py-2 text-sm mt-1" />
              </label>
            </div>
            <div class="rounded border border-blue-200 bg-blue-50 p-2 text-xs text-blue-800 space-y-1">
              <div>窗口预判：{{ describeDraftWindowState(sessionForm) }}</div>
              <div>参与提示：{{ describeDraftJoinability(sessionForm) }}</div>
              <div v-if="describeDraftRecommendation(sessionForm)" class="font-medium">
                建议动作：{{ describeDraftRecommendation(sessionForm) }}
              </div>
            </div>
            <button
              @click="createSession"
              :disabled="createSessionLoading"
              class="px-3 py-2 rounded bg-indigo-600 text-white text-sm disabled:opacity-50"
            >
              {{ createSessionLoading ? '创建中...' : '创建场次' }}
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div class="bg-white border rounded-lg p-4 space-y-3">
            <div class="text-sm font-semibold text-gray-900">编辑辩题</div>
            <select v-model="topicEditForm.topicId" @change="syncTopicEditFormFromId(topicEditForm.topicId)" class="w-full border rounded px-3 py-2 text-sm">
              <option value="">选择辩题</option>
              <option v-for="topic in topics" :key="topic.id" :value="String(topic.id)">
                {{ topic.title }} (#{{ topic.id }})
              </option>
            </select>
            <input v-model="topicEditForm.title" class="w-full border rounded px-3 py-2 text-sm" placeholder="标题" />
            <textarea
              v-model="topicEditForm.description"
              rows="3"
              class="w-full border rounded px-3 py-2 text-sm"
              placeholder="辩题描述"
            />
            <div class="grid grid-cols-2 gap-2">
              <input v-model="topicEditForm.category" class="border rounded px-3 py-2 text-sm" placeholder="分类" />
              <label class="inline-flex items-center gap-2 text-sm text-gray-700">
                <input v-model="topicEditForm.isActive" type="checkbox" class="rounded border-gray-300" />
                active
              </label>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <input v-model="topicEditForm.stancePro" class="border rounded px-3 py-2 text-sm" placeholder="正方立场" />
              <input v-model="topicEditForm.stanceCon" class="border rounded px-3 py-2 text-sm" placeholder="反方立场" />
            </div>
            <textarea
              v-model="topicEditForm.contextSeed"
              rows="2"
              class="w-full border rounded px-3 py-2 text-sm"
              placeholder="背景知识（可空）"
            />
            <button
              @click="updateTopic"
              :disabled="updateTopicLoading || !topicEditForm.topicId"
              class="px-3 py-2 rounded bg-amber-600 text-white text-sm disabled:opacity-50"
            >
              {{ updateTopicLoading ? '保存中...' : '保存辩题' }}
            </button>
          </div>

          <div class="bg-white border rounded-lg p-4 space-y-3">
            <div class="text-sm font-semibold text-gray-900">编辑场次</div>
            <select v-model="sessionEditForm.sessionId" @change="syncSessionEditFormFromId(sessionEditForm.sessionId)" class="w-full border rounded px-3 py-2 text-sm">
              <option value="">选择场次</option>
              <option v-for="session in sessions" :key="session.id" :value="String(session.id)">
                #{{ session.id }} · {{ topicTitle(session.topicId) }}
              </option>
            </select>
            <div class="grid grid-cols-2 gap-2">
              <select v-model="sessionEditForm.status" class="border rounded px-3 py-2 text-sm">
                <option value="scheduled">scheduled</option>
                <option value="open">open</option>
                <option value="running">running</option>
                <option value="judging">judging</option>
                <option value="closed">closed</option>
                <option value="canceled">canceled</option>
              </select>
              <input
                v-model.number="sessionEditForm.maxParticipantsPerSide"
                type="number"
                min="1"
                class="border rounded px-3 py-2 text-sm"
                placeholder="每侧人数上限"
              />
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
              <label class="text-xs text-gray-600">
                开始时间
                <input v-model="sessionEditForm.scheduledStartAtLocal" type="datetime-local" class="w-full border rounded px-3 py-2 text-sm mt-1" />
              </label>
              <label class="text-xs text-gray-600">
                结束时间
                <input v-model="sessionEditForm.endAtLocal" type="datetime-local" class="w-full border rounded px-3 py-2 text-sm mt-1" />
              </label>
            </div>
            <div class="rounded border border-violet-200 bg-violet-50 p-2 text-xs text-violet-800 space-y-1">
              <div>窗口预判：{{ describeDraftWindowState(sessionEditForm) }}</div>
              <div>参与提示：{{ describeDraftJoinability(sessionEditForm) }}</div>
              <div v-if="describeDraftRecommendation(sessionEditForm)" class="font-medium">
                建议动作：{{ describeDraftRecommendation(sessionEditForm) }}
              </div>
            </div>
            <div class="flex flex-wrap gap-2">
              <button
                @click="updateSession"
                :disabled="updateSessionLoading || !sessionEditForm.sessionId"
                class="px-3 py-2 rounded bg-violet-600 text-white text-sm disabled:opacity-50"
              >
                {{ updateSessionLoading ? '保存中...' : '保存场次' }}
              </button>
              <button
                @click="openSessionJudgeReport(sessionEditForm.sessionId)"
                :disabled="!sessionEditForm.sessionId"
                class="px-3 py-2 rounded border border-gray-300 text-sm bg-white hover:bg-gray-100 disabled:opacity-50"
              >
                查看判决
              </button>
            </div>
          </div>
        </div>

        <div class="bg-white border rounded-lg p-4 space-y-3">
          <div class="flex items-center justify-between">
            <div class="text-sm font-semibold text-gray-900">场次看板</div>
            <div class="text-xs text-gray-500">topics: {{ topics.length }} · sessions: {{ sessions.length }}</div>
          </div>
          <div class="overflow-x-auto">
            <table class="min-w-full text-sm">
              <thead>
                <tr class="text-left text-gray-500 border-b">
                  <th class="py-2 pr-4">Session</th>
                  <th class="py-2 pr-4">Topic</th>
                  <th class="py-2 pr-4">Status</th>
                  <th class="py-2 pr-4">Scheduled</th>
                  <th class="py-2 pr-4">End</th>
                  <th class="py-2 pr-4">Joinable</th>
                  <th class="py-2 pr-4">Window</th>
                  <th class="py-2 pr-4">Reason</th>
                  <th class="py-2 pr-4">Recommend</th>
                  <th class="py-2 pr-4">Action</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in sessions.slice(0, 50)" :key="item.id" class="border-b last:border-b-0">
                  <td class="py-2 pr-4">#{{ item.id }}</td>
                  <td class="py-2 pr-4">{{ topicTitle(item.topicId) }}</td>
                  <td class="py-2 pr-4">{{ item.status }}</td>
                  <td class="py-2 pr-4">{{ formatDateTime(item.scheduledStartAt) }}</td>
                  <td class="py-2 pr-4">{{ formatDateTime(item.endAt) }}</td>
                  <td class="py-2 pr-4">{{ item.joinable ? 'yes' : 'no' }}</td>
                  <td class="py-2 pr-4">
                    <span
                      class="inline-flex items-center rounded px-2 py-1 text-xs"
                      :class="windowStateBadgeClass(item)"
                    >
                      {{ windowStateLabel(item) }}
                    </span>
                  </td>
                  <td class="py-2 pr-4 text-xs text-gray-700">{{ joinabilityReason(item) }}</td>
                  <td class="py-2 pr-4">
                    <button
                      v-if="hasRecommendedAction(item)"
                      @click="applyRecommendedAction(item)"
                      :disabled="quickUpdateSessionId === item.id"
                      class="px-2 py-1 rounded border border-emerald-300 text-xs bg-emerald-50 text-emerald-700 hover:bg-emerald-100 disabled:opacity-50"
                    >
                      {{ quickUpdateSessionId === item.id ? '处理中...' : recommendedActionLabel(item) }}
                    </button>
                    <span v-else class="text-xs text-gray-400">-</span>
                  </td>
                  <td class="py-2 pr-4">
                    <div class="flex flex-wrap gap-1">
                      <button
                        @click="openSessionJudgeReport(item.id)"
                        class="px-2 py-1 rounded border border-gray-300 text-xs bg-white hover:bg-gray-100"
                      >
                        判决
                      </button>
                      <button
                        v-for="nextStatus in nextQuickStatusActions(item.status)"
                        :key="`${item.id}-${nextStatus}`"
                        @click="quickUpdateSessionStatus(item, nextStatus)"
                        :disabled="quickUpdateSessionId === item.id"
                        class="px-2 py-1 rounded border border-gray-300 text-xs bg-white hover:bg-gray-100 disabled:opacity-50"
                      >
                        {{ quickUpdateSessionId === item.id ? '处理中...' : `设为 ${nextStatus}` }}
                      </button>
                    </div>
                  </td>
                </tr>
                <tr v-if="sessions.length === 0">
                  <td colspan="10" class="py-4 text-center text-gray-500">暂无场次</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="bg-white border rounded-lg p-4 space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <div class="text-sm font-semibold text-gray-900">判决证据审阅与复核</div>
              <div class="text-xs text-gray-500 mt-1">
                scanned: {{ reviewMeta.scannedCount }} · returned: {{ reviewMeta.returnedCount }}
              </div>
            </div>
            <button
              @click="refreshJudgeReviews"
              :disabled="reviewLoading"
              class="px-3 py-1 rounded border text-xs bg-white hover:bg-gray-100 disabled:opacity-50"
            >
              {{ reviewLoading ? '刷新中...' : '刷新审阅列表' }}
            </button>
          </div>

          <div v-if="reviewErrorText" class="bg-red-50 text-red-700 border border-red-200 rounded p-2 text-xs">
            {{ reviewErrorText }}
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-7 gap-2">
            <label class="text-xs text-gray-600">
              开始时间
              <input v-model="reviewFilter.fromLocal" type="datetime-local" class="w-full border rounded px-2 py-1 mt-1" />
            </label>
            <label class="text-xs text-gray-600">
              结束时间
              <input v-model="reviewFilter.toLocal" type="datetime-local" class="w-full border rounded px-2 py-1 mt-1" />
            </label>
            <label class="text-xs text-gray-600">
              Winner
              <select v-model="reviewFilter.winner" class="w-full border rounded px-2 py-1 mt-1">
                <option value="">all</option>
                <option value="pro">pro</option>
                <option value="con">con</option>
                <option value="draw">draw</option>
              </select>
            </label>
            <label class="text-xs text-gray-600">
              Rejudge
              <select v-model="reviewFilter.rejudgeTriggered" class="w-full border rounded px-2 py-1 mt-1">
                <option value="">all</option>
                <option value="true">yes</option>
                <option value="false">no</option>
              </select>
            </label>
            <label class="text-xs text-gray-600">
              Evidence
              <select v-model="reviewFilter.hasVerdictEvidence" class="w-full border rounded px-2 py-1 mt-1">
                <option value="">all</option>
                <option value="true">has refs</option>
                <option value="false">no refs</option>
              </select>
            </label>
            <label class="text-xs text-gray-600">
              Limit
              <input v-model.number="reviewFilter.limit" type="number" min="1" max="200" class="w-full border rounded px-2 py-1 mt-1" />
            </label>
            <label class="inline-flex items-center gap-2 text-xs text-gray-700 mt-5">
              <input v-model="reviewFilter.anomalyOnly" type="checkbox" class="rounded border-gray-300" />
              仅异常
            </label>
          </div>

          <div class="overflow-x-auto">
            <table class="min-w-full text-xs">
              <thead>
                <tr class="text-left text-gray-500 border-b">
                  <th class="py-2 pr-3">Created</th>
                  <th class="py-2 pr-3">Session</th>
                  <th class="py-2 pr-3">Winner</th>
                  <th class="py-2 pr-3">Gap</th>
                  <th class="py-2 pr-3">Evidence</th>
                  <th class="py-2 pr-3">Flags</th>
                  <th class="py-2 pr-3">Action</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in reviewRows" :key="row.reportId" class="border-b last:border-b-0">
                  <td class="py-2 pr-3 text-gray-700">{{ formatDateTime(row.createdAt) }}</td>
                  <td class="py-2 pr-3 text-gray-900">#{{ row.sessionId }}</td>
                  <td class="py-2 pr-3 text-gray-900">{{ row.winner }}</td>
                  <td class="py-2 pr-3 text-gray-900">{{ row.scoreGap }}</td>
                  <td class="py-2 pr-3 text-gray-900">{{ row.verdictEvidenceCount }}</td>
                  <td class="py-2 pr-3 text-gray-700">{{ judgeReviewAbnormalText(row.abnormalFlags) }}</td>
                  <td class="py-2 pr-3">
                    <div class="flex flex-wrap gap-1">
                      <button
                        @click="openSessionJudgeReport(row.sessionId)"
                        class="px-2 py-1 rounded border border-gray-300 bg-white hover:bg-gray-100"
                      >
                        查看
                      </button>
                      <button
                        @click="triggerJudgeRejudge(row.sessionId)"
                        :disabled="rejudgeReviewSessionId === row.sessionId"
                        class="px-2 py-1 rounded border border-amber-300 bg-amber-50 text-amber-700 hover:bg-amber-100 disabled:opacity-50"
                      >
                        {{ rejudgeReviewSessionId === row.sessionId ? '处理中...' : '触发复核' }}
                      </button>
                    </div>
                  </td>
                </tr>
                <tr v-if="reviewRows.length === 0">
                  <td colspan="7" class="py-4 text-center text-gray-500">暂无审阅数据</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import Sidebar from '../components/Sidebar.vue';
import {
  buildQuickUpdateSessionPayload,
  getOpsSessionJoinability,
  getOpsSessionRecommendedAction,
  getOpsSessionWindowState,
  nextQuickStatusActions as resolveNextQuickStatusActions,
} from '../debate-ops-utils';

function toLocalInputValue(date) {
  const d = new Date(date);
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const min = String(d.getMinutes()).padStart(2, '0');
  return `${yyyy}-${mm}-${dd}T${hh}:${min}`;
}

function emptyTopicEditForm() {
  return {
    topicId: '',
    title: '',
    description: '',
    category: '',
    stancePro: '',
    stanceCon: '',
    contextSeed: '',
    isActive: true,
  };
}

function emptySessionEditForm(date = new Date()) {
  const plusOneHour = new Date(date.getTime() + 60 * 60 * 1000);
  return {
    sessionId: '',
    status: 'scheduled',
    scheduledStartAtLocal: toLocalInputValue(date),
    endAtLocal: toLocalInputValue(plusOneHour),
    maxParticipantsPerSide: 500,
  };
}

function parseOptionalBoolean(value) {
  if (value === true || value === 'true') {
    return true;
  }
  if (value === false || value === 'false') {
    return false;
  }
  return null;
}

export default {
  components: {
    Sidebar,
  },
  data() {
    const now = new Date();
    const minus24Hours = new Date(now.getTime() - 24 * 60 * 60 * 1000);
    const plusOneHour = new Date(now.getTime() + 60 * 60 * 1000);
    return {
      loading: false,
      reviewLoading: false,
      roleLoading: false,
      createTopicLoading: false,
      createSessionLoading: false,
      updateTopicLoading: false,
      updateSessionLoading: false,
      quickUpdateSessionId: 0,
      rejudgeReviewSessionId: 0,
      errorText: '',
      reviewErrorText: '',
      roleErrorText: '',
      topics: [],
      sessions: [],
      reviewRows: [],
      roleAssignments: [],
      reviewMeta: {
        scannedCount: 0,
        returnedCount: 0,
      },
      roleForm: {
        userId: '',
        role: 'ops_reviewer',
      },
      reviewFilter: {
        fromLocal: toLocalInputValue(minus24Hours),
        toLocal: toLocalInputValue(now),
        winner: '',
        rejudgeTriggered: '',
        hasVerdictEvidence: '',
        anomalyOnly: true,
        limit: 50,
      },
      topicForm: {
        title: '',
        description: '',
        category: 'game',
        stancePro: '支持',
        stanceCon: '反对',
        contextSeed: '',
        isActive: true,
      },
      sessionForm: {
        topicId: '',
        status: 'scheduled',
        scheduledStartAtLocal: toLocalInputValue(now),
        endAtLocal: toLocalInputValue(plusOneHour),
        maxParticipantsPerSide: 500,
      },
      topicEditForm: emptyTopicEditForm(),
      sessionEditForm: emptySessionEditForm(now),
    };
  },
  methods: {
    formatDateTime(value) {
      if (!value) {
        return '-';
      }
      const date = new Date(value);
      return Number.isNaN(date.getTime()) ? '-' : date.toLocaleString();
    },
    topicTitle(topicId) {
      const topic = this.topics.find((item) => Number(item.id) === Number(topicId));
      return topic ? `${topic.title} (#${topic.id})` : `topic#${topicId}`;
    },
    toIso(localText) {
      if (!localText) {
        return '';
      }
      const date = new Date(localText);
      if (Number.isNaN(date.getTime())) {
        return '';
      }
      return date.toISOString();
    },
    workspaceUsers() {
      const usersMap = this.$store?.state?.users || {};
      return Object.values(usersMap).sort((a, b) => Number(a.id || 0) - Number(b.id || 0));
    },
    roleLabel(role) {
      const value = String(role || '');
      if (value === 'ops_admin') {
        return 'ops_admin（场次管理+审阅+复核）';
      }
      if (value === 'ops_reviewer') {
        return 'ops_reviewer（审阅+复核）';
      }
      if (value === 'ops_viewer') {
        return 'ops_viewer（仅审阅）';
      }
      return value || '-';
    },
    userLabel(userId) {
      const id = Number(userId || 0);
      if (!id) {
        return '-';
      }
      const usersMap = this.$store?.state?.users || {};
      const user = usersMap[id];
      if (!user) {
        return `#${id}`;
      }
      return `${user.fullname || 'unknown'} (#${id})`;
    },
    async refreshRoleAssignments() {
      this.roleLoading = true;
      this.roleErrorText = '';
      try {
        const response = await this.$store.dispatch('listOpsRoleAssignments');
        this.roleAssignments = Array.isArray(response?.items) ? response.items : [];
      } catch (error) {
        this.roleErrorText = error?.response?.data?.error || error?.message || '加载角色列表失败';
      } finally {
        this.roleLoading = false;
      }
    },
    async upsertRoleAssignment() {
      const userId = Number(this.roleForm.userId || 0);
      if (!userId) {
        return;
      }
      this.roleLoading = true;
      this.roleErrorText = '';
      try {
        await this.$store.dispatch('upsertOpsRoleAssignment', {
          userId,
          role: this.roleForm.role,
        });
        await this.refreshRoleAssignments();
      } catch (error) {
        this.roleErrorText = error?.response?.data?.error || error?.message || '授予角色失败';
      } finally {
        this.roleLoading = false;
      }
    },
    async revokeRoleAssignment(userIdRaw) {
      const userId = Number(userIdRaw || 0);
      if (!userId) {
        return;
      }
      this.roleLoading = true;
      this.roleErrorText = '';
      try {
        await this.$store.dispatch('revokeOpsRoleAssignment', { userId });
        await this.refreshRoleAssignments();
      } catch (error) {
        this.roleErrorText = error?.response?.data?.error || error?.message || '撤销角色失败';
      } finally {
        this.roleLoading = false;
      }
    },
    judgeReviewAbnormalText(flags) {
      const values = Array.isArray(flags) ? flags : [];
      if (values.length === 0) {
        return '-';
      }
      return values.join(' / ');
    },
    buildJudgeReviewPayload() {
      return {
        from: this.toIso(this.reviewFilter.fromLocal),
        to: this.toIso(this.reviewFilter.toLocal),
        winner: this.reviewFilter.winner || null,
        rejudgeTriggered: parseOptionalBoolean(this.reviewFilter.rejudgeTriggered),
        hasVerdictEvidence: parseOptionalBoolean(this.reviewFilter.hasVerdictEvidence),
        anomalyOnly: !!this.reviewFilter.anomalyOnly,
        limit: Number(this.reviewFilter.limit || 50),
      };
    },
    async refreshJudgeReviews() {
      this.reviewLoading = true;
      this.reviewErrorText = '';
      try {
        const payload = this.buildJudgeReviewPayload();
        const response = await this.$store.dispatch('listJudgeReviewsOps', payload);
        this.reviewRows = Array.isArray(response?.items) ? response.items : [];
        this.reviewMeta = {
          scannedCount: Number(response?.scannedCount || 0),
          returnedCount: Number(response?.returnedCount || this.reviewRows.length),
        };
      } catch (error) {
        this.reviewErrorText = error?.response?.data?.error || error?.message || '加载判决审阅列表失败';
      } finally {
        this.reviewLoading = false;
      }
    },
    async triggerJudgeRejudge(sessionIdRaw) {
      const sessionId = Number(sessionIdRaw);
      if (!sessionId) {
        return;
      }
      this.rejudgeReviewSessionId = sessionId;
      this.reviewErrorText = '';
      try {
        await this.$store.dispatch('requestJudgeRejudgeOps', { sessionId });
        await this.refreshJudgeReviews();
      } catch (error) {
        this.reviewErrorText = error?.response?.data?.error || error?.message || '触发复核失败';
      } finally {
        this.rejudgeReviewSessionId = 0;
      }
    },
    buildSessionDraftForTiming(form) {
      return {
        status: String(form?.status || 'scheduled'),
        scheduledStartAt: this.toIso(form?.scheduledStartAtLocal),
        endAt: this.toIso(form?.endAtLocal),
        joinable: false,
      };
    },
    windowStateLabel(session) {
      const state = getOpsSessionWindowState(session);
      if (state === 'upcoming') {
        return '待开始';
      }
      if (state === 'active') {
        return '窗口中';
      }
      if (state === 'expired') {
        return '已结束';
      }
      return '时间异常';
    },
    windowStateBadgeClass(session) {
      const state = getOpsSessionWindowState(session);
      if (state === 'upcoming') {
        return 'bg-amber-100 text-amber-800';
      }
      if (state === 'active') {
        return 'bg-emerald-100 text-emerald-800';
      }
      if (state === 'expired') {
        return 'bg-gray-200 text-gray-700';
      }
      return 'bg-red-100 text-red-700';
    },
    joinabilityReason(session) {
      return getOpsSessionJoinability(session).text;
    },
    recommendedAction(session) {
      return getOpsSessionRecommendedAction(session);
    },
    hasRecommendedAction(session) {
      return !!this.recommendedAction(session)?.targetStatus;
    },
    recommendedActionLabel(session) {
      const rec = this.recommendedAction(session);
      return rec?.label || '';
    },
    describeDraftWindowState(form) {
      return this.windowStateLabel(this.buildSessionDraftForTiming(form));
    },
    describeDraftJoinability(form) {
      return this.joinabilityReason(this.buildSessionDraftForTiming(form));
    },
    describeDraftRecommendation(form) {
      const rec = this.recommendedAction(this.buildSessionDraftForTiming(form));
      if (!rec) {
        return '';
      }
      return `${rec.label}（${rec.reason}）`;
    },
    syncTopicEditFormFromId(topicIdRaw) {
      const selectedTopicId = String(topicIdRaw || '');
      const topic = this.topics.find((item) => String(item.id) === selectedTopicId);
      if (!topic) {
        this.topicEditForm = {
          ...emptyTopicEditForm(),
          topicId: selectedTopicId,
        };
        return;
      }
      this.topicEditForm = {
        topicId: String(topic.id),
        title: topic.title || '',
        description: topic.description || '',
        category: topic.category || '',
        stancePro: topic.stancePro || '',
        stanceCon: topic.stanceCon || '',
        contextSeed: topic.contextSeed || '',
        isActive: !!topic.isActive,
      };
    },
    syncSessionEditFormFromId(sessionIdRaw) {
      const selectedSessionId = String(sessionIdRaw || '');
      const session = this.sessions.find((item) => String(item.id) === selectedSessionId);
      if (!session) {
        this.sessionEditForm = {
          ...emptySessionEditForm(new Date()),
          sessionId: selectedSessionId,
        };
        return;
      }
      this.sessionEditForm = {
        sessionId: String(session.id),
        status: session.status || 'scheduled',
        scheduledStartAtLocal: toLocalInputValue(session.scheduledStartAt || new Date()),
        endAtLocal: toLocalInputValue(session.endAt || new Date(Date.now() + 60 * 60 * 1000)),
        maxParticipantsPerSide: Number(session.maxParticipantsPerSide || 500),
      };
    },
    async refreshData() {
      this.loading = true;
      this.errorText = '';
      try {
        const [topics, sessions, reviews, roleAssignments] = await Promise.all([
          this.$store.dispatch('listDebateTopics', { activeOnly: false, limit: 200 }),
          this.$store.dispatch('listDebateSessions', { limit: 200 }),
          this.$store.dispatch('listJudgeReviewsOps', this.buildJudgeReviewPayload()),
          this.$store.dispatch('listOpsRoleAssignments'),
        ]);
        this.topics = topics || [];
        this.sessions = sessions || [];
        this.reviewRows = Array.isArray(reviews?.items) ? reviews.items : [];
        this.roleAssignments = Array.isArray(roleAssignments?.items) ? roleAssignments.items : [];
        this.reviewMeta = {
          scannedCount: Number(reviews?.scannedCount || 0),
          returnedCount: Number(reviews?.returnedCount || this.reviewRows.length),
        };
        this.reviewErrorText = '';
        this.roleErrorText = '';
        if (!this.topicEditForm.topicId && this.topics.length > 0) {
          this.topicEditForm.topicId = String(this.topics[0].id);
        }
        if (!this.sessionEditForm.sessionId && this.sessions.length > 0) {
          this.sessionEditForm.sessionId = String(this.sessions[0].id);
        }
        this.syncTopicEditFormFromId(this.topicEditForm.topicId);
        this.syncSessionEditFormFromId(this.sessionEditForm.sessionId);
      } catch (error) {
        this.errorText = error?.response?.data?.error || error?.message || '刷新失败';
      } finally {
        this.loading = false;
      }
    },
    async createTopic() {
      await this.upsertTopic('create');
    },
    async createSession() {
      await this.upsertSession('create');
    },
    async updateTopic() {
      await this.upsertTopic('update');
    },
    async upsertTopic(mode = 'create') {
      const isCreate = mode === 'create';
      if (!isCreate && !this.topicEditForm.topicId) {
        return;
      }
      if (isCreate) {
        this.createTopicLoading = true;
      } else {
        this.updateTopicLoading = true;
      }
      this.errorText = '';
      try {
        if (isCreate) {
          await this.$store.dispatch('createDebateTopicOps', {
            title: this.topicForm.title,
            description: this.topicForm.description,
            category: this.topicForm.category,
            stancePro: this.topicForm.stancePro,
            stanceCon: this.topicForm.stanceCon,
            contextSeed: this.topicForm.contextSeed,
            isActive: this.topicForm.isActive,
          });
          this.topicForm.title = '';
          this.topicForm.description = '';
          this.topicForm.contextSeed = '';
        } else {
          await this.$store.dispatch('updateDebateTopicOps', {
            topicId: Number(this.topicEditForm.topicId),
            title: this.topicEditForm.title,
            description: this.topicEditForm.description,
            category: this.topicEditForm.category,
            stancePro: this.topicEditForm.stancePro,
            stanceCon: this.topicEditForm.stanceCon,
            contextSeed: this.topicEditForm.contextSeed,
            isActive: this.topicEditForm.isActive,
          });
        }
        await this.refreshData();
      } catch (error) {
        this.errorText =
          error?.response?.data?.error || error?.message || (isCreate ? '创建辩题失败' : '更新辩题失败');
      } finally {
        if (isCreate) {
          this.createTopicLoading = false;
        } else {
          this.updateTopicLoading = false;
        }
      }
    },
    async updateSession() {
      await this.upsertSession('update');
    },
    async upsertSession(mode = 'create') {
      const isCreate = mode === 'create';
      if (!isCreate && !this.sessionEditForm.sessionId) {
        return;
      }
      if (isCreate) {
        this.createSessionLoading = true;
      } else {
        this.updateSessionLoading = true;
      }
      this.errorText = '';
      try {
        const scheduledStartAt = this.toIso(
          isCreate ? this.sessionForm.scheduledStartAtLocal : this.sessionEditForm.scheduledStartAtLocal,
        );
        const endAt = this.toIso(isCreate ? this.sessionForm.endAtLocal : this.sessionEditForm.endAtLocal);
        if (!scheduledStartAt || !endAt) {
          throw new Error('请填写有效的开始/结束时间');
        }
        if (isCreate) {
          await this.$store.dispatch('createDebateSessionOps', {
            topicId: Number(this.sessionForm.topicId),
            status: this.sessionForm.status,
            scheduledStartAt,
            endAt,
            maxParticipantsPerSide: Number(this.sessionForm.maxParticipantsPerSide),
          });
        } else {
          await this.$store.dispatch('updateDebateSessionOps', {
            sessionId: Number(this.sessionEditForm.sessionId),
            status: this.sessionEditForm.status,
            scheduledStartAt,
            endAt,
            maxParticipantsPerSide: Number(this.sessionEditForm.maxParticipantsPerSide),
          });
        }
        await this.refreshData();
      } catch (error) {
        this.errorText =
          error?.response?.data?.error || error?.message || (isCreate ? '创建场次失败' : '更新场次失败');
      } finally {
        if (isCreate) {
          this.createSessionLoading = false;
        } else {
          this.updateSessionLoading = false;
        }
      }
    },
    async quickUpdateSessionStatus(session, nextStatus) {
      const sessionId = Number(session?.id || 0);
      if (!sessionId) {
        return;
      }
      this.quickUpdateSessionId = sessionId;
      this.errorText = '';
      try {
        const payload = buildQuickUpdateSessionPayload(session, nextStatus);
        await this.$store.dispatch('updateDebateSessionOps', payload);
        await this.refreshData();
      } catch (error) {
        this.errorText = error?.response?.data?.error || error?.message || '快速更新场次状态失败';
      } finally {
        this.quickUpdateSessionId = 0;
      }
    },
    async applyRecommendedAction(session) {
      const recommendation = this.recommendedAction(session);
      if (!recommendation?.targetStatus) {
        return;
      }
      await this.quickUpdateSessionStatus(session, recommendation.targetStatus);
    },
    nextQuickStatusActions(status) {
      return resolveNextQuickStatusActions(status);
    },
    async openSessionJudgeReport(sessionIdRaw) {
      const sessionId = Number(sessionIdRaw);
      if (!sessionId) {
        return;
      }
      await this.$router.push({
        path: '/judge-report',
        query: { sessionId: String(sessionId) },
      });
    },
  },
  async mounted() {
    await this.refreshData();
  },
};
</script>
