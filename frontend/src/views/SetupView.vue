<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { useRouter } from "vue-router";
import type { FormInst, FormRules } from "naive-ui";
import { ShieldCheckmarkOutline } from "@vicons/ionicons5";
import { useAuthStore } from "@/stores/auth";
import { message } from "@/utils/feedback";
import { extractError } from "@/api/client";

const router = useRouter();
const auth = useAuthStore();
const formRef = ref<FormInst | null>(null);
const loading = ref(false);

const model = reactive({
  username: "",
  displayName: "",
  email: "",
  password: "",
  confirm: "",
});

const rules: FormRules = {
  username: [
    { required: true, message: "请输入管理员用户名", trigger: ["blur", "input"] },
    {
      pattern: /^[A-Za-z0-9_.-]{3,32}$/,
      message: "3-32 位字母、数字、下划线、点或短横线",
      trigger: ["blur", "input"],
    },
  ],
  password: [
    { required: true, message: "请输入密码", trigger: ["blur", "input"] },
    { min: 12, message: "密码长度至少 12 位", trigger: ["blur", "input"] },
    {
      validator: (_rule, value: string) => {
        if (!value) return false;
        return (
          /[a-z]/.test(value) &&
          /[A-Z]/.test(value) &&
          /[0-9]/.test(value) &&
          /[^A-Za-z0-9]/.test(value)
        );
      },
      message: "需同时包含大写、小写、数字和特殊字符",
      trigger: ["blur", "input"],
    },
  ],
  confirm: [
    {
      validator: (_rule, value: string) => value === model.password,
      message: "两次输入的密码不一致",
      trigger: ["blur", "input"],
    },
  ],
  email: [{ type: "email", message: "邮箱格式不正确", trigger: ["blur"] }],
};

const strength = computed(() => {
  const v = model.password;
  let score = 0;
  if (v.length >= 12) score += 1;
  if (v.length >= 16) score += 1;
  if (/[a-z]/.test(v) && /[A-Z]/.test(v)) score += 1;
  if (/[0-9]/.test(v)) score += 1;
  if (/[^A-Za-z0-9]/.test(v)) score += 1;
  return Math.min(score, 5);
});

const strengthLabel = computed(
  () => ["非常弱", "弱", "一般", "较强", "强", "很强"][strength.value],
);
const strengthColor = computed(
  () => ["#e5484d", "#e5484d", "#f5a524", "#2563eb", "#0d9488", "#16a34a"][strength.value],
);

async function submit() {
  try {
    await formRef.value?.validate();
  } catch {
    return;
  }
  loading.value = true;
  try {
    await auth.setup({
      username: model.username,
      password: model.password,
      displayName: model.displayName || undefined,
      email: model.email || undefined,
    });
    message.success("管理员账号创建成功，欢迎使用 EasyTier 控制台");
    router.push({ name: "dashboard" });
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
          <n-icon :component="ShieldCheckmarkOutline" :size="26" />
        </div>
        <h1>初始化控制台</h1>
        <p>首次使用，请创建管理员账号。密码采用 Argon2id 强加密存储。</p>
      </div>
      <n-form
        ref="formRef"
        :model="model"
        :rules="rules"
        label-placement="top"
        size="large"
        @keydown.enter.prevent="submit"
      >
        <n-form-item label="管理员用户名" path="username">
          <n-input v-model:value="model.username" placeholder="例如 admin" />
        </n-form-item>
        <n-grid :cols="2" :x-gap="12">
          <n-form-item-gi label="显示名称（可选）">
            <n-input v-model:value="model.displayName" placeholder="网络管理员" />
          </n-form-item-gi>
          <n-form-item-gi label="邮箱（可选）" path="email">
            <n-input v-model:value="model.email" placeholder="admin@example.com" />
          </n-form-item-gi>
        </n-grid>
        <n-form-item label="密码" path="password">
          <n-input
            v-model:value="model.password"
            type="password"
            show-password-on="click"
            placeholder="至少 12 位，含大小写、数字与特殊字符"
          />
        </n-form-item>
        <div class="strength">
          <div class="strength-bar">
            <span
              v-for="i in 5"
              :key="i"
              :style="{
                background: i <= strength ? strengthColor : 'rgba(128,128,128,0.2)',
              }"
            />
          </div>
          <span class="strength-label" :style="{ color: strengthColor }">
            {{ strengthLabel }}
          </span>
        </div>
        <n-form-item label="确认密码" path="confirm">
          <n-input
            v-model:value="model.confirm"
            type="password"
            show-password-on="click"
            placeholder="再次输入密码"
          />
        </n-form-item>
        <n-button type="primary" block size="large" :loading="loading" @click="submit">
          创建管理员并进入控制台
        </n-button>
      </n-form>
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
  max-width: 480px;
  border-radius: 16px;
  box-shadow: 0 18px 50px rgba(0, 0, 0, 0.14);
}
.auth-head {
  text-align: center;
  margin-bottom: 20px;
}
.auth-logo {
  width: 54px;
  height: 54px;
  border-radius: 14px;
  background: linear-gradient(135deg, #2563eb, #0d9488);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 12px;
}
.auth-head h1 {
  font-size: 20px;
  margin: 0 0 6px;
}
.auth-head p {
  font-size: 13px;
  opacity: 0.65;
  margin: 0;
}
.strength {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: -6px 0 12px;
}
.strength-bar {
  display: flex;
  gap: 4px;
  flex: 1;
}
.strength-bar span {
  height: 5px;
  border-radius: 3px;
  flex: 1;
  transition: background 0.2s;
}
.strength-label {
  font-size: 12px;
  font-weight: 600;
  width: 44px;
  text-align: right;
}
</style>
