<template>
  <div class="flex items-center justify-center min-h-screen bg-gray-100">
    <div class="w-full max-w-md p-8 space-y-6 bg-white rounded-xl shadow-2xl">
      <h1 class="text-2xl font-bold text-gray-800 text-center">绑定手机号</h1>
      <p class="text-sm text-gray-600 text-center">
        为了继续使用功能，请先完成手机号验证码绑定（仅支持中国大陆 +86）。
      </p>

      <form @submit.prevent="bindPhone" class="space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700">手机号</label>
          <input
            v-model="phone"
            type="text"
            placeholder="13800138000"
            required
            class="mt-1 block w-full px-3 py-2 bg-gray-50 border border-gray-300 rounded-md text-sm"
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700">验证码</label>
          <div class="mt-1 flex gap-2">
            <input
              v-model="smsCode"
              type="text"
              placeholder="6位验证码"
              required
              class="flex-1 px-3 py-2 bg-gray-50 border border-gray-300 rounded-md text-sm"
            />
            <button
              type="button"
              @click="sendCode"
              class="px-3 py-2 text-sm text-white bg-blue-600 rounded-md hover:bg-blue-700"
            >
              发送验证码
            </button>
          </div>
        </div>

        <p v-if="tips" class="text-xs text-gray-600">{{ tips }}</p>
        <p v-if="errorText" class="text-sm text-red-600">{{ errorText }}</p>

        <button
          type="submit"
          class="w-full py-2 px-4 rounded-md text-white bg-emerald-600 hover:bg-emerald-700"
        >
          绑定并继续
        </button>
      </form>
    </div>
  </div>
</template>

<script>
export default {
  data() {
    return {
      phone: '',
      smsCode: '',
      tips: '',
      errorText: '',
    };
  },
  methods: {
    async sendCode() {
      this.errorText = '';
      this.tips = '';
      try {
        const ret = await this.$store.dispatch('sendSmsCodeV2', {
          phone: this.phone,
          scene: 'bind_phone',
        });
        if (ret?.debugCode) {
          this.tips = `开发环境验证码：${ret.debugCode}`;
        } else {
          this.tips = '验证码已发送，请查收短信。';
        }
      } catch (error) {
        this.errorText = error?.response?.data?.error || '发送验证码失败';
      }
    },
    async bindPhone() {
      this.errorText = '';
      try {
        await this.$store.dispatch('bindPhoneV2', {
          phone: this.phone,
          smsCode: this.smsCode,
        });
        this.$router.push('/home');
      } catch (error) {
        this.errorText = error?.response?.data?.error || '绑定失败';
      }
    },
  },
};
</script>
