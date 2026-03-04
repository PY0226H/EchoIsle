<template>
  <div class="flex h-screen">
    <Sidebar />
    <div class="flex-1 overflow-y-auto bg-gray-50">
      <div class="max-w-4xl mx-auto p-6 space-y-4">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h1 class="text-2xl font-bold text-gray-900">通知中心</h1>
            <p class="text-sm text-gray-600 mt-1">聚合关键赛事通知：判决生成、平局投票决议。</p>
          </div>
          <button
            @click="goTo('/home')"
            class="px-4 py-2 rounded border bg-white hover:bg-gray-100 text-sm"
          >
            返回首页
          </button>
        </div>

        <div class="bg-white border rounded-lg p-4 space-y-3">
          <div class="flex items-center justify-between">
            <div class="text-sm font-semibold text-gray-900">通知列表</div>
            <div class="text-xs text-gray-500">unread: {{ notificationCount }}</div>
          </div>

          <div v-if="notificationCount === 0" class="text-sm text-gray-600">
            当前暂无关键通知，参与一场辩论后会在此展示最新状态。
          </div>

          <div v-else class="space-y-2">
            <button
              v-for="item in notificationItems"
              :key="item.key"
              @click="openNotification(item)"
              class="w-full text-left border rounded p-3 bg-gray-50 hover:bg-gray-100"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-semibold text-gray-900">{{ item.title }}</div>
                <div class="text-xs text-gray-500">{{ formatDateTime(item.createdAtMs) }}</div>
              </div>
              <div class="text-xs text-gray-600 mt-1">{{ item.subtitle }}</div>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import Sidebar from '../components/Sidebar.vue';
import {
  buildNotificationCenterItems,
  countNotificationCenterItems,
} from '../notification-center-utils';

export default {
  components: {
    Sidebar,
  },
  computed: {
    notificationItems() {
      return buildNotificationCenterItems({
        latestJudgeReportEvent: this.$store.getters.getLatestJudgeReportEvent,
        latestDrawVoteResolvedEvent: this.$store.getters.getLatestDrawVoteResolvedEvent,
      });
    },
    notificationCount() {
      return countNotificationCenterItems(this.notificationItems);
    },
  },
  methods: {
    formatDateTime(value) {
      if (!value) {
        return '-';
      }
      const date = new Date(value);
      return Number.isNaN(date.getTime()) ? '-' : date.toLocaleString();
    },
    async goTo(path) {
      if (this.$route.path === path) {
        return;
      }
      await this.$router.push(path);
    },
    async openNotification(item) {
      if (!item?.path) {
        return;
      }
      if (item.query) {
        await this.$router.push({ path: item.path, query: item.query });
        return;
      }
      await this.$router.push(item.path);
    },
  },
};
</script>
