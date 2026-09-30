<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { NIcon } from "naive-ui";
import { TrashOutline } from "@vicons/ionicons5";
import { networkApi, userApi } from "@/api";
import { extractError } from "@/api/client";
import { message, dialog } from "@/utils/feedback";
import { useAuthStore } from "@/stores/auth";
import type { Network, NetworkShare, NetworkShares, SelectableUser } from "@/types";

const props = defineProps<{
  show: boolean;
  network?: Network | null;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "transferred"): void;
}>();

const visible = computed({
  get: () => props.show,
  set: (v) => emit("update:show", v),
});

const auth = useAuthStore();
const { t } = useI18n();
const loading = ref(false);
const saving = ref(false);
const shares = ref<NetworkShares | null>(null);
const users = ref<SelectableUser[]>([]);
const selectedUserId = ref<string | null>(null);
const selectedPermission = ref<"view" | "manage">("view");
const transferUserId = ref<string | null>(null);
const transferring = ref(false);

const permissionOptions = computed(() => [
  { label: t("networkForm.share.permissionReadonly"), value: "view" },
  { label: t("networkForm.share.permissionManage"), value: "manage" },
]);

const sharedIds = computed(
  () => new Set((shares.value?.items ?? []).map((i) => i.userId)),
);

const candidateUsers = computed(() =>
  users.value.filter(
    (u) =>
      !u.disabled &&
      u.id !== auth.user?.id &&
      u.id !== props.network?.ownerId &&
      !sharedIds.value.has(u.id),
  ),
);

const candidateOptions = computed(() =>
  candidateUsers.value.map((u) => ({
    label: u.displayName ? `${u.username}（${u.displayName}）` : u.username,
    value: u.id,
  })),
);

const canTransfer = computed(() => {
  return (
    auth.user?.role === "admin" ||
    (!!props.network?.ownerId && props.network.ownerId === auth.user?.id)
  );
});

const transferOptions = computed(() =>
  users.value
    .filter((u) => !u.disabled && u.id !== props.network?.ownerId && u.id !== auth.user?.id)
    .map((u) => ({
      label: u.displayName ? `${u.username}（${u.displayName}）` : u.username,
      value: u.id,
    })),
);

const ownerLabel = computed(() => {
  if (!props.network) return "-";
  if (props.network.ownerId && props.network.ownerId === auth.user?.id)
    return t("networkForm.share.self");
  return shares.value?.owner?.username ?? props.network.ownerId ?? t("common.unknown");
});

function roleLabel(role: string) {
  const labels: Record<string, string> = {
    admin: t("role.admin"),
    operator: t("role.operator"),
    viewer: t("role.viewer"),
  };
  return labels[role] ?? role;
}

