<script setup lang="ts">
import { computed, h, onMounted, reactive, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
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
const { t } = useI18n();
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
    message.warning(t("settings.newPasswordMin"));
    return;
  }
  if (pwModel.newPassword !== pwModel.confirm) {
    message.warning(t("settings.passwordMismatch"));
    return;
  }
  pwLoading.value = true;
  try {
    await authApi.changePassword({
      oldPassword: pwModel.oldPassword,
      newPassword: pwModel.newPassword,
    });
    message.success(t("settings.passwordChanged"));
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
    message.warning(t("settings.userValidation"));
    return;
  }
  userSubmitting.value = true;
  try {
    await userApi.create({ ...userModel });
    message.success(t("settings.userCreated"));
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
    message.warning(t("settings.passwordMin"));
    return;
  }
  resetSubmitting.value = true;
  try {
    await userApi.resetPassword(resetTarget.value.id, resetValue.value);
    message.success(t("settings.passwordReset"));
    showResetModal.value = false;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    resetSubmitting.value = false;
  }
}

function removeUser(user: User) {
  dialog.error({
    title: t("settings.deleteUserTitle"),
    content: t("settings.deleteUserConfirm", { username: user.username }),
    positiveText: t("common.delete"),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      try {
        await userApi.remove(user.id);
        message.success(t("settings.userDeleted"));
        await loadUsers();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

const roleOptions = computed(() => [
  { label: t("role.admin"), value: "admin" },
  { label: t("role.operator"), value: "operator" },
  { label: t("role.viewer"), value: "viewer" },
]);

const userColumns = computed<DataTableColumns<User>>(() => [
  { title: t("settings.username"), key: "username" },
  { title: t("settings.displayName"), key: "displayName", render: (u) => u.displayName ?? "-" },
  {
    title: t("common.role"),
    key: "role",
    width: 100,
    render: (u) =>
      ({ admin: t("role.admin"), operator: t("role.operator"), viewer: t("role.viewer") })[
        u.role
      ] ?? u.role,
  },
  {
    title: t("common.status"),
    key: "disabled",
    width: 90,
    render: (u) => (u.disabled ? t("common.disabled") : t("common.enabled")),
  },
  {
    title: t("settings.colLastLogin"),
    key: "lastLoginAt",
    width: 170,
    render: (u) => formatTime(u.lastLoginAt),
  },
  {
    title: t("common.actions"),
    key: "actions",
    width: 220,
    render: (u) =>
      h("div", { class: "row-actions" }, [
        h(
          NButton,
          { size: "tiny", quaternary: true, onClick: () => toggleDisabled(u) },
          { default: () => (u.disabled ? t("common.enabled") : t("common.disabled")) },
        ),
        h(
          NButton,
          { size: "tiny", quaternary: true, onClick: () => resetPassword(u) },
          { default: () => t("settings.resetPassword") },
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
]);

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
    message.success(t("settings.saved"));
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
        <h1 class="page-title">{{ t("nav.settings") }}</h1>
        <div class="page-subtitle">{{ t("settings.subtitle") }}</div>
      </div>
    </div>

    <n-card size="small">
      <n-tabs v-model:value="activeTab" type="line" animated>
        <!-- 账号安全 -->
        <n-tab-pane name="security" :tab="t('settings.tabSecurity')">
          <n-form
            ref="pwFormRef"
            :model="pwModel"
            label-placement="left"
            label-width="100"
            style="max-width: 480px"
          >
            <n-form-item :label="t('settings.oldPassword')">
              <n-input
                v-model:value="pwModel.oldPassword"
                type="password"
                show-password-on="click"
              />
            </n-form-item>
            <n-form-item :label="t('settings.newPassword')">
              <n-input
                v-model:value="pwModel.newPassword"
                type="password"
                show-password-on="click"
                :placeholder="t('settings.passwordPlaceholder')"
              />
            </n-form-item>
            <n-form-item :label="t('settings.confirmPassword')">
              <n-input
                v-model:value="pwModel.confirm"
                type="password"
                show-password-on="click"
              />
            </n-form-item>
            <n-form-item>
              <n-button type="primary" :loading="pwLoading" @click="changePassword">
                <template #icon><n-icon :component="KeyOutline" /></template>
                {{ t("nav.changePassword") }}
              </n-button>
            </n-form-item>
          </n-form>
        </n-tab-pane>

        <!-- 用户管理 -->
        <n-tab-pane name="users" :tab="t('settings.tabUsers')" :disabled="!isAdmin">
          <div class="toolbar">
            <n-button size="small" type="primary" @click="showUserModal = true">
              <template #icon><n-icon :component="AddOutline" /></template>
              {{ t("settings.createUser") }}
            </n-button>
            <n-button size="small" @click="loadUsers">
              <template #icon><n-icon :component="RefreshOutline" /></template>
              {{ t("common.refresh") }}
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
        <n-tab-pane name="system" :tab="t('settings.tabSystem')" :disabled="!isAdmin">
          <n-grid :cols="24" :x-gap="16">
            <n-gi :span="14">
              <n-form label-placement="top">
                <n-form-item :label="t('settings.corePath')">
                  <n-input
                    v-model:value="settings['easytier.corePath']"
                    :placeholder="t('settings.corePathPlaceholder')"
                  />
                </n-form-item>
                <n-form-item :label="t('settings.cliPath')">
                  <n-input
                    v-model:value="settings['easytier.cliPath']"
                    :placeholder="t('settings.cliPathPlaceholder')"
                  />
                </n-form-item>
                <n-form-item :label="t('settings.defaultExternalNode')">
                  <n-input
                    v-model:value="settings['easytier.defaultExternalNode']"
                    placeholder="tcp://public.easytier.cn:11010"
                  />
                </n-form-item>
                <n-grid :cols="2" :x-gap="14">
                  <n-form-item-gi :label="t('settings.rpcPortStart')">
                    <n-input-number
                      v-model:value="settings['easytier.rpcPortStart']"
                      :min="1"
                      :max="65535"
                      style="width: 100%"
                    />
                  </n-form-item-gi>
                  <n-form-item-gi :label="t('settings.listenPortStart')">
                    <n-input-number
                      v-model:value="settings['easytier.listenPortStart']"
                      :min="1"
                      :max="65535"
                      style="width: 100%"
                    />
                  </n-form-item-gi>
                </n-grid>
                <n-divider title-placement="left" style="margin: 4px 0 12px">
                  {{ t("settings.logs") }}
                </n-divider>
                <n-form-item :label="t('settings.retentionDays')">
                  <n-input-number
                    v-model:value="settings['log.retentionDays']"
                    :min="0"
                    :max="3650"
                    style="width: 220px"
                  />
                  <span class="hint">
                    {{ t("settings.retentionHint") }}
                  </span>
                </n-form-item>
                <n-button type="primary" :loading="settingsLoading" @click="saveSettings">
                  <template #icon><n-icon :component="SaveOutline" /></template>
                  {{ t("settings.save") }}
                </n-button>
              </n-form>
            </n-gi>
            <n-gi :span="10">
              <n-card :title="t('settings.binariesTitle')" size="small">
                <div class="bin-row">
                  <span>easytier-core</span>
                  <n-tag
                    :type="binaries?.core.available ? 'success' : 'error'"
                    size="small"
                    :bordered="false"
                  >
                    {{ binaries?.core.available ? binaries?.core.version ?? t("settings.available") : t("settings.unavailable") }}
                  </n-tag>
                </div>
                <div class="bin-row">
                  <span>easytier-cli</span>
                  <n-tag
                    :type="binaries?.cli.available ? 'success' : 'error'"
                    size="small"
                    :bordered="false"
                  >
                    {{ binaries?.cli.available ? binaries?.cli.version ?? t("settings.available") : t("settings.unavailable") }}
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
                  {{ t("settings.recheck") }}
                </n-button>
              </n-card>
            </n-gi>
          </n-grid>
        </n-tab-pane>

        <!-- 关于 -->
        <n-tab-pane name="about" :tab="t('settings.tabAbout')">
          <n-descriptions :column="1" size="small" bordered style="max-width: 720px">
            <n-descriptions-item :label="t('settings.consoleVersion')">v{{ info?.version ?? "1.0.0" }}</n-descriptions-item>
            <n-descriptions-item :label="t('settings.backendRuntime')">{{ info?.node ?? "-" }}</n-descriptions-item>
            <n-descriptions-item :label="t('settings.platform')">{{ info?.platform ?? "-" }}</n-descriptions-item>
            <n-descriptions-item :label="t('settings.uptime')">
              {{ info ? secondsToUptime(info.uptime) : "-" }}
            </n-descriptions-item>
            <n-descriptions-item :label="t('settings.dataDir')">
              <CopyText :text="info?.dataDir ?? ''" />
            </n-descriptions-item>
            <n-descriptions-item :label="t('settings.techStack')">
              <div class="tech-stack">
                <div><strong>{{ t("settings.techFrontend") }}</strong>Vue 3 · Vite · TypeScript · Naive UI · Pinia · Vue Router · Vue Flow · ECharts</div>
                <div><strong>{{ t("settings.techBackend") }}</strong>Rust · Axum · tokio · rusqlite (SQLite) · AES-256-GCM · Argon2id · JSON Web Token</div>
                <div><strong>{{ t("settings.techRuntime") }}</strong>{{ t("settings.techRuntimeValue") }}</div>
              </div>
            </n-descriptions-item>
            <n-descriptions-item :label="t('settings.projectDescription')">
              {{ t("settings.projectDescriptionText") }}
            </n-descriptions-item>
            <n-descriptions-item :label="t('settings.author')">Inborn Lee（李颖博）</n-descriptions-item>
          </n-descriptions>
        </n-tab-pane>
      </n-tabs>
    </n-card>

    <n-modal
      v-model:show="showUserModal"
      preset="card"
      :title="t('settings.createUser')"
      style="width: 520px; max-width: 94vw"
    >
      <n-form :model="userModel" label-placement="top">
        <n-grid :cols="2" :x-gap="14">
          <n-form-item-gi :label="t('settings.username')">
            <n-input v-model:value="userModel.username" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('settings.displayName')">
            <n-input v-model:value="userModel.displayName" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('common.role')">
            <n-select v-model:value="userModel.role" :options="roleOptions" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('settings.email')">
            <n-input v-model:value="userModel.email" />
          </n-form-item-gi>
        </n-grid>
        <n-form-item :label="t('settings.initialPassword')">
          <n-input
            v-model:value="userModel.password"
            type="password"
            show-password-on="click"
            :placeholder="t('settings.passwordPlaceholder')"
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <div class="modal-footer">
          <n-button @click="showUserModal = false">{{ t("common.cancel") }}</n-button>
          <n-button type="primary" :loading="userSubmitting" @click="createUser">{{ t("common.create") }}</n-button>
        </div>
      </template>
    </n-modal>

    <n-modal
      v-model:show="showResetModal"
      preset="card"
      :title="t('settings.resetPasswordTitle', { username: resetTarget?.username ?? '' })"
      style="width: 460px; max-width: 94vw"
    >
      <n-form label-placement="top">
        <n-form-item :label="t('settings.newPassword')">
          <n-input
            v-model:value="resetValue"
            type="password"
            show-password-on="click"
            :placeholder="t('settings.passwordPlaceholder')"
          />
        </n-form-item>
      </n-form>
      <template #footer>
        <div class="modal-footer">
          <n-button @click="showResetModal = false">{{ t("common.cancel") }}</n-button>
          <n-button type="primary" :loading="resetSubmitting" @click="submitReset">
            {{ t("settings.reset") }}
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
