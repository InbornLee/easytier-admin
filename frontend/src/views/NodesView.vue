<script setup lang="ts">
import { computed, h, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import type { DataTableColumns } from "naive-ui";
import { NButton, NIcon, NTag } from "naive-ui";
import {
  AddOutline,
  RefreshOutline,
  LinkOutline,
  CreateOutline,
  TrashOutline,
  ShieldOutline,
} from "@vicons/ionicons5";
import StatusTag from "@/components/StatusTag.vue";
import NodeFormModal from "@/components/NodeFormModal.vue";
import NodeJoinModal from "@/components/NodeJoinModal.vue";
import RotateCredentialModal from "@/components/RotateCredentialModal.vue";
import { networkApi, nodeApi } from "@/api";
import { extractError } from "@/api/client";
import { message, dialog } from "@/utils/feedback";
import { costInfo, formatBytes, formatTime } from "@/utils/format";
import type { Network, NodeItem } from "@/types";

const router = useRouter();
const { t } = useI18n();
const loading = ref(false);
const nodes = ref<NodeItem[]>([]);
const networks = ref<Network[]>([]);
const filterNetwork = ref<string | null>(null);

const showNodeModal = ref(false);
const editingNode = ref<NodeItem | null>(null);
const showJoinModal = ref(false);
const joinNodeId = ref<string | null>(null);
const showRotateModal = ref(false);
const rotateNode = ref<NodeItem | null>(null);

const networkMap = computed(() => {
  const map = new Map<string, Network>();
  networks.value.forEach((n) => map.set(n.id, n));
  return map;
});

const filteredNodes = computed(() =>
  filterNetwork.value
    ? nodes.value.filter((n) => n.networkId === filterNetwork.value)
    : nodes.value,
);

async function load() {
  loading.value = true;
  try {
    const [n, net] = await Promise.all([
      nodeApi.list(undefined, true),
      networkApi.list(),
    ]);
    nodes.value = n;
    networks.value = net;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  editingNode.value = null;
  showNodeModal.value = true;
}
function openEdit(node: NodeItem) {
  editingNode.value = node;
  showNodeModal.value = true;
}
function openJoin(node: NodeItem) {
  joinNodeId.value = node.id;
  showJoinModal.value = true;
}
async function onSaved(payload: { node: NodeItem; credentialSecret?: string }) {
  await load();
  if (payload.credentialSecret) {
    joinNodeId.value = payload.node.id;
    showJoinModal.value = true;
  }
}
function rotate(node: NodeItem) {
  rotateNode.value = node;
  showRotateModal.value = true;
}

async function onRotated(payload: { node: NodeItem }) {
  joinNodeId.value = payload.node.id;
  showJoinModal.value = true;
  await load();
}
function remove(node: NodeItem) {
  dialog.error({
    title: t("nodes.deleteTitle"),
    content: t("nodes.deleteConfirm", { name: node.name }),
    positiveText: t("common.delete"),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      try {
        await nodeApi.remove(node.id);
        message.success(t("nodes.deleted"));
        await load();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

const columns = computed<DataTableColumns<NodeItem>>(() => [
  {
    title: t("nodes.colNode"),
    key: "name",
    render: (row) =>
      h("div", {}, [
        h("div", { class: "cell-name" }, [
          row.name,
          !row.credentialId
            ? h("span", { class: "no-cred" }, t("nodes.noCredential"))
            : null,
        ]),
        h("div", { class: "cell-sub mono" }, `${row.hostname} · ${row.ipv4 || "DHCP"}`),
      ]),
  },
  {
    title: t("nodes.colNetwork"),
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
    title: t("common.status"),
    key: "status",
    width: 90,
    render: (row) => h(StatusTag, { kind: "node", status: row.status }),
  },
  {
    title: t("nodes.colLink"),
    key: "cost",
    width: 110,
    render: (row) => {
      if (!row.live?.online) return "-";
      const info = costInfo(row.live.cost);
      return h(
        NTag,
        { type: info.type, size: "small", bordered: false, round: true },
        { default: () => info.text },
      );
    },
  },
  {
    title: t("nodes.colLatency"),
    key: "lat",
    width: 90,
    render: (row) =>
      row.live?.online && row.live.latMs != null ? `${row.live.latMs} ms` : "-",
  },
  {
    title: t("nodes.colTraffic"),
    key: "traffic",
    width: 150,
    render: (row) =>
      row.live?.online
        ? `${formatBytes(row.live.rxBytes)} / ${formatBytes(row.live.txBytes)}`
        : "-",
  },
  {
    title: t("nodes.colTunnel"),
    key: "tunnel",
    width: 130,
    render: (row) =>
      row.live?.online
        ? h("span", { class: "mono" }, `${row.live.tunnelProto} · ${row.live.natType}`)
        : "-",
  },
  {
    title: t("nodes.colLastSeen"),
    key: "lastSeenAt",
    width: 165,
    render: (row) => (row.lastSeenAt ? formatTime(row.lastSeenAt) : "-"),
  },
  {
    title: t("common.actions"),
    key: "actions",
    width: 280,
    fixed: "right",
    render: (row) =>
      h("div", { class: "row-actions" }, [
        h(
          NButton,
          { size: "tiny", quaternary: true, type: "primary", onClick: () => openJoin(row) },
          { icon: () => h(NIcon, { component: LinkOutline }), default: () => t("nodes.actionJoin") },
        ),
        h(
          NButton,
          { size: "tiny", quaternary: true, onClick: () => openEdit(row) },
          { icon: () => h(NIcon, { component: CreateOutline }), default: () => t("common.edit") },
        ),
        h(
          NButton,
          { size: "tiny", quaternary: true, onClick: () => rotate(row) },
          { icon: () => h(NIcon, { component: ShieldOutline }), default: () => t("nodes.actionRotate") },
        ),
        h(
          NButton,
          { size: "tiny", quaternary: true, type: "error", onClick: () => remove(row) },
          { icon: () => h(NIcon, { component: TrashOutline }), default: () => t("common.delete") },
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
        <h1 class="page-title">{{ t("nodes.title") }}</h1>
        <div class="page-subtitle">{{ t("nodes.subtitle") }}</div>
      </div>
      <div class="header-actions">
        <n-select
          v-model:value="filterNetwork"
          :options="networks.map((n) => ({ label: n.name, value: n.id }))"
          :placeholder="t('nodes.allNetworks')"
          clearable
          size="small"
          style="width: 170px"
        />
        <n-button size="small" @click="load">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          {{ t("common.refresh") }}
        </n-button>
        <n-button size="small" type="primary" @click="openCreate">
          <template #icon><n-icon :component="AddOutline" /></template>
          {{ t("nodes.createTitle") }}
        </n-button>
      </div>
    </div>

    <n-card size="small">
      <n-data-table
        :columns="columns"
        :data="filteredNodes"
        :loading="loading"
        :scroll-x="1150"
        size="small"
        :row-key="(row: NodeItem) => row.id"
      />
    </n-card>

    <NodeFormModal
      v-model:show="showNodeModal"
      :networks="networks"
      :node="editingNode"
      @saved="onSaved"
    />
    <NodeJoinModal v-model:show="showJoinModal" :node-id="joinNodeId" />
    <RotateCredentialModal
      v-model:show="showRotateModal"
      :node="rotateNode"
      @rotated="onRotated"
    />
  </div>
</template>

<style scoped>
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.cell-name {
  font-weight: 600;
}
.cell-sub {
  font-size: 12px;
  opacity: 0.6;
}
.row-actions {
  display: flex;
  gap: 2px;
}
.no-cred {
  margin-left: 8px;
  font-size: 11px;
  color: #f5a524;
  border: 1px solid rgba(245, 165, 36, 0.5);
  border-radius: 4px;
  padding: 0 5px;
  font-weight: 500;
}
.net-link {
  color: #2563eb;
  cursor: pointer;
}
</style>
