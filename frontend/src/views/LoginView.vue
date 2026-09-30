<script setup lang="ts">
import { reactive, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import type { FormInst } from "naive-ui";
import { GitNetworkOutline } from "@vicons/ionicons5";
import { useAuthStore } from "@/stores/auth";
import { message } from "@/utils/feedback";
import { extractError } from "@/api/client";

const router = useRouter();
const route = useRoute();
const auth = useAuthStore();
const formRef = ref<FormInst | null>(null);
const loading = ref(false);
const model = reactive({ username: "", password: "" });

async function submit() {
  if (!model.username || !model.password) {
    message.warning("请输入用户名和密码");
    return;
  }
  loading.value = true;
  try {
    await auth.login(model.username, model.password);
    message.success("登录成功");
    const redirect = (route.query.redirect as string) || "/";
    router.push(redirect);
  } catch (err) {
    message.error(extractError(err));
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="auth-page">
    <n-card class="auth-card" :bordered="false">
      <div class="auth-head">
        <div class="auth-logo">
          <n-icon :component="GitNetworkOutline" :size="28" />
        </div>
        <h1>EasyTier 控制台</h1>
        <p>简单、安全、去中心化的网络与节点管理平台</p>
      </div>
      <n-form ref="formRef" :model="model" size="large" @keydown.enter.prevent="submit">
        <n-form-item label="用户名">
          <n-input v-model:value="model.username" placeholder="请输入用户名" />
        </n-form-item>
        <n-form-item label="密码">
          <n-input
            v-model:value="model.password"
            type="password"
            show-password-on="click"
            placeholder="请输入密码"
          />
        </n-form-item>
        <n-button type="primary" block size="large" :loading="loading" @click="submit">
          登录
        </n-button>
      </n-form>
      <div class="auth-foot">EasyTier · 基于 easytier-core / easytier-cli 构建</div>
    </n-card>
  </div>
</template>

<style scoped>
.auth-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: radial-gradient(circle at 20% 20%, rgba(37, 99, 235, 0.16), transparent 45%),
    radial-gradient(circle at 80% 70%, rgba(13, 148, 136, 0.16), transparent 45%),
    var(--et-bg);
}
:global(.dark) .auth-page {
  background: radial-gradient(circle at 20% 20%, rgba(37, 99, 235, 0.22), transparent 45%),
    radial-gradient(circle at 80% 70%, rgba(13, 148, 136, 0.2), transparent 45%), #101014;
}
.auth-card {
  width: 100%;
  max-width: 420px;
  border-radius: 16px;
  box-shadow: 0 18px 50px rgba(0, 0, 0, 0.14);
}
.auth-head {
  text-align: center;
  margin-bottom: 22px;
}
.auth-logo {
  width: 58px;
  height: 58px;
  border-radius: 16px;
  background: linear-gradient(135deg, #2563eb, #0d9488);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 12px;
}
.auth-head h1 {
  font-size: 21px;
  margin: 0 0 6px;
}
.auth-head p {
  font-size: 13px;
  opacity: 0.65;
  margin: 0;
}
.auth-foot {
  text-align: center;
  font-size: 12px;
  opacity: 0.5;
  margin-top: 18px;
}
</style>