async function load() {
  if (!props.network) return;
  loading.value = true;
  try {
    const [s, u] = await Promise.all([
      networkApi.shares(props.network.id),
      userApi.selectable(),
    ]);
    shares.value = s;
    users.value = u;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.show,
  (v) => {
    if (!v) return;
    selectedUserId.value = null;
    selectedPermission.value = "view";
    void load();
  },
);

async function addShare() {
  if (!props.network || !selectedUserId.value) {
    message.warning(t("networkForm.share.selectUserRequired"));
    return;
  }
  saving.value = true;
  try {
    await networkApi.share(props.network.id, {
      userId: selectedUserId.value,
      permission: selectedPermission.value,
    });
    message.success(t("networkForm.share.shared"));
    selectedUserId.value = null;
    await load();
  } catch (err) {
    message.error(extractError(err));
  } finally {
    saving.value = false;
  }
}

async function changePermission(item: NetworkShare, perm: "view" | "manage") {
  if (!props.network) return;
  try {
    await networkApi.share(props.network.id, { userId: item.userId, permission: perm });
    message.success(t("networkForm.share.permissionUpdated"));
    await load();
  } catch (err) {
    message.error(extractError(err));
  }
}

function removeShare(item: NetworkShare) {
  if (!props.network) return;
  dialog.warning({
    title: t("networkForm.share.unshareTitle"),
    content: t("networkForm.share.unshareContent", { name: item.username }),
    positiveText: t("networkForm.share.unshareTitle"),
    negativeText: t("common.back"),
    onPositiveClick: async () => {
      try {
        await networkApi.unshare(props.network!.id, item.userId);
        message.success(t("networkForm.share.unshared"));
        await load();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

function transfer() {
  if (!props.network || !transferUserId.value) {
    message.warning(t("networkForm.share.transferRequired"));
    return;
  }
  const target = users.value.find((u) => u.id === transferUserId.value);
  dialog.warning({
    title: t("networkForm.share.transferTitle"),
    content: t("networkForm.share.transferContent", {
      network: props.network.name,
      user: target?.username ?? "",
    }),
    positiveText: t("networkForm.share.transferConfirm"),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      transferring.value = true;
      try {
        await networkApi.transfer(props.network!.id, transferUserId.value!);
        message.success(t("networkForm.share.transferred"));
        transferUserId.value = null;
        emit("transferred");
        await load();
      } catch (err) {
        message.error(extractError(err));
      } finally {
        transferring.value = false;
      }
    },
  });
}
</script>

<template>
  <n-modal
    v-model:show="visible"
    preset="card"
    :title="t('networkForm.share.title')"
    style="width: 640px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-spin :show="loading">
      <n-alert type="info" :show-icon="true" class="mb">
        {{ t("networkForm.share.alertPrefix") }}<strong>{{ t("networkForm.share.permissionReadonly") }}</strong>{{ t("networkForm.share.alertReadonlySuffix") }}
        <strong>{{ t("networkForm.share.permissionManage") }}</strong>{{ t("networkForm.share.alertManageSuffix") }}
      </n-alert>

      <div v-if="network" class="owner-row">
        <span class="label">{{ t("networkForm.share.owner") }}</span>
        <n-tag size="small" :bordered="false" type="success">{{ ownerLabel }}</n-tag>
        <span class="owner-name">{{ network.name }}</span>
      </div>

      <n-divider title-placement="left" style="margin: 8px 0 12px">{{ t("networkForm.share.sharedUsers") }}</n-divider>
      <div v-if="shares && shares.items.length === 0" class="empty">{{ t("networkForm.share.noShares") }}</div>
      <div v-for="item in shares?.items ?? []" :key="item.userId" class="share-row">
        <div class="share-user">
          <span class="mono">{{ item.username }}</span>
          <span class="sub">{{ item.displayName || "-" }} · {{ roleLabel(item.role) }}</span>
        </div>
        <n-select
          :value="item.permission"
          size="small"
          style="width: 120px"
          :options="permissionOptions"
          @update:value="(v: 'view' | 'manage') => changePermission(item, v)"
        />
        <n-button size="tiny" quaternary type="error" @click="removeShare(item)">
          <template #icon><n-icon :component="TrashOutline" /></template>
        </n-button>
      </div>

      <n-divider title-placement="left" style="margin: 16px 0 12px">{{ t("networkForm.share.addShare") }}</n-divider>
      <div class="add-row">
        <n-select
          v-model:value="selectedUserId"
          filterable
          clearable
          size="small"
          :placeholder="t('networkForm.share.selectUser')"
          :options="candidateOptions"
          style="flex: 1"
        />
        <n-select
          v-model:value="selectedPermission"
          size="small"
          style="width: 120px"
          :options="permissionOptions"
        />
        <n-button size="small" type="primary" :loading="saving" @click="addShare">{{ t("networkForm.share.shareAction") }}</n-button>
      </div>

      <template v-if="canTransfer">
        <n-divider title-placement="left" style="margin: 18px 0 12px">{{ t("networkForm.share.transferSection") }}</n-divider>
        <n-alert type="warning" :show-icon="true" class="mb" style="font-size: 12px">
          {{ t("networkForm.share.transferAlert") }}
        </n-alert>
        <div class="add-row">
          <n-select
            v-model:value="transferUserId"
            filterable
            clearable
            size="small"
            :placeholder="t('networkForm.share.selectNewOwner')"
            :options="transferOptions"
            style="flex: 1"
          />
          <n-button size="small" type="warning" :loading="transferring" @click="transfer">
            {{ t("networkForm.share.transferAction") }}
          </n-button>
        </div>
      </template>
    </n-spin>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">{{ t("common.close") }}</n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.mb {
  margin-bottom: 14px;
}
.owner-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.owner-row .label {
  font-size: 13px;
  opacity: 0.65;
}
.owner-name {
  font-weight: 600;
}
.share-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  border-bottom: 1px dashed rgba(128, 128, 128, 0.18);
}
.share-user {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}
.share-user .sub {
  font-size: 12px;
  opacity: 0.55;
}
.empty {
  font-size: 13px;
  opacity: 0.55;
  padding: 4px 0 8px;
}
.add-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
}
</style>
