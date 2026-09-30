<script setup lang="ts">
import { computed, h, onMounted, reactive, ref, watch } from "vue";
import { useRoute } from "vue-router";
import type { DataTableColumns, FormInst } from "naive-ui";
import { NButton, NIcon } from "naive-ui";
import {
  RefreshOutline,
  SaveOutline,
  TrashOutline,
  KeyOutline,
  AddOutline,
} from "@vicons/ionicons5";
import CopyText from "@/components/CopyText.vue";
import { authApi, systemApi, userApi } from "@/api";
import { extractError } from "@/api/client";
import { message, dialog } from "@/utils/feedback";
import { formatTime, secondsToUptime } from "@/utils/format";
import { useAuthStore } from "@/stores/auth";
import type { SystemBinaries, SystemInfo, User } from "@/types";

const route = useRoute();
const auth = useAuthStore();
const isAdmin = computed(() => auth.user?.role === "admin");
const activeTab = ref((route.query.tab as string) || "security");

watch(
  () => route.query.tab,
  (t) => {
    if (t) activeTab.value = t as string;
  },
);

// ---------------- 账号安全 ----------------
const pwFormRef = ref<FormInst | null>(null);
const pwModel = reactive({ oldPassword: "", newPassword: "", confirm: "" });
const pwLoading = ref(false);

async function changePassword() {
  if (pwModel.newPassword.length < 12) {
    message.warning("新密码长度至少 12 位");
    return;
  }
  if (pwModel.newPassword !== pwModel.confirm) {
    message.warning("两次输入的新密码不一致");
    return;
  }
  pwLoading.value = true;
  try {
    await authApi.changePassword({
      oldPassword: pwModel.oldPassword,
      newPassword: pwModel.newPassword,
    });
    message.success("密码修改成功");
    pwModel.oldPassword = pwModel.newPassword = pwModel.confirm = "";
  } catch (err) {
    message.error(extractError(err));
  } finally {
    pwLoading.value = false;
  }
}

// ---------------- 用户管理 ----------------
const users = ref<User[]>([]);
const usersLoading = ref(false);
const showUserModal = ref(false);
const userSubmitting = ref(false);
const showResetModal = ref(false);
const resetTarget = ref<User | null>(null);
const resetValue = ref("");
const resetSubmitting = ref(false);
const userModel = reactive({
  username: "",
  password: "",
  displayName: "",
  email: "",
  role: "operator",
});

async function loadUsers() {
  if (!isAdmin.value) return;
  usersLoading.value = true;
  try {
    users.value = await userApi.list();
  } catch (err) {
    message.error(extractError(err));
  } finally {
    usersLoading.value = false;
  }
}

async function createUser() {
  if (!userModel.username || userModel.password.length < 12) {
    message.warning("请填写用户名，密码至少 12 位");
    return;
  }
  userSubmitting.value = true;
  try {
    await userApi.create({ ...userModel });
    message.success("用户已创建");
    showUserModal.value = false;
    Object.assign(userModel, {
      username: "",
      password: "",
      displayName: "",
      email: "",
      role: "operator",
    });
    await loadUsers();
  } catch (err) {
    message.error(extractError(err));
  } finally {
    userSubmitting.value = false;
  }
}

async function toggleDisabled(user: User) {
  try {
    await userApi.update(user.id, { disabled: !user.disabled });
    await loadUsers();
  } catch (err) {
    message.error(extractError(err));
  }
}

function resetPassword(user: User) {
  resetTarget.value = user;
  resetValue.value = "";
  showResetModal.value = true;
}

async function submitReset() {
  if (!resetTarget.value) return;
  if (resetValue.value.length < 12) {
    message.warning("密码至少 12 位");
    return;
  }
  resetSubmitting.value = true;
  try {
    await userApi.resetPassword(resetTarget.value.id, resetValue.value);
    message.success("密码已重置");
    showResetModal.value = false;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    resetSubmitting.value = false;
  }
}

