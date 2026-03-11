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

        <div class="bg-slate-50 border border-slate-200 rounded-lg p-3 text-xs text-slate-700 space-y-2">
          <div>
            当前身份：
            <span v-if="opsRbacMe.isOwner" class="font-semibold text-slate-900">platform admin</span>
            <span v-else-if="opsRbacMe.role" class="font-semibold text-slate-900">{{ opsRbacMe.role }}</span>
            <span v-else class="font-semibold text-slate-900">普通成员（未分配 Ops 角色）</span>
          </div>
          <div class="flex flex-wrap gap-2">
            <span class="px-2 py-1 rounded bg-white border">
              场次管理: {{ canDebateManage ? 'yes' : 'no' }}
            </span>
            <span class="px-2 py-1 rounded bg-white border">
              判决审阅: {{ canJudgeReview ? 'yes' : 'no' }}
            </span>
            <span class="px-2 py-1 rounded bg-white border">
              复核触发: {{ canJudgeRejudge ? 'yes' : 'no' }}
            </span>
            <span class="px-2 py-1 rounded bg-white border">
              角色管理: {{ canRoleManage ? 'yes' : 'no' }}
            </span>
          </div>
        </div>

        <div class="bg-white border rounded-lg p-4 space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <div class="text-sm font-semibold text-gray-900">Ops RBAC 角色管理</div>
              <div class="text-xs text-gray-500 mt-1">仅 platform admin 可授予/撤销角色。</div>
            </div>
            <button
              @click="refreshRoleAssignments"
              :disabled="roleLoading || !canRoleManage"
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
              <option v-for="user in platformUsers()" :key="user.id" :value="String(user.id)">
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
              :disabled="roleLoading || !canRoleManage || !roleForm.userId"
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
                      :disabled="roleLoading || !canRoleManage"
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
              :disabled="createTopicLoading || !canDebateManage"
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
              :disabled="createSessionLoading || !canDebateManage"
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
              :disabled="updateTopicLoading || !canDebateManage || !topicEditForm.topicId"
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
                :disabled="updateSessionLoading || !canDebateManage || !sessionEditForm.sessionId"
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
                      :disabled="quickUpdateSessionId === item.id || !canDebateManage"
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
                        :disabled="quickUpdateSessionId === item.id || !canDebateManage"
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
              :disabled="reviewLoading || !canJudgeReview"
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
                        :disabled="rejudgeReviewSessionId === row.sessionId || !canJudgeRejudge"
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

        <div class="bg-white border rounded-lg p-4 space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <div class="text-sm font-semibold text-gray-900">裁判观测汇总（Ops Dashboard）</div>
              <div class="text-xs text-gray-500 mt-1">
                updated: {{ formatDateTime(observabilityUpdatedAt) }} ·
                metrics: {{ formatDateTime(observabilityMetricsUpdatedAt) }}
              </div>
            </div>
            <div class="flex items-center gap-2">
              <button
                @click="toggleObservabilityThresholdSettings"
                class="px-3 py-1 rounded border text-xs bg-white hover:bg-gray-100"
              >
                {{ observabilityThresholdSettingsOpen ? '收起阈值设置' : '阈值设置' }}
              </button>
              <button
                @click="refreshJudgeObservability"
                :disabled="observabilityLoading || !canJudgeReview"
                class="px-3 py-1 rounded border text-xs bg-white hover:bg-gray-100 disabled:opacity-50"
              >
                {{ observabilityLoading ? '刷新中...' : '刷新观测汇总' }}
              </button>
            </div>
          </div>

          <div v-if="observabilityErrorText" class="bg-red-50 text-red-700 border border-red-200 rounded p-2 text-xs">
            {{ observabilityErrorText }}
          </div>
          <div
            v-if="observabilityMetricsErrorText"
            class="bg-red-50 text-red-700 border border-red-200 rounded p-2 text-xs"
          >
            {{ observabilityMetricsErrorText }}
          </div>

          <div class="grid grid-cols-1 md:grid-cols-4 gap-2">
            <label class="text-xs text-gray-600">
              时间窗口（小时）
              <input
                v-model.number="observabilityFilter.hours"
                type="number"
                min="1"
                max="168"
                class="w-full border rounded px-2 py-1 mt-1"
              />
            </label>
            <label class="text-xs text-gray-600">
              返回上限
              <input
                v-model.number="observabilityFilter.limit"
                type="number"
                min="1"
                max="200"
                class="w-full border rounded px-2 py-1 mt-1"
              />
            </label>
            <label class="text-xs text-gray-600">
              会话 ID（可选）
              <input
                v-model.trim="observabilityFilter.debateSessionId"
                type="text"
                placeholder="例如 123"
                class="w-full border rounded px-2 py-1 mt-1"
              />
            </label>
            <div class="flex items-end gap-2">
              <button
                @click="refreshJudgeObservability"
                :disabled="observabilityLoading || !canJudgeReview"
                class="px-3 py-2 rounded bg-slate-700 text-white text-xs disabled:opacity-50"
              >
                查询
              </button>
              <button
                @click="clearObservabilitySessionFilter"
                :disabled="observabilityLoading"
                class="px-3 py-2 rounded border text-xs bg-white hover:bg-gray-100 disabled:opacity-50"
              >
                清空会话
              </button>
            </div>
          </div>

          <div
            v-if="observabilityThresholdSettingsOpen"
            class="rounded border border-slate-200 bg-slate-50 p-3 space-y-3"
          >
            <div class="text-xs font-semibold text-slate-800">异常阈值配置</div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-2">
              <label class="text-xs text-gray-600">
                低成功率阈值（%）
                <input
                  v-model.number="observabilityThresholds.lowSuccessRateThreshold"
                  type="number"
                  min="1"
                  max="99.99"
                  step="0.01"
                  class="w-full border rounded px-2 py-1 mt-1"
                />
              </label>
              <label class="text-xs text-gray-600">
                高重试阈值
                <input
                  v-model.number="observabilityThresholds.highRetryThreshold"
                  type="number"
                  min="0.1"
                  max="10"
                  step="0.1"
                  class="w-full border rounded px-2 py-1 mt-1"
                />
              </label>
              <label class="text-xs text-gray-600">
                高合并事件阈值
                <input
                  v-model.number="observabilityThresholds.highCoalescedThreshold"
                  type="number"
                  min="0.1"
                  max="20"
                  step="0.1"
                  class="w-full border rounded px-2 py-1 mt-1"
                />
              </label>
              <label class="text-xs text-gray-600">
                高 DB 延迟阈值（ms）
                <input
                  v-model.number="observabilityThresholds.highDbLatencyThresholdMs"
                  type="number"
                  min="1"
                  max="60000"
                  class="w-full border rounded px-2 py-1 mt-1"
                />
              </label>
              <label class="text-xs text-gray-600">
                低缓存命中率阈值（%）
                <input
                  v-model.number="observabilityThresholds.lowCacheHitRateThreshold"
                  type="number"
                  min="0"
                  max="99.99"
                  step="0.01"
                  class="w-full border rounded px-2 py-1 mt-1"
                />
              </label>
              <label class="text-xs text-gray-600">
                缓存检查最小请求数
                <input
                  v-model.number="observabilityThresholds.minRequestForCacheHitCheck"
                  type="number"
                  min="1"
                  max="1000000"
                  class="w-full border rounded px-2 py-1 mt-1"
                />
              </label>
            </div>
            <div class="flex items-center gap-2">
              <button
                @click="persistObservabilityThresholds"
                class="px-3 py-1 rounded bg-slate-700 text-white text-xs"
              >
                保存阈值
              </button>
              <button
                @click="resetObservabilityThresholds"
                class="px-3 py-1 rounded border text-xs bg-white hover:bg-gray-100"
              >
                恢复默认
              </button>
              <div v-if="observabilityThresholdNoticeText" class="text-xs text-emerald-700">
                {{ observabilityThresholdNoticeText }}
              </div>
            </div>
          </div>

          <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-8 gap-2 text-xs">
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">Requests</div>
              <div class="font-semibold text-gray-900">{{ observabilityMetrics.requestTotal }}</div>
            </div>
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">Cache Hit</div>
              <div class="font-semibold text-emerald-700">{{ observabilityMetrics.cacheHitTotal }}</div>
            </div>
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">Cache Miss</div>
              <div class="font-semibold text-amber-700">{{ observabilityMetrics.cacheMissTotal }}</div>
            </div>
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">Hit Rate</div>
              <div class="font-semibold text-indigo-700">{{ formatPercent(observabilityMetrics.cacheHitRate) }}</div>
            </div>
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">Miss Rate</div>
              <div class="font-semibold text-orange-700">{{ formatPercent(observabilityCacheMissRate) }}</div>
            </div>
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">DB Queries</div>
              <div class="font-semibold text-gray-900">{{ observabilityMetrics.dbQueryTotal }}</div>
            </div>
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">DB Errors</div>
              <div class="font-semibold text-rose-700">{{ observabilityMetrics.dbErrorTotal }}</div>
            </div>
            <div class="rounded border bg-gray-50 p-2">
              <div class="text-gray-500">Avg DB Latency</div>
              <div class="font-semibold text-gray-900">{{ formatDecimal(observabilityMetrics.avgDbLatencyMs) }} ms</div>
            </div>
          </div>

          <div class="rounded border border-slate-200 bg-slate-50 p-3 space-y-2">
            <div class="text-xs font-semibold text-slate-800">SLI/SLO 健康度</div>
            <div
              v-if="observabilitySliSnapshot.dangerCount > 0"
              class="rounded border border-rose-200 bg-rose-50 text-rose-700 p-2 text-xs"
            >
              当前存在 {{ observabilitySliSnapshot.dangerCount }} 项严重偏离 SLO，请优先处理。
            </div>
            <div
              v-else-if="observabilitySliSnapshot.warningCount > 0"
              class="rounded border border-amber-200 bg-amber-50 text-amber-700 p-2 text-xs"
            >
              当前存在 {{ observabilitySliSnapshot.warningCount }} 项预警，建议关注波动趋势。
            </div>
            <div
              v-else
              class="rounded border border-emerald-200 bg-emerald-50 text-emerald-700 p-2 text-xs"
            >
              当前 SLI 指标均满足目标阈值。
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-2 text-xs">
              <div
                v-for="item in observabilitySliSnapshot.indicators"
                :key="`sli-${item.code}`"
                class="rounded border p-2"
                :class="observabilitySliStatusClass(item.status)"
              >
                <div class="font-semibold">{{ item.label }}</div>
                <div class="mt-1">
                  当前: {{ formatDecimal(item.value) }}{{ item.unit }} ·
                  目标: {{ item.comparator === 'lte' ? '<=' : '>=' }}{{ formatDecimal(item.target) }}{{ item.unit }}
                </div>
                <div class="mt-1 text-[11px] opacity-90">
                  状态: {{ observabilitySliStatusText(item.status) }}
                </div>
              </div>
            </div>
          </div>

          <div
            v-if="observabilitySuppressedAnomalyCount > 0"
            class="rounded border border-indigo-200 bg-indigo-50 text-indigo-700 p-2 text-xs flex items-center justify-between gap-2"
          >
            <div>当前有 {{ observabilitySuppressedAnomalyCount }} 条异常处于抑制窗口内。</div>
            <button
              @click="clearAllObservabilitySuppression"
              class="px-2 py-1 rounded border border-indigo-300 bg-white hover:bg-indigo-100"
            >
              清除全部抑制
            </button>
          </div>
          <div v-if="observabilityAnomalyNoticeText" class="text-xs text-emerald-700">
            {{ observabilityAnomalyNoticeText }}
          </div>

          <div v-if="observabilityAnomalies.length > 0" class="space-y-2">
            <div
              v-for="anomaly in observabilityAnomalies"
              :key="anomaly.stateKey || anomaly.code"
              class="border rounded p-2 text-xs flex items-start justify-between gap-2"
              :class="observabilityAlertClass(anomaly.level)"
            >
              <div class="space-y-1">
                <div>{{ anomaly.text }}</div>
                <div
                  v-if="Array.isArray(anomaly.sessionIds) && anomaly.sessionIds.length > 0"
                  class="text-[11px] opacity-80"
                >
                  sessions: {{ anomaly.sessionIds.join(', ') }}
                </div>
                <div v-if="anomaly.acknowledgedAtMs > 0" class="text-[11px] opacity-80">
                  已处理: {{ formatDateTime(anomaly.acknowledgedAtMs) }}
                </div>
              </div>
              <div class="flex flex-col items-end gap-1">
                <button
                  @click="applyObservabilityAnomaly(anomaly)"
                  :disabled="
                    !canApplyAnomalyAction(anomaly)
                    || observabilityLoading
                    || observabilityMetricsLoading
                    || reviewLoading
                  "
                  class="px-2 py-1 rounded border border-current bg-white/70 hover:bg-white disabled:opacity-50 whitespace-nowrap"
                >
                  {{ anomalyActionLabel(anomaly) }}
                </button>
                <button
                  @click="markObservabilityAnomalyHandled(anomaly)"
                  class="px-2 py-1 rounded border border-current bg-white/70 hover:bg-white whitespace-nowrap"
                >
                  标记已处理
                </button>
                <button
                  @click="suppressObservabilityAnomaly(anomaly, 1)"
                  class="px-2 py-1 rounded border border-current bg-white/70 hover:bg-white whitespace-nowrap"
                >
                  抑制 1h
                </button>
                <button
                  v-if="anomalyHasState(anomaly)"
                  @click="clearObservabilityAnomalyState(anomaly)"
                  class="px-2 py-1 rounded border border-current bg-white/70 hover:bg-white whitespace-nowrap"
                >
                  清除标记
                </button>
              </div>
            </div>
          </div>
          <div v-else class="rounded border border-emerald-200 bg-emerald-50 text-emerald-700 p-2 text-xs">
            当前窗口未发现明显异常。
          </div>

          <div class="rounded border border-slate-200 bg-slate-50 p-3 space-y-2">
            <div class="flex items-center justify-between gap-2">
              <div class="text-xs font-semibold text-slate-800">异常码趋势（最近 24h 对比）</div>
              <button
                @click="clearObservabilityAnomalyTrendHistory"
                class="px-2 py-1 rounded border text-xs bg-white hover:bg-gray-100"
              >
                清空趋势历史
              </button>
            </div>
            <div class="text-[11px] text-gray-600">
              latest: {{ formatDateTime(observabilityAnomalyTrendSummary.latestAtMs) }} ·
              samples(24h/前24h): {{ observabilityAnomalyTrendSummary.recentSamples }}/{{ observabilityAnomalyTrendSummary.previousSamples }}
            </div>
            <div v-if="observabilityTrendNoticeText" class="text-xs text-emerald-700">
              {{ observabilityTrendNoticeText }}
            </div>
            <div class="flex flex-wrap gap-1">
              <span
                v-for="row in observabilityAnomalyCodeStats.rows"
                :key="`current-code-${row.code}`"
                class="px-2 py-1 rounded bg-white border text-[11px] text-gray-700"
              >
                {{ row.code }}: {{ row.count }}
              </span>
              <span
                v-if="observabilityAnomalyCodeStats.rows.length === 0"
                class="px-2 py-1 rounded bg-white border text-[11px] text-gray-500"
              >
                当前窗口无异常码
              </span>
            </div>
            <div class="overflow-x-auto">
              <table class="min-w-full text-xs">
                <thead>
                  <tr class="text-left text-gray-500 border-b">
                    <th class="py-2 pr-3">Code</th>
                    <th class="py-2 pr-3 text-right">最近24h均值</th>
                    <th class="py-2 pr-3 text-right">前24h均值</th>
                    <th class="py-2 pr-3 text-right">Delta</th>
                    <th class="py-2 pr-3">趋势</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="row in observabilityAnomalyTrendSummary.rows"
                    :key="`trend-${row.code}`"
                    class="border-b last:border-b-0"
                  >
                    <td class="py-2 pr-3 text-gray-900">{{ row.code }}</td>
                    <td class="py-2 pr-3 text-right text-gray-900">{{ formatDecimal(row.recentAvg) }}</td>
                    <td class="py-2 pr-3 text-right text-gray-900">{{ formatDecimal(row.previousAvg) }}</td>
                    <td class="py-2 pr-3 text-right" :class="observabilityTrendClass(row)">
                      {{ row.delta > 0 ? '+' : '' }}{{ formatDecimal(row.delta) }}
                    </td>
                    <td class="py-2 pr-3" :class="observabilityTrendClass(row)">
                      {{ observabilityTrendText(row) }}
                    </td>
                  </tr>
                  <tr v-if="observabilityAnomalyTrendSummary.rows.length === 0">
                    <td colspan="5" class="py-4 text-center text-gray-500">暂无趋势数据（请刷新观测后生成快照）</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div class="overflow-x-auto">
            <table class="min-w-full text-xs">
              <thead>
                <tr class="text-left text-gray-500 border-b">
                  <th class="py-2 pr-3">Session</th>
                  <th class="py-2 pr-3">Source</th>
                  <th class="py-2 pr-3 text-right">Success</th>
                  <th class="py-2 pr-3 text-right">Runs</th>
                  <th class="py-2 pr-3 text-right">Failure</th>
                  <th class="py-2 pr-3 text-right">Avg Retry</th>
                  <th class="py-2 pr-3 text-right">Avg Coalesced</th>
                  <th class="py-2 pr-3">Last Seen</th>
                  <th class="py-2 pr-3">Action</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="row in observabilityRows"
                  :key="observabilityRowKey(row)"
                  class="border-b last:border-b-0"
                >
                  <td class="py-2 pr-3 text-gray-900">{{ row.debateSessionId || '-' }}</td>
                  <td class="py-2 pr-3 text-gray-700">{{ row.sourceEventType || '-' }}</td>
                  <td class="py-2 pr-3 text-right text-gray-900">{{ formatPercent(row.successRate) }}</td>
                  <td class="py-2 pr-3 text-right text-gray-900">{{ row.totalRuns || 0 }}</td>
                  <td class="py-2 pr-3 text-right text-gray-900">{{ row.failureRuns || 0 }}</td>
                  <td class="py-2 pr-3 text-right text-gray-900">{{ formatDecimal(row.avgRetryCount) }}</td>
                  <td class="py-2 pr-3 text-right text-gray-900">{{ formatDecimal(row.avgCoalescedEvents) }}</td>
                  <td class="py-2 pr-3 text-gray-700">{{ formatDateTime(row.lastSeenAtMs) }}</td>
                  <td class="py-2 pr-3">
                    <div class="flex flex-wrap gap-1">
                      <button
                        @click="focusObservabilitySession(row.debateSessionId)"
                        class="px-2 py-1 rounded border border-gray-300 bg-white hover:bg-gray-100"
                      >
                        按会话过滤
                      </button>
                      <button
                        @click="openSessionJudgeReport(row.debateSessionId)"
                        class="px-2 py-1 rounded border border-gray-300 bg-white hover:bg-gray-100"
                      >
                        查看判决
                      </button>
                    </div>
                  </td>
                </tr>
                <tr v-if="observabilityRows.length === 0">
                  <td colspan="9" class="py-4 text-center text-gray-500">当前窗口暂无汇总数据</td>
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
import { normalizeJudgeRefreshSummaryQuery } from '../judge-refresh-summary-utils';
import {
  appendObservabilityAnomalyTrendSnapshot,
  buildObservabilitySliSnapshot,
  buildObservabilityAnomalyCodeStats,
  buildObservabilityAnomalyStateKey,
  DEFAULT_OBSERVABILITY_SLO_TARGETS,
  DEFAULT_OBSERVABILITY_THRESHOLDS,
  buildJudgeObservabilityAnomalies,
  normalizeObservabilitySessionId,
  normalizeObservabilityAnomalyStateMap,
  normalizeObservabilityAnomalyTrendHistory,
  normalizeObservabilitySloTargets,
  normalizeObservabilityThresholds,
  projectObservabilityAnomalies,
  summarizeObservabilityAnomalyTrend,
} from '../judge-observability-utils';
import {
  emptyOpsRbacMe,
  normalizeOpsRbacMe,
  resolveOpsErrorText,
} from '../ops-permission-utils';

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

