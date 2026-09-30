<script setup lang="ts">
import { computed, h, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import type { DataTableColumns } from "naive-ui";
import { NButton, NIcon } from "naive-ui";
import { AddOutline, RefreshOutline, TrashOutline } from "@vicons/ionicons5";
import StatusTag from "@/components/StatusTag.vue";
import CopyText from "@/components/CopyText.vue";
import CredentialFormModal from "@/components/CredentialFormModal.vue";
import { networkApi, credentialApi } from "@/api";
import { extractError } from "@/api/client";
import { message, dialog } from "@/utils/feedback";
import { formatDuration, formatTime } from "@/utils/format";
import type { Credential, Network } from "@/types";

const router = useRouter();
const loading = ref(false);
const credentials = ref<Credential[]>([]);
const networks = ref<Network[]>([]);
const filterNetwork = ref<string | null>(null);
const showCredModal = ref(false);

const secretModal = ref(false);
const secretValue = ref("");
const secretCredential = ref<Credential | null>(null);

const networkMap = computed(() => {
  const map = new Map<string, Network>();
  networks.value.forEach((n) => map.set(n.id, n));
  return map;
});

const filtered = computed(() =>
  filterNetwork.value
    ? credentials.value.filter((c) => c.networkId === filterNetwork.value)
    : credentials.value,
);

async function load() {
  loading.value = true;
  try {
    const [c, n] = await Promise.all([credentialApi.list(), networkApi.list()]);
    credentials.value = c;
    networks.value = n;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    loading.value = false;
  }
}

function onCreated(payload: { credential: Credential; secret: string }) {
  secretCredential.value = payload.credential;
  secretValue.value = payload.secret;
  secretModal.value = true;
  load();
}

function revoke(cred: Credential) {
  dialog.warning({
    title: "撤销凭据",
    content: "撤销后使用该凭据的节点将被移除，确定继续？",
    positiveText: "撤销",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await credentialApi.revoke(cred.id);
        message.success("已撤销");
        await load();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

function remove(cred: Credential) {
  dialog.error({
    title: "删除凭据",
    content: "删除后将同时撤销该凭据并移除记录，使用该凭据的节点将被移除。确定删除？",
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await credentialApi.remove(cred.id);
        message.success("已删除");
        await load();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

async function cleanupInvalid() {
  const invalid = credentials.value.filter((c) => c.status !== "active");
  if (invalid.length === 0) {
    message.info("没有已失效的凭据需要清理");
    return;
  }
  dialog.warning({
    title: "清理失效凭据",
    content: `将删除 ${invalid.length} 条已过期/已撤销的凭据记录（会同步撤销），确定继续？`,
    positiveText: "清理",
    negativeText: "取消",
    onPositiveClick: async () => {
      let ok = 0;
      for (const c of invalid) {
        try {
          await credentialApi.remove(c.id);
          ok += 1;
        } catch {
          /* ignore individual failure */
        }
      }
      message.success(`已清理 ${ok} 条凭据`);
      await load();
    },
  });
}

const columns = computed<DataTableColumns<Credential>>(() => [
  {
    title: "凭据 ID",
    key: "credentialId",
    render: (row) => h("span", { class: "mono" }, row.credentialId),
  },
  {
    title: "所属网络",
    key: "networkId",
    width: 140,
    render: (row) =>
      h(
        "a",
        {
          class: "net-link",
          onClick: () =>
            router.push({ name: "network-detail", params: { id: row.networkId } }),
        },
        networkMap.value.get(row.networkId)?.name ?? row.networkId,
      ),
  },
  {
    title: "状态",
    key: "status",
    width: 100,
    render: (row) => h(StatusTag, { kind: "credential", status: row.status }),
  },
  {
    title: "剩余有效期",
    key: "remaining",
    width: 130,
    render: (row) =>
      row.status === "active" ? formatDuration(row.remainingSeconds) : "-",
  },
  {
    title: "到期时间",
    key: "expiresAt",
    width: 170,
    render: (row) => formatTime(row.expiresAt),
  },
  {
    title: "权限",
    key: "perms",
    render: (row) =>
      [
        row.allowRelay ? "允许中继" : "禁止中继",
        row.reusable ? "可复用" : "独占",
        row.groups.length ? `分组:${row.groups.join(",")}` : null,
        row.allowedProxyCidrs.length ? `代理:${row.allowedProxyCidrs.join(",")}` : null,
      ]
        .filter(Boolean)
        .join(" · "),
  },
  {
    title: "操作",
    key: "actions",
    width: 160,
    fixed: "right",
    render: (row) =>
      h("div", { class: "row-actions" }, [
        row.status === "active"
          ? h(
              NButton,
              {
                size: "tiny",
                quaternary: true,
                type: "warning",
                onClick: () => revoke(row),
              },
              { default: () => "撤销" },
            )
          : null,
        h(
          NButton,
          { size: "tiny", quaternary: true, type: "error", onClick: () => remove(row) },
          { icon: () => h(NIcon, { component: TrashOutline }) },
        ),
      ]),
  },
]);

onMounted(load);
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h1 class="page-title">凭据管理</h1>
        <div class="page-subtitle">通过短期凭据让设备安全接入网络，无需分发网络主密钥</div>
      </div>
      <div class="header-actions">
        <n-select
          v-model:value="filterNetwork"
          :options="networks.map((n) => ({ label: n.name, value: n.id }))"
          placeholder="全部网络"
          clearable
          size="small"
          style="width: 170px"
        />
        <n-button size="small" @click="load">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          刷新
        </n-button>
        <n-button size="small" @click="cleanupInvalid">
          <template #icon><n-icon :component="TrashOutline" /></template>
          清理失效
        </n-button>
        <n-button size="small" type="primary" @click="showCredModal = true">
          <template #icon><n-icon :component="AddOutline" /></template>
          签发临时凭据
        </n-button>
      </div>
    </div>

    <n-card size="small">
      <n-data-table
        :columns="columns"
        :data="filtered"
        :loading="loading"
        :scroll-x="1100"
        size="small"
        :row-key="(row: Credential) => row.id"
      />
    </n-card>

    <CredentialFormModal
      v-model:show="showCredModal"
      :networks="networks"
      @created="onCreated"
    />

    <n-modal
      v-model:show="secretModal"
      preset="card"
      title="凭据已签发"
      style="width: 560px; max-width: 94vw"
    >
      <n-alert type="warning" :show-icon="true" class="mb">
        凭据密钥仅在此处显示一次，请立即复制并通过安全渠道发送给目标设备。
      </n-alert>
      <n-descriptions v-if="secretCredential" :column="1" size="small" bordered class="mb">
        <n-descriptions-item label="凭据 ID">
          <span class="mono">{{ secretCredential.credentialId }}</span>
        </n-descriptions-item>
        <n-descriptions-item label="到期时间">
          {{ formatTime(secretCredential.expiresAt) }}
        </n-descriptions-item>
        <n-descriptions-item label="凭据密钥">
          <CopyText :text="secretValue" />
        </n-descriptions-item>
      </n-descriptions>
      <div class="code-block mono">{{ secretValue }}</div>
      <template #footer>
        <div class="modal-footer">
          <n-button type="primary" @click="secretModal = false">我已保存</n-button>
        </div>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.row-actions {
  display: flex;
  gap: 4px;
}
.net-link {
  color: #2563eb;
  cursor: pointer;
}
.mb {
  margin-bottom: 12px;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
}
</style>
