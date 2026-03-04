<template>
  <div class="flex h-screen">
    <Sidebar />
    <div class="flex-1 overflow-y-auto bg-gray-50">
      <div class="max-w-6xl mx-auto p-6 space-y-4">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h1 class="text-2xl font-bold text-gray-900">首页</h1>
            <p class="text-sm text-gray-600 mt-1">
              四入口：会话、辩论广场、搜索、个人中心。
            </p>
          </div>
          <button
            @click="refreshHome"
            :disabled="loading"
            class="px-4 py-2 rounded bg-blue-600 text-white text-sm disabled:opacity-50"
          >
            {{ loading ? '刷新中...' : '刷新首页' }}
          </button>
        </div>

        <div v-if="errorText" class="bg-red-50 text-red-700 border border-red-200 rounded p-3 text-sm">
          {{ errorText }}
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <button
            @click="goTo('/chat')"
            class="bg-white border rounded-lg p-4 text-left hover:border-blue-300 hover:shadow-sm transition"
          >
            <div class="text-xs uppercase text-gray-500">入口 1</div>
            <div class="text-lg font-semibold text-gray-900 mt-1">会话</div>
            <div class="text-sm text-gray-600 mt-2">
              群聊 {{ groupChannels.length }} · 单聊 {{ singleChannels.length }}
            </div>
          </button>

          <button
            @click="goTo('/debate')"
            class="bg-white border rounded-lg p-4 text-left hover:border-blue-300 hover:shadow-sm transition"
          >
            <div class="text-xs uppercase text-gray-500">入口 2</div>
            <div class="text-lg font-semibold text-gray-900 mt-1">辩论广场</div>
            <div class="text-sm text-gray-600 mt-2">
              场次 {{ debateStats.total }} · 进行中 {{ debateStats.live }} · 可加入 {{ debateStats.joinable }}
            </div>
          </button>

          <div class="bg-white border rounded-lg p-4">
            <div class="text-xs uppercase text-gray-500">入口 3</div>
            <div class="text-lg font-semibold text-gray-900 mt-1">搜索</div>
            <div class="text-sm text-gray-600 mt-2 mb-3">
              检索会话/辩题/场次并一键跳转。
            </div>
            <input
              v-model.trim="searchQuery"
              type="text"
              placeholder="输入关键词，例如：平衡 / session 12 / General"
              class="w-full border rounded px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div class="bg-white border rounded-lg p-4 text-left hover:border-blue-300 hover:shadow-sm transition">
            <div class="text-xs uppercase text-gray-500">入口 4</div>
            <div class="text-lg font-semibold text-gray-900 mt-1">个人中心</div>
            <div class="text-sm text-gray-600 mt-2">
              当前余额 {{ walletBalance }} · 通知 {{ notificationCount }}
            </div>
            <div class="mt-3 flex flex-wrap gap-2">
              <button
                type="button"
                class="px-2 py-1 text-xs rounded border bg-white hover:bg-gray-100"
                @click="goTo('/me')"
              >
                个人资料
              </button>
              <button
                type="button"
                class="px-2 py-1 text-xs rounded border bg-white hover:bg-gray-100"
                @click="goTo('/notifications')"
              >
                通知中心
              </button>
              <button
                type="button"
                class="px-2 py-1 text-xs rounded border bg-white hover:bg-gray-100"
                @click="goTo('/wallet')"
              >
                去充值
              </button>
            </div>
          </div>
        </div>

        <div class="bg-white border rounded-lg p-4 space-y-2">
          <div class="flex items-center justify-between">
            <div class="text-sm font-semibold text-gray-900">搜索结果</div>
            <div class="text-xs text-gray-500">items: {{ searchResults.length }}</div>
          </div>
          <div v-if="searchResults.length === 0" class="text-sm text-gray-600">
            暂无匹配结果，请尝试其它关键词。
          </div>
          <div v-else class="space-y-2">
            <button
              v-for="item in searchResults"
              :key="item.key"
              @click="openSearchItem(item)"
              class="w-full text-left border rounded p-3 bg-gray-50 hover:bg-gray-100"
            >
              <div class="text-sm font-semibold text-gray-900">{{ item.title }}</div>
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
  buildHomeSearchIndex,
  filterHomeSearchItems,
  summarizeDebateSessionStats,
} from '../home-utils';
import {
  buildNotificationCenterItems,
  countNotificationCenterItems,
} from '../notification-center-utils';

export default {
  components: {
    Sidebar,
  },
  data() {
    return {
      loading: false,
      errorText: '',
      walletBalance: 0,
      topics: [],
      sessions: [],
      searchQuery: '',
    };
  },
  computed: {
    groupChannels() {
      return this.$store.getters.getChannels || [];
    },
    singleChannels() {
      return this.$store.getters.getSingChannels || [];
    },
    debateStats() {
      return summarizeDebateSessionStats(this.sessions);
    },
    searchIndex() {
      const channels = [...this.groupChannels, ...this.singleChannels];
      return buildHomeSearchIndex({
        channels,
        topics: this.topics,
        sessions: this.sessions,
        topicTitleById: (topicId) => {
          const topic = this.topics.find((item) => item.id === topicId);
          return topic?.title || '';
        },
      });
    },
    searchResults() {
      return filterHomeSearchItems(this.searchIndex, this.searchQuery, { limit: 12 });
    },
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
    async refreshHome() {
      this.loading = true;
      this.errorText = '';
      try {
        const [topics, sessions, wallet] = await Promise.all([
          this.$store.dispatch('listDebateTopics', { activeOnly: true, limit: 100 }),
          this.$store.dispatch('listDebateSessions', { limit: 100 }),
          this.$store.dispatch('fetchWalletBalance'),
        ]);
        this.topics = topics || [];
        this.sessions = sessions || [];
        this.walletBalance = Number(wallet?.balance || 0);
      } catch (error) {
        this.errorText = error?.response?.data?.error || error?.message || '刷新首页失败';
      } finally {
        this.loading = false;
      }
    },
    async goTo(path) {
      if (this.$route.path === path) {
        return;
      }
      await this.$router.push(path);
    },
    async openSearchItem(item) {
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
  async mounted() {
    await this.refreshHome();
  },
};
</script>
