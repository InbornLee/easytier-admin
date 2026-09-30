<script setup lang="ts">
import { computed, ref, watch } from "vue";
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
const loading = ref(false);
const saving = ref(false);
const shares = ref<NetworkShares | null>(null);
const users = ref<SelectableUser[]>([]);
const selectedUserId = ref<string | null>(null);
const selectedPermission = ref<"view" | "manage">("view");
const transferUserId = ref<string | null>(null);
const transferring = ref(false);

const permissionOptions = [
  { label: "只读", value: "view" },
  { label: "可管理", value: "manage" },
];

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
  if (props.network.ownerId && props.network.ownerId === auth.user?.id) return "我";
  return shares.value?.owner?.username ?? props.network.ownerId ?? "未知";
});

function roleLabel(role: string) {
  return ({ admin: "管理员", operator: "运维", viewer: "访客" } as Record<string, string>)[
    role
  ] ?? role;
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
    message.warning("请选择要分享的用户");
    return;
  }
  saving.value = true;
  try {
    await networkApi.share(props.network.id, {
      userId: selectedUserId.value,
      permission: selectedPermission.value,
    });
    message.success("已分享");
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
    message.success("权限已更新");
    await load();
  } catch (err) {
    message.error(extractError(err));
  }
}

function removeShare(item: NetworkShare) {
  if (!props.network) return;
  dialog.warning({
    title: "取消分享",
    content: `确定取消对「${item.username}」的分享吗？`,
    positiveText: "取消分享",
    negativeText: "返回",
    onPositiveClick: async () => {
      try {
        await networkApi.unshare(props.network!.id, item.userId);
        message.success("已取消分享");
        await load();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

function transfer() {
  if (!props.network || !transferUserId.value) {
    message.warning("请选择要转移到的用户");
    return;
  }
  const target = users.value.find((u) => u.id === transferUserId.value);
  dialog.warning({
    title: "转移网络归属",
    content: `确定将网络「${props.network.name}」的归属转移给「${
      target?.username ?? ""
    }」吗？原所有者将保留「可管理」权限。`,
    positiveText: "确认转移",
    negativeText: "取消",
    onPositiveClick: async () => {
      transferring.value = true;
      try {
        await networkApi.transfer(props.network!.id, transferUserId.value!);
        message.success("归属已转移");
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
    title="分享网络"
    style="width: 640px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-spin :show="loading">
      <n-alert type="info" :show-icon="true" class="mb">
        被分享的用户可在「网络管理」中看到该网络。<strong>只读</strong>仅可查看与获取接入信息；
        <strong>可管理</strong>可编辑网络、启停及管理节点与凭据。
      </n-alert>

      <div v-if="network" class="owner-row">
        <span class="label">所有者</span>
        <n-tag size="small" :bordered="false" type="success">{{ ownerLabel }}</n-tag>
        <span class="owner-name">{{ network.name }}</span>
      </div>

      <n-divider title-placement="left" style="margin: 8px 0 12px">已分享用户</n-divider>
      <div v-if="shares && shares.items.length === 0" class="empty">尚未分享给任何用户</div>
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

      <n-divider title-placement="left" style="margin: 16px 0 12px">添加分享</n-divider>
      <div class="add-row">
        <n-select
          v-model:value="selectedUserId"
          filterable
          clearable
          size="small"
          placeholder="选择用户"
          :options="candidateOptions"
          style="flex: 1"
        />
        <n-select
          v-model:value="selectedPermission"
          size="small"
          style="width: 120px"
          :options="permissionOptions"
        />
        <n-button size="small" type="primary" :loading="saving" @click="addShare">分享</n-button>
      </div>

      <template v-if="canTransfer">
        <n-divider title-placement="left" style="margin: 18px 0 12px">归属转移</n-divider>
        <n-alert type="warning" :show-icon="true" class="mb" style="font-size: 12px">
          转移后该网络归属于目标用户；原所有者将保留「可管理」权限。此操作会记录审计日志。
        </n-alert>
        <div class="add-row">
          <n-select
            v-model:value="transferUserId"
            filterable
            clearable
            size="small"
            placeholder="选择新的所有者"
            :options="transferOptions"
            style="flex: 1"
          />
          <n-button size="small" type="warning" :loading="transferring" @click="transfer">
            转移归属
          </n-button>
        </div>
      </template>
    </n-spin>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">关闭</n-button>
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