const OBSERVABILITY_THRESHOLDS_STORAGE_KEY = 'ops_observability_thresholds_v1';
const OBSERVABILITY_ANOMALY_STATE_STORAGE_KEY = 'ops_observability_anomaly_state_v1';
const OBSERVABILITY_ANOMALY_TREND_HISTORY_STORAGE_KEY = 'ops_observability_anomaly_trend_history_v1';

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
      observabilityLoading: false,
      observabilityMetricsLoading: false,
      createTopicLoading: false,
      createSessionLoading: false,
      updateTopicLoading: false,
      updateSessionLoading: false,
      quickUpdateSessionId: 0,
      rejudgeReviewSessionId: 0,
      errorText: '',
      reviewErrorText: '',
      roleErrorText: '',
      observabilityErrorText: '',
      observabilityMetricsErrorText: '',
      observabilityThresholdNoticeText: '',
      observabilityAnomalyNoticeText: '',
      observabilityTrendNoticeText: '',
      topics: [],
      sessions: [],
      reviewRows: [],
      roleAssignments: [],
      observabilityRows: [],
      reviewMeta: {
        scannedCount: 0,
        returnedCount: 0,
      },
      observabilityUpdatedAt: null,
      observabilityMetricsUpdatedAt: null,
      observabilityMetrics: {
        requestTotal: 0,
        cacheHitTotal: 0,
        cacheMissTotal: 0,
        cacheHitRate: 0,
        dbQueryTotal: 0,
        dbErrorTotal: 0,
        avgDbLatencyMs: 0,
        lastDbLatencyMs: 0,
      },
      opsRbacMe: emptyOpsRbacMe(),
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
      observabilityFilter: {
        hours: 24,
        limit: 20,
        debateSessionId: '',
      },
      observabilityThresholds: normalizeObservabilityThresholds(DEFAULT_OBSERVABILITY_THRESHOLDS),
      observabilitySloTargets: normalizeObservabilitySloTargets(DEFAULT_OBSERVABILITY_SLO_TARGETS),
      observabilityThresholdSettingsOpen: false,
      observabilityAnomalyState: {},
      observabilityAnomalyTrendHistory: [],
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
  computed: {
    canDebateManage() {
      return !!this.opsRbacMe?.permissions?.debateManage;
    },
    canJudgeReview() {
      return !!this.opsRbacMe?.permissions?.judgeReview;
    },
    canJudgeRejudge() {
      return !!this.opsRbacMe?.permissions?.judgeRejudge;
    },
    canRoleManage() {
      return !!this.opsRbacMe?.permissions?.roleManage;
    },
    observabilityAnomaliesRaw() {
      return buildJudgeObservabilityAnomalies({
        rows: this.observabilityRows,
        metrics: this.observabilityMetrics,
      }, this.observabilityThresholds);
    },
    observabilityAnomalyProjection() {
      return projectObservabilityAnomalies(
        this.observabilityAnomaliesRaw,
        this.observabilityAnomalyState,
      );
    },
    observabilityAnomalies() {
      return this.observabilityAnomalyProjection.visible;
    },
    observabilitySuppressedAnomalyCount() {
      return Number(this.observabilityAnomalyProjection.suppressedCount || 0);
    },
    observabilityAnomalyCodeStats() {
      return buildObservabilityAnomalyCodeStats(this.observabilityAnomaliesRaw);
    },
    observabilityAnomalyTrendSummary() {
      return summarizeObservabilityAnomalyTrend(this.observabilityAnomalyTrendHistory);
    },
    observabilitySliSnapshot() {
      return buildObservabilitySliSnapshot({
        rows: this.observabilityRows,
        metrics: this.observabilityMetrics,
      }, this.observabilitySloTargets);
    },
    observabilityCacheMissRate() {
      const missRate = 100 - Number(this.observabilityMetrics?.cacheHitRate || 0);
      return Math.max(0, missRate);
    },
  },
  methods: {
    resolveErrorText(error, fallback) {
      return resolveOpsErrorText(error, fallback);
    },
    async syncOpsRbacSnapshot() {
      const ret = await this.$store.dispatch('getOpsRbacMe');
      this.opsRbacMe = normalizeOpsRbacMe(ret);
    },
    formatDateTime(value) {
      if (!value) {
        return '-';
      }
      const date = new Date(value);
      return Number.isNaN(date.getTime()) ? '-' : date.toLocaleString();
    },
    formatPercent(value) {
      const n = Number(value);
      if (!Number.isFinite(n)) {
        return '-';
      }
      return `${n.toFixed(2)}%`;
    },
    formatDecimal(value) {
      const n = Number(value);
      if (!Number.isFinite(n)) {
        return '-';
      }
      return n.toFixed(2);
    },
    observabilitySliStatusClass(status) {
      if (status === 'healthy') {
        return 'bg-emerald-50 border-emerald-200 text-emerald-700';
      }
      if (status === 'danger') {
        return 'bg-rose-50 border-rose-200 text-rose-700';
      }
      return 'bg-amber-50 border-amber-200 text-amber-700';
    },
    observabilitySliStatusText(status) {
      if (status === 'healthy') {
        return '达标';
      }
      if (status === 'danger') {
        return '严重偏离';
      }
      return '预警';
    },
    observabilityRowKey(row) {
      return `${row?.debateSessionId || ''}:${row?.sourceEventType || ''}`;
    },
    observabilityAlertClass(level) {
      if (level === 'danger') {
        return 'bg-red-50 border-red-200 text-red-700';
      }
      return 'bg-amber-50 border-amber-200 text-amber-800';
    },
    toggleObservabilityThresholdSettings() {
      this.observabilityThresholdSettingsOpen = !this.observabilityThresholdSettingsOpen;
    },
    applyOpsObservabilityConfigPayload(payload, { persistLocal = true } = {}) {
      const thresholds = normalizeObservabilityThresholds(payload?.thresholds || DEFAULT_OBSERVABILITY_THRESHOLDS);
      const anomalyState = normalizeObservabilityAnomalyStateMap(payload?.anomalyState || {});
      this.observabilityThresholds = thresholds;
      this.observabilityAnomalyState = anomalyState;
      if (!persistLocal) {
        return;
      }
      localStorage.setItem(
        OBSERVABILITY_THRESHOLDS_STORAGE_KEY,
        JSON.stringify(thresholds),
      );
      if (Object.keys(anomalyState).length === 0) {
        localStorage.removeItem(OBSERVABILITY_ANOMALY_STATE_STORAGE_KEY);
      } else {
        localStorage.setItem(
          OBSERVABILITY_ANOMALY_STATE_STORAGE_KEY,
          JSON.stringify(anomalyState),
        );
      }
    },
    async syncOpsObservabilityConfigFromServer() {
      if (!this.canJudgeReview) {
        return;
      }
      try {
        const payload = await this.$store.dispatch('getOpsObservabilityConfig');
        this.applyOpsObservabilityConfigPayload(payload);
      } catch (error) {
        this.loadObservabilityThresholds();
        this.loadObservabilityAnomalyState();
        this.observabilityThresholdNoticeText = `${this.resolveErrorText(error, '观测配置同步失败')}（当前使用本地配置）`;
      }
    },
    loadObservabilityThresholds() {
      const raw = localStorage.getItem(OBSERVABILITY_THRESHOLDS_STORAGE_KEY);
      if (!raw) {
        this.observabilityThresholds = normalizeObservabilityThresholds(
          DEFAULT_OBSERVABILITY_THRESHOLDS,
        );
        return;
      }
      try {
        const parsed = JSON.parse(raw);
        this.observabilityThresholds = normalizeObservabilityThresholds(parsed);
      } catch (_error) {
        this.observabilityThresholds = normalizeObservabilityThresholds(
          DEFAULT_OBSERVABILITY_THRESHOLDS,
        );
      }
    },
    persistObservabilityThresholds() {
      const normalized = normalizeObservabilityThresholds(this.observabilityThresholds);
      this.observabilityThresholds = normalized;
      localStorage.setItem(
        OBSERVABILITY_THRESHOLDS_STORAGE_KEY,
        JSON.stringify(normalized),
      );
      this.observabilityThresholdNoticeText = '阈值已保存';
      this.syncOpsObservabilityThresholdsToServer();
    },
    async syncOpsObservabilityThresholdsToServer() {
      if (!this.canJudgeReview) {
        return;
      }
      try {
        const payload = await this.$store.dispatch(
          'upsertOpsObservabilityThresholds',
          this.observabilityThresholds,
        );
        this.applyOpsObservabilityConfigPayload(payload);
        this.observabilityThresholdNoticeText = '阈值已保存（已同步服务端）';
      } catch (error) {
        this.observabilityThresholdNoticeText = `${this.resolveErrorText(error, '阈值服务端同步失败')}（当前仅本地生效）`;
      }
    },
    resetObservabilityThresholds() {
      this.observabilityThresholds = normalizeObservabilityThresholds(
        DEFAULT_OBSERVABILITY_THRESHOLDS,
      );
      localStorage.removeItem(OBSERVABILITY_THRESHOLDS_STORAGE_KEY);
      this.observabilityThresholdNoticeText = '已恢复默认阈值';
      this.syncOpsObservabilityThresholdsToServer();
    },
    loadObservabilityAnomalyState() {
      const raw = localStorage.getItem(OBSERVABILITY_ANOMALY_STATE_STORAGE_KEY);
      if (!raw) {
        this.observabilityAnomalyState = {};
        return;
      }
      try {
        const parsed = JSON.parse(raw);
        this.observabilityAnomalyState = normalizeObservabilityAnomalyStateMap(parsed);
      } catch (_error) {
        this.observabilityAnomalyState = {};
      }
    },
    persistObservabilityAnomalyState() {
      const normalized = normalizeObservabilityAnomalyStateMap(this.observabilityAnomalyState);
      this.observabilityAnomalyState = normalized;
      if (Object.keys(normalized).length === 0) {
        localStorage.removeItem(OBSERVABILITY_ANOMALY_STATE_STORAGE_KEY);
        this.syncOpsObservabilityAnomalyStateToServer();
        return;
      }
      localStorage.setItem(
        OBSERVABILITY_ANOMALY_STATE_STORAGE_KEY,
        JSON.stringify(normalized),
      );
      this.syncOpsObservabilityAnomalyStateToServer();
    },
    async syncOpsObservabilityAnomalyStateToServer() {
      if (!this.canJudgeReview) {
        return;
      }
      try {
        const payload = await this.$store.dispatch(
          'upsertOpsObservabilityAnomalyState',
          { anomalyState: this.observabilityAnomalyState },
        );
        this.applyOpsObservabilityConfigPayload(payload);
        this.observabilityAnomalyNoticeText = '异常状态已同步服务端';
      } catch (error) {
        this.observabilityAnomalyNoticeText = `${this.resolveErrorText(error, '异常状态服务端同步失败')}（当前仅本地生效）`;
      }
    },
    anomalyHasState(anomaly) {
      const key = buildObservabilityAnomalyStateKey(anomaly);
      if (!key) {
        return false;
      }
      const item = this.observabilityAnomalyState[key];
      return !!item && (Number(item.acknowledgedAtMs || 0) > 0 || Number(item.suppressUntilMs || 0) > 0);
    },
    markObservabilityAnomalyHandled(anomaly) {
      const key = buildObservabilityAnomalyStateKey(anomaly);
      if (!key) {
        return;
      }
      const current = this.observabilityAnomalyState[key] || {};
      this.observabilityAnomalyState = {
        ...this.observabilityAnomalyState,
        [key]: {
          acknowledgedAtMs: Date.now(),
          suppressUntilMs: Number(current.suppressUntilMs || 0),
        },
      };
      this.persistObservabilityAnomalyState();
      this.observabilityAnomalyNoticeText = '异常已标记为已处理';
    },
    suppressObservabilityAnomaly(anomaly, hoursRaw = 1) {
      const key = buildObservabilityAnomalyStateKey(anomaly);
      if (!key) {
        return;
      }
      const hours = Math.max(1, Math.trunc(Number(hoursRaw || 1)));
      const current = this.observabilityAnomalyState[key] || {};
      this.observabilityAnomalyState = {
        ...this.observabilityAnomalyState,
        [key]: {
          acknowledgedAtMs: Number(current.acknowledgedAtMs || 0),
          suppressUntilMs: Date.now() + hours * 60 * 60 * 1000,
        },
      };
      this.persistObservabilityAnomalyState();
      this.observabilityAnomalyNoticeText = `异常已抑制 ${hours} 小时`;
    },
    clearObservabilityAnomalyState(anomaly) {
      const key = buildObservabilityAnomalyStateKey(anomaly);
      if (!key) {
        return;
      }
      const nextState = { ...this.observabilityAnomalyState };
      delete nextState[key];
      this.observabilityAnomalyState = nextState;
      this.persistObservabilityAnomalyState();
      this.observabilityAnomalyNoticeText = '异常标记已清除';
    },
    clearAllObservabilitySuppression() {
      const now = Date.now();
      const normalized = normalizeObservabilityAnomalyStateMap(this.observabilityAnomalyState, now);
      const nextState = {};
      Object.entries(normalized).forEach(([key, value]) => {
        const acknowledgedAtMs = Number(value?.acknowledgedAtMs || 0);
        if (acknowledgedAtMs > 0) {
          nextState[key] = {
            acknowledgedAtMs,
            suppressUntilMs: 0,
          };
        }
      });
      this.observabilityAnomalyState = nextState;
      this.persistObservabilityAnomalyState();
      this.observabilityAnomalyNoticeText = '已清除全部抑制窗口';
    },
    loadObservabilityAnomalyTrendHistory() {
      const raw = localStorage.getItem(OBSERVABILITY_ANOMALY_TREND_HISTORY_STORAGE_KEY);
      if (!raw) {
        this.observabilityAnomalyTrendHistory = [];
        return;
      }
      try {
        const parsed = JSON.parse(raw);
        this.observabilityAnomalyTrendHistory = normalizeObservabilityAnomalyTrendHistory(parsed);
      } catch (_error) {
        this.observabilityAnomalyTrendHistory = [];
      }
    },
    persistObservabilityAnomalyTrendHistory() {
      const normalized = normalizeObservabilityAnomalyTrendHistory(this.observabilityAnomalyTrendHistory);
      this.observabilityAnomalyTrendHistory = normalized;
      if (normalized.length === 0) {
        localStorage.removeItem(OBSERVABILITY_ANOMALY_TREND_HISTORY_STORAGE_KEY);
        return;
      }
      localStorage.setItem(
        OBSERVABILITY_ANOMALY_TREND_HISTORY_STORAGE_KEY,
        JSON.stringify(normalized),
      );
    },
    captureObservabilityTrendSnapshot() {
      const now = Date.now();
      this.observabilityAnomalyTrendHistory = appendObservabilityAnomalyTrendSnapshot(
        this.observabilityAnomalyTrendHistory,
        this.observabilityAnomaliesRaw,
        now,
      );
      this.persistObservabilityAnomalyTrendHistory();
      this.observabilityTrendNoticeText = '';
    },
    clearObservabilityAnomalyTrendHistory() {
      this.observabilityAnomalyTrendHistory = [];
      localStorage.removeItem(OBSERVABILITY_ANOMALY_TREND_HISTORY_STORAGE_KEY);
      this.observabilityTrendNoticeText = '趋势历史已清空';
    },
    observabilityTrendText(row) {
      if (!row) {
        return '持平';
      }
      if (row.trend === 'up') {
        return '上升';
      }
      if (row.trend === 'down') {
        return '下降';
      }
      return '持平';
    },
    observabilityTrendClass(row) {
      if (!row) {
        return 'text-gray-700';
      }
      if (row.trend === 'up') {
        return 'text-rose-700';
      }
      if (row.trend === 'down') {
        return 'text-emerald-700';
      }
      return 'text-gray-700';
    },
    anomalyActionLabel(anomaly) {
      if (!anomaly) {
        return '';
      }
      if (anomaly.action === 'review_sessions') {
        return '联动审阅';
      }
      if (anomaly.action === 'refresh_metrics') {
        return '刷新指标';
      }
      return '刷新汇总';
    },
    canApplyAnomalyAction(anomaly) {
      if (!anomaly || !this.canJudgeReview) {
        return false;
      }
      if (anomaly.action === 'review_sessions') {
        return Array.isArray(anomaly.sessionIds) && anomaly.sessionIds.length > 0;
      }
      return true;
    },
    setReviewWindowFromObservabilityHours() {
      const hours = Number(this.observabilityFilter.hours || 24);
      const now = new Date();
      const from = new Date(now.getTime() - Math.max(1, hours) * 60 * 60 * 1000);
      this.reviewFilter.fromLocal = toLocalInputValue(from);
      this.reviewFilter.toLocal = toLocalInputValue(now);
      this.reviewFilter.anomalyOnly = true;
    },
    async applyObservabilityAnomaly(anomaly) {
      if (!this.canApplyAnomalyAction(anomaly)) {
        return;
      }
      if (anomaly.action === 'review_sessions') {
        const sessionId = normalizeObservabilitySessionId(anomaly.sessionIds[0]);
        if (sessionId) {
          this.observabilityFilter.debateSessionId = String(sessionId);
          await this.refreshJudgeObservability();
          this.setReviewWindowFromObservabilityHours();
          await this.refreshJudgeReviews();
          return;
        }
      }
      if (anomaly.action === 'refresh_metrics') {
        await this.refreshJudgeObservabilityMetrics();
        return;
      }
      await this.refreshJudgeObservability();
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
    platformUsers() {
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
      if (!this.canRoleManage) {
        this.roleAssignments = [];
        this.roleErrorText = '仅 platform admin 可管理 Ops 角色';
        return;
      }
      this.roleLoading = true;
      this.roleErrorText = '';
      try {
        const response = await this.$store.dispatch('listOpsRoleAssignments');
        this.roleAssignments = Array.isArray(response?.items) ? response.items : [];
      } catch (error) {
        this.roleErrorText = this.resolveErrorText(error, '加载角色列表失败');
      } finally {
        this.roleLoading = false;
      }
    },
    async upsertRoleAssignment() {
      if (!this.canRoleManage) {
        this.roleErrorText = '仅 platform admin 可管理 Ops 角色';
        return;
      }
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
        this.roleErrorText = this.resolveErrorText(error, '授予角色失败');
      } finally {
        this.roleLoading = false;
      }
    },
    async revokeRoleAssignment(userIdRaw) {
      if (!this.canRoleManage) {
        this.roleErrorText = '仅 platform admin 可管理 Ops 角色';
        return;
      }
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
        this.roleErrorText = this.resolveErrorText(error, '撤销角色失败');
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
      if (!this.canJudgeReview) {
        this.reviewRows = [];
        this.reviewMeta = {
          scannedCount: 0,
          returnedCount: 0,
        };
        this.reviewErrorText = '当前账号没有判决审阅权限';
        return;
      }
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
        this.reviewErrorText = this.resolveErrorText(error, '加载判决审阅列表失败');
      } finally {
        this.reviewLoading = false;
      }
    },
    async triggerJudgeRejudge(sessionIdRaw) {
      if (!this.canJudgeRejudge) {
        this.reviewErrorText = '当前账号没有触发复核权限';
        return;
      }
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
        this.reviewErrorText = this.resolveErrorText(error, '触发复核失败');
      } finally {
        this.rejudgeReviewSessionId = 0;
      }
    },
    buildObservabilityPayload() {
      return normalizeJudgeRefreshSummaryQuery({
        hours: this.observabilityFilter.hours,
        limit: this.observabilityFilter.limit,
        debateSessionId: this.observabilityFilter.debateSessionId,
      });
    },
    async refreshJudgeObservabilityMetrics({ silent = false, suppressOnError = false } = {}) {
      if (!this.canJudgeReview) {
        this.observabilityMetrics = {
          requestTotal: 0,
          cacheHitTotal: 0,
          cacheMissTotal: 0,
          cacheHitRate: 0,
          dbQueryTotal: 0,
          dbErrorTotal: 0,
          avgDbLatencyMs: 0,
          lastDbLatencyMs: 0,
        };
        this.observabilityMetricsUpdatedAt = null;
        this.observabilityMetricsErrorText = '当前账号没有裁判观测权限';
        return;
      }
      if (!silent) {
        this.observabilityMetricsLoading = true;
        this.observabilityMetricsErrorText = '';
      }
      try {
        const payload = await this.$store.dispatch('fetchJudgeRefreshSummaryMetrics');
        this.observabilityMetrics = {
          ...this.observabilityMetrics,
          ...(payload || {}),
        };
        this.observabilityMetricsUpdatedAt = Date.now();
      } catch (error) {
        if (!silent && !suppressOnError) {
          this.observabilityMetricsErrorText = this.resolveErrorText(error, '加载观测指标失败');
        }
      } finally {
        if (!silent) {
          this.observabilityMetricsLoading = false;
        }
      }
    },
    async refreshJudgeObservability({ silent = false } = {}) {
      if (!this.canJudgeReview) {
        this.observabilityRows = [];
        this.observabilityUpdatedAt = null;
        this.observabilityErrorText = '当前账号没有裁判观测权限';
        return;
      }
      if (!silent) {
        this.observabilityLoading = true;
        this.observabilityErrorText = '';
      }
      try {
        const payload = this.buildObservabilityPayload();
        const response = await this.$store.dispatch('fetchJudgeRefreshSummary', payload);
        this.observabilityFilter.hours = Number(response?.windowHours || payload.hours);
        this.observabilityFilter.limit = Number(response?.limit || payload.limit);
        this.observabilityRows = Array.isArray(response?.rows) ? response.rows : [];
        this.observabilityUpdatedAt = Date.now();
        await this.refreshJudgeObservabilityMetrics({ silent, suppressOnError: silent });
        this.captureObservabilityTrendSnapshot();
      } catch (error) {
        if (!silent) {
          this.observabilityErrorText = this.resolveErrorText(error, '加载裁判观测汇总失败');
        }
      } finally {
        if (!silent) {
          this.observabilityLoading = false;
        }
      }
    },
    async focusObservabilitySession(sessionIdRaw) {
      const sessionId = normalizeObservabilitySessionId(sessionIdRaw);
      if (!sessionId) {
        return;
      }
      this.observabilityFilter.debateSessionId = String(sessionId);
      await this.refreshJudgeObservability();
    },
    clearObservabilitySessionFilter() {
      this.observabilityFilter.debateSessionId = '';
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
        await this.syncOpsRbacSnapshot();
        const [topics, sessions, reviews, roleAssignments] = await Promise.all([
          this.$store.dispatch('listDebateTopics', { activeOnly: false, limit: 200 }),
          this.$store.dispatch('listDebateSessions', { limit: 200 }),
          this.canJudgeReview
            ? this.$store.dispatch('listJudgeReviewsOps', this.buildJudgeReviewPayload())
            : Promise.resolve({ scannedCount: 0, returnedCount: 0, items: [] }),
          this.canRoleManage
            ? this.$store.dispatch('listOpsRoleAssignments')
            : Promise.resolve({ items: [] }),
        ]);
        this.topics = topics || [];
        this.sessions = sessions || [];
        this.reviewRows = Array.isArray(reviews?.items) ? reviews.items : [];
        this.roleAssignments = Array.isArray(roleAssignments?.items) ? roleAssignments.items : [];
        this.reviewMeta = {
          scannedCount: Number(reviews?.scannedCount || 0),
          returnedCount: Number(reviews?.returnedCount || this.reviewRows.length),
        };
        this.reviewErrorText = this.canJudgeReview ? '' : '当前账号没有判决审阅权限';
        this.roleErrorText = this.canRoleManage ? '' : '仅 platform admin 可管理 Ops 角色';
        if (this.canJudgeReview) {
          this.observabilityErrorText = '';
          this.observabilityMetricsErrorText = '';
          await this.syncOpsObservabilityConfigFromServer();
          await this.refreshJudgeObservability({ silent: true });
        } else {
          this.observabilityRows = [];
          this.observabilityUpdatedAt = null;
          this.observabilityErrorText = '当前账号没有裁判观测权限';
          this.observabilityMetricsErrorText = '当前账号没有裁判观测权限';
        }
        if (!this.topicEditForm.topicId && this.topics.length > 0) {
          this.topicEditForm.topicId = String(this.topics[0].id);
        }
        if (!this.sessionEditForm.sessionId && this.sessions.length > 0) {
          this.sessionEditForm.sessionId = String(this.sessions[0].id);
        }
        this.syncTopicEditFormFromId(this.topicEditForm.topicId);
        this.syncSessionEditFormFromId(this.sessionEditForm.sessionId);
      } catch (error) {
        this.errorText = this.resolveErrorText(error, '刷新失败');
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
      if (!this.canDebateManage) {
        this.errorText = '当前账号没有场次管理权限';
        return;
      }
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
        this.errorText = this.resolveErrorText(error, isCreate ? '创建辩题失败' : '更新辩题失败');
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
      if (!this.canDebateManage) {
        this.errorText = '当前账号没有场次管理权限';
        return;
      }
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
        this.errorText = this.resolveErrorText(error, isCreate ? '创建场次失败' : '更新场次失败');
      } finally {
        if (isCreate) {
          this.createSessionLoading = false;
        } else {
          this.updateSessionLoading = false;
        }
      }
    },
    async quickUpdateSessionStatus(session, nextStatus) {
      if (!this.canDebateManage) {
        this.errorText = '当前账号没有场次管理权限';
        return;
      }
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
        this.errorText = this.resolveErrorText(error, '快速更新场次状态失败');
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
    this.loadObservabilityThresholds();
    this.loadObservabilityAnomalyState();
    this.loadObservabilityAnomalyTrendHistory();
    await this.refreshData();
  },
};
</script>