function removeUser(user: User) {
  dialog.error({
    title: "删除用户",
    content: `确定删除用户「${user.username}」吗？`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await userApi.remove(user.id);
        message.success("已删除");
        await loadUsers();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

const userColumns: DataTableColumns<User> = [
  { title: "用户名", key: "username" },
  { title: "显示名称", key: "displayName", render: (u) => u.displayName ?? "-" },
  {
    title: "角色",
    key: "role",
    width: 100,
    render: (u) => ({ admin: "管理员", operator: "运维", viewer: "访客" })[u.role] ?? u.role,
  },
  {
    title: "状态",
    key: "disabled",
    width: 90,
    render: (u) => (u.disabled ? "已禁用" : "正常"),
  },
  { title: "最近登录", key: "lastLoginAt", width: 170, render: (u) => formatTime(u.lastLoginAt) },
  {
    title: "操作",
    key: "actions",
    width: 220,
    render: (u) =>
      h("div", { class: "row-actions" }, [
        h(
          NButton,
          { size: "tiny", quaternary: true, onClick: () => toggleDisabled(u) },
          { default: () => (u.disabled ? "启用" : "禁用") },
        ),
        h(
          NButton,
          { size: "tiny", quaternary: true, onClick: () => resetPassword(u) },
          { default: () => "重置密码" },
        ),
        h(
          NButton,
          {
            size: "tiny",
            quaternary: true,
            type: "error",
            disabled: u.id === auth.user?.id,
            onClick: () => removeUser(u),
          },
          { icon: () => h(NIcon, { component: TrashOutline }) },
        ),
      ]),
  },
];

// ---------------- 系统设置 ----------------
const settings = reactive({
  "easytier.corePath": "",
  "easytier.cliPath": "",
  "easytier.defaultExternalNode": "",
  "easytier.rpcPortStart": 15888,
  "easytier.listenPortStart": 11010,
  "log.retentionDays": 30,
});
const settingsLoading = ref(false);
const binaries = ref<SystemBinaries | null>(null);
const checkingBinaries = ref(false);

async function loadSettings() {
  if (!isAdmin.value) return;
  settingsLoading.value = true;
  try {
    const res = await systemApi.settings();
    Object.assign(settings, res.effective);
  } catch (err) {
    message.error(extractError(err));
  } finally {
    settingsLoading.value = false;
  }
}

async function saveSettings() {
  try {
    await systemApi.updateSettings({ ...settings });
    message.success("设置已保存");
  } catch (err) {
    message.error(extractError(err));
  }
}

async function checkBinaries() {
  checkingBinaries.value = true;
  try {
    binaries.value = await systemApi.binaries();
  } catch (err) {
    message.error(extractError(err));
  } finally {
    checkingBinaries.value = false;
  }
}

// ---------------- 关于 ----------------
const info = ref<SystemInfo | null>(null);
async function loadInfo() {
  try {
    info.value = await systemApi.info();
  } catch {
    /* ignore */
  }
}

onMounted(async () => {
  await Promise.all([loadInfo(), loadUsers(), loadSettings(), checkBinaries()]);
});
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h1 class="page-title">系统设置</h1>
        <div class="page-subtitle">账号安全、用户权限与 EasyTier 运行参数</div>
      </div>
    </div>

    <n-card size="small">
      <n-tabs v-model:value="activeTab" type="line" animated>
        <!-- 账号安全 -->
        <n-tab-pane name="security" tab="账号安全">
          <n-form
            ref="pwFormRef"
            :model="pwModel"
            label-placement="left"
            label-width="100"
            style="max-width: 480px"
          >
            <n-form-item label="原密码">
              <n-input
                v-model:value="pwModel.oldPassword"
                type="password"
                show-password-on="click"
              />
            </n-form-item>
            <n-form-item label="新密码">
              <n-input
                v-model:value="pwModel.newPassword"
                type="password"
                show-password-on="click"
                placeholder="至少 12 位，含大小写、数字与特殊字符"
              />
            </n-form-item>
            <n-form-item label="确认新密码">
              <n-input
                v-model:value="pwModel.confirm"
                type="password"
                show-password-on="click"
              />
            </n-form-item>
            <n-form-item>
              <n-button type="primary" :loading="pwLoading" @click="changePassword">
                <template #icon><n-icon :component="KeyOutline" /></template>
                修改密码
              </n-button>
            </n-form-item>
          </n-form>
        </n-tab-pane>

        <!-- 用户管理 -->
        <n-tab-pane name="users" tab="用户管理" :disabled="!isAdmin">
          <div class="toolbar">
            <n-button size="small" type="primary" @click="showUserModal = true">
              <template #icon><n-icon :component="AddOutline" /></template>
              新建用户
            </n-button>
            <n-button size="small" @click="loadUsers">
              <template #icon><n-icon :component="RefreshOutline" /></template>
              刷新
            </n-button>
          </div>
          <n-data-table
            :columns="userColumns"
            :data="users"
            :loading="usersLoading"
            size="small"
            :row-key="(u: User) => u.id"
          />
        </n-tab-pane>

        <!-- 系统设置 -->
        <n-tab-pane name="system" tab="EasyTier 设置" :disabled="!isAdmin">
          <n-grid :cols="24" :x-gap="16">
            <n-gi :span="14">
              <n-form label-placement="top">
                <n-form-item label="easytier-core 路径">
                  <n-input
                    v-model:value="settings['easytier.corePath']"
                    placeholder="easytier-core 或绝对路径"
                  />
                </n-form-item>
                <n-form-item label="easytier-cli 路径">
                  <n-input
                    v-model:value="settings['easytier.cliPath']"
                    placeholder="easytier-cli 或绝对路径"
                  />
                </n-form-item>
                <n-form-item label="默认公共共享节点">
                  <n-input
                    v-model:value="settings['easytier.defaultExternalNode']"
                    placeholder="tcp://public.easytier.cn:11010"
                  />
                </n-form-item>
                <n-grid :cols="2" :x-gap="14">
                  <n-form-item-gi label="RPC 门户起始端口">
                    <n-input-number
                      v-model:value="settings['easytier.rpcPortStart']"
                      :min="1"
                      :max="65535"
                      style="width: 100%"
                    />
                  </n-form-item-gi>
                  <n-form-item-gi label="监听起始端口">
                    <n-input-number
                      v-model:value="settings['easytier.listenPortStart']"
                      :min="1"
                      :max="65535"
                      style="width: 100%"
                    />
                  </n-form-item-gi>
                </n-grid>
                <n-divider title-placement="left" style="margin: 4px 0 12px">
                  日志
                </n-divider>
                <n-form-item label="日志储存期限（天）">
                  <n-input-number
                    v-model:value="settings['log.retentionDays']"
                    :min="0"
                    :max="3650"
                    style="width: 220px"
                  />
                  <span class="hint">
                    运行日志与操作审计的保留天数，0 表示永久保留；超期日志每 6 小时自动清理。
                  </span>
                </n-form-item>
                <n-button type="primary" :loading="settingsLoading" @click="saveSettings">
                  <template #icon><n-icon :component="SaveOutline" /></template>
                  保存设置
                </n-button>
              </n-form>
            </n-gi>
            <n-gi :span="10">
              <n-card title="二进制检测" size="small">
                <div class="bin-row">
                  <span>easytier-core</span>
                  <n-tag
                    :type="binaries?.core.available ? 'success' : 'error'"
                    size="small"
                    :bordered="false"
                  >
                    {{ binaries?.core.available ? binaries?.core.version ?? "可用" : "不可用" }}
                  </n-tag>
                </div>
                <div class="bin-row">
                  <span>easytier-cli</span>
                  <n-tag
                    :type="binaries?.cli.available ? 'success' : 'error'"
                    size="small"
                    :bordered="false"
                  >
                    {{ binaries?.cli.available ? binaries?.cli.version ?? "可用" : "不可用" }}
                  </n-tag>
                </div>
                <n-alert
                  v-if="binaries && (!binaries.core.available || !binaries.cli.available)"
                  type="warning"
                  :show-icon="true"
                  style="margin-top: 12px; font-size: 12px"
                >
                  {{ binaries.core.error || binaries.cli.error }}
                </n-alert>
                <n-button
                  size="small"
                  style="margin-top: 12px"
                  :loading="checkingBinaries"
                  @click="checkBinaries"
                >
                  <template #icon><n-icon :component="RefreshOutline" /></template>
                  重新检测
                </n-button>
              </n-card>
            </n-gi>
          </n-grid>
        </n-tab-pane>

        <!-- 关于 -->
        <n-tab-pane name="about" tab="关于">
          <n-descriptions :column="1" size="small" bordered style="max-width: 720px">
            <n-descriptions-item label="控制台版本">v{{ info?.version ?? "1.0.0" }}</n-descriptions-item>
            <n-descriptions-item label="后端运行时">{{ info?.node ?? "-" }}</n-descriptions-item>
            <n-descriptions-item label="运行平台">{{ info?.platform ?? "-" }}</n-descriptions-item>
            <n-descriptions-item label="运行时长">
              {{ info ? secondsToUptime(info.uptime) : "-" }}
            </n-descriptions-item>
            <n-descriptions-item label="数据目录">
              <CopyText :text="info?.dataDir ?? ''" />
            </n-descriptions-item>
            <n-descriptions-item label="技术栈">
              <div class="tech-stack">
                <div><strong>前端：</strong>Vue 3 · Vite · TypeScript · Naive UI · Pinia · Vue Router · Vue Flow · ECharts</div>
                <div><strong>后端：</strong>Rust · Axum · tokio · rusqlite (SQLite) · AES-256-GCM · Argon2id · JSON Web Token</div>
                <div><strong>运行环境：</strong>内嵌 easytier-core / easytier-cli（静态链接）</div>
              </div>
            </n-descriptions-item>
            <n-descriptions-item label="项目说明">
              基于 easytier-core / easytier-cli 构建的 EasyTier 控制台，提供网络、节点、凭据与日志管理能力。
            </n-descriptions-item>
            <n-descriptions-item label="作者">Inborn Lee（李颖博）</n-descriptions-item>
          </n-descriptions>
        </n-tab-pane>
      </n-tabs>
    </n-card>

    <n-modal
      v-model:show="showUserModal"
      preset="card"
      title="新建用户"
      style="width: 520px; max-width: 94vw"
    >
      <n-form :model="userModel" label-placement="top">
        <n-grid :cols="2" :x-gap="14">
          <n-form-item-gi label="用户名">
            <n-input v-model:value="userModel.username" />
          </n-form-item-gi>
          <n-form-item-gi label="显示名称">
            <n-input v-model:value="userModel.displayName" />
          </n-form-item-gi>
          <n-form-item-gi label="角色">
            <n-select
              v-model:value="userModel.role"
              :options="[
                { label: '管理员', value: 'admin' },
                { label: '运维', value: 'operator' },
                { label: '访客', value: 'viewer' },
              ]"
            />
          </n-form-item-gi>
          <n-form-item-gi label="邮箱">
            <n-input v-model:value="userModel.email" />
          </n-form-item-gi>
        </n-grid>
        <n-form-item label="初始密码">
          <n-input
            v-model:value="userModel.password"
            type="password"
            show-password-on="click"
            placeholder="至少 12 位，含大小写、数字与特殊字符"
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <div class="modal-footer">
          <n-button @click="showUserModal = false">取消</n-button>
          <n-button type="primary" :loading="userSubmitting" @click="createUser">创建</n-button>
        </div>
      </template>
    </n-modal>

    <n-modal
      v-model:show="showResetModal"
      preset="card"
      :title="`重置「${resetTarget?.username ?? ''}」的密码`"
      style="width: 460px; max-width: 94vw"
    >
      <n-form label-placement="top">
        <n-form-item label="新密码">
          <n-input
            v-model:value="resetValue"
            type="password"
            show-password-on="click"
            placeholder="至少 12 位，含大小写、数字与特殊字符"
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <div class="modal-footer">
          <n-button @click="showResetModal = false">取消</n-button>
          <n-button type="primary" :loading="resetSubmitting" @click="submitReset">
            重置
          </n-button>
        </div>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  gap: 10px;
  margin-bottom: 12px;
}
.row-actions {
  display: flex;
  gap: 4px;
}
.bin-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px dashed rgba(128, 128, 128, 0.2);
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.hint {
  margin-left: 10px;
  font-size: 12px;
  opacity: 0.6;
}
.tech-stack {
  display: flex;
  flex-direction: column;
  gap: 4px;
  line-height: 1.6;
}
</style>
