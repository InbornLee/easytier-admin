<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import type { DataTableColumns } from "naive-ui";
import { NButton, NIcon, NTag } from "naive-ui";
import {
  ArrowBackOutline,
  PlayOutline,
  StopOutline,
  RefreshOutline,
  AddOutline,
  KeyOutline,
  TrashOutline,
  CreateOutline,
  LinkOutline,
  ShieldOutline,
  ShareSocialOutline,
} from "@vicons/ionicons5";
import StatusTag from "@/components/StatusTag.vue";
import CopyText from "@/components/CopyText.vue";
import TopologyGraph from "@/components/TopologyGraph.vue";
import LogViewer, { type LogLine } from "@/components/LogViewer.vue";
import NodeFormModal from "@/components/NodeFormModal.vue";
import NodeJoinModal from "@/components/NodeJoinModal.vue";
import CredentialFormModal from "@/components/CredentialFormModal.vue";
import RotateCredentialModal from "@/components/RotateCredentialModal.vue";
import NetworkFormModal from "@/components/NetworkFormModal.vue";
import NetworkShareModal from "@/components/NetworkShareModal.vue";
import { useAuthStore } from "@/stores/auth";
import { networkApi, nodeApi, logApi, credentialApi } from "@/api";
import { extractError } from "@/api/client";
import { message, dialog } from "@/utils/feedback";
import {
  credentialStatusLabel,
  costInfo,
  formatBytes,
  formatDuration,
  formatTime,
} from "@/utils/format";
import { useLogStream } from "@/composables/useLogStream";
import type {
  Credential,
  LiveState,
  Network,
  NodeItem,
  RouteListItem,
  Topology,
} from "@/types";

const route = useRoute();
const router = useRouter();
const networkId = computed(() => route.params.id as string);

const network = ref<Network | null>(null);
const live = ref<LiveState | null>(null);
const nodes = ref<NodeItem[]>([]);
const credentials = ref<Credential[]>([]);
const routes = ref<RouteListItem[]>([]);
const topology = ref<Topology | null>(null);
const configText = ref("");
const historyLogs = ref<LogLine[]>([]);

const activeTab = ref("nodes");
const loading = ref(false);
const actionLoading = ref<string | null>(null);

const showNodeModal = ref(false);
const editingNode = ref<NodeItem | null>(null);
const showJoinModal = ref(false);
const joinNodeId = ref<string | null>(null);
const showRotateModal = ref(false);
const rotateNode = ref<NodeItem | null>(null);
const showCredModal = ref(false);
const showEditModal = ref(false);
const showShareModal = ref(false);

const auth = useAuthStore();
const canManage = computed(
  () =>
    auth.user?.role === "admin" ||
    network.value?.access === "owner" ||
    network.value?.access === "manage",
);

const nodeSummary = computed(() => {
  const total = nodes.value.length;
  const online = nodes.value.filter(
    (n) => n.live?.online || n.status === "online",
  ).length;
  return { total, online, offline: Math.max(0, total - online) };
});

const { lines: liveLogs, connected } = useLogStream(networkId, 800);
const allLogs = computed<LogLine[]>(() => [...historyLogs.value, ...liveLogs.value]);
let refreshTimer: number | null = null;

async function loadBase() {
  loading.value = true;
  try {
    network.value = await networkApi.get(networkId.value);
  } catch (err) {
    message.error(extractError(err));
    router.push({ name: "networks" });
  } finally {
    loading.value = false;
  }
}

async function loadTab(tab: string) {
  const id = networkId.value;
  try {
    if (tab === "overview") {
      live.value = await networkApi.live(id);
      configText.value = await networkApi.config(id);
    } else if (tab === "nodes") {
      nodes.value = await networkApi.nodes(id);
    } else if (tab === "topology") {
      topology.value = await networkApi.topology(id);
    } else if (tab === "credentials") {
      credentials.value = await networkApi.credentials(id);
    } else if (tab === "routes") {
      const res = await networkApi.routes(id);
      routes.value = res.routes;
    } else if (tab === "logs") {
      const res = await logApi.nodes({ networkId: id, pageSize: 200 });
      historyLogs.value = [...res.items].reverse();
    }
  } catch (err) {
    message.error(extractError(err));
  }
}

async function loadAll() {
  await loadBase();
  await loadTab(activeTab.value);
  if (activeTab.value !== "overview") await loadTab("overview");
}

watch(activeTab, (t) => loadTab(t));
watch(networkId, () => loadAll());

async function doAction(action: "start" | "stop" | "restart") {
  actionLoading.value = action;
  try {
    await networkApi[action](networkId.value);
    message.success("操作成功");
    await loadAll();
  } catch (err) {
    message.error(extractError(err));
  } finally {
    actionLoading.value = null;
  }
}

function openCreateNode() {
  editingNode.value = null;
  showNodeModal.value = true;
}

function openEditNode(node: NodeItem) {
  editingNode.value = node;
  showNodeModal.value = true;
}

function openJoin(node: NodeItem) {
  joinNodeId.value = node.id;
  showJoinModal.value = true;
}

async function onNodeSaved(payload: { node: NodeItem; credentialSecret?: string }) {
  await loadTab("nodes");
  if (payload.credentialSecret) {
    joinNodeId.value = payload.node.id;
    showJoinModal.value = true;
  }
}

function removeNode(node: NodeItem) {
  dialog.error({
    title: "删除节点",
    content: `确定删除节点「${node.name}」吗？其凭据也会被撤销。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await nodeApi.remove(node.id);
        message.success("已删除");
        await loadTab("nodes");
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

function removeNetwork() {
  if (!network.value) return;
  dialog.error({
    title: "删除网络",
    content: `确定删除网络「${network.value.name}」吗？该网络下的节点与凭据将一并删除，且不可恢复。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await networkApi.remove(networkId.value);
        message.success("网络已删除");
        router.push({ name: "networks" });
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

function rotateCredential(node: NodeItem) {
  rotateNode.value = node;
  showRotateModal.value = true;
}

async function onNodeRotated(payload: { node: NodeItem }) {
  message.success("凭据已更换，请查看新的接入信息");
  joinNodeId.value = payload.node.id;
  showJoinModal.value = true;
  await loadTab("nodes");
}

async function onCredentialCreated() {
  await loadTab("credentials");
}

async function revokeCredential(cred: Credential) {
  dialog.warning({
    title: "撤销凭据",
    content: "撤销后使用该凭据的节点将被移除，且不可恢复。确定继续？",
    positiveText: "撤销",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await credentialApi.revoke(cred.id);
        message.success("已撤销");
        await loadTab("credentials");
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

async function deleteCredential(cred: Credential) {
  dialog.error({
    title: "删除凭据",
    content: "删除后将同时撤销该凭据并移除记录，使用该凭据的节点将被移除。确定删除？",
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await credentialApi.remove(cred.id);
        message.success("已删除");
        await loadTab("credentials");
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

const nodeColumns = computed<DataTableColumns<NodeItem>>(() => [
  {
    title: "节点",
    key: "name",
    render: (row) =>
      h("div", {}, [
        h("div", { class: "cell-name" }, [
          row.name,
          !row.credentialId
            ? h("span", { class: "no-cred" }, "无凭据")
            : null,
        ]),
        h("div", { class: "cell-sub mono" }, `${row.hostname} · ${row.ipv4 || "DHCP"}`),
      ]),
  },
  {
    title: "状态",
    key: "status",
    width: 100,
    render: (row) => h(StatusTag, { kind: "node", status: row.status }),
  },
  {
    title: "链路",
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
    title: "延迟",
    key: "lat",
    width: 90,
    render: (row) =>
      row.live?.online && row.live.latMs != null ? `${row.live.latMs} ms` : "-",
  },
  {
    title: "流量 (收/发)",
    key: "traffic",
    width: 160,
    render: (row) =>
      row.live?.online
        ? `${formatBytes(row.live.rxBytes)} / ${formatBytes(row.live.txBytes)}`
        : "-",
  },
  {
    title: "隧道 / NAT",
    key: "tunnel",
    width: 130,
    render: (row) =>
      row.live?.online
        ? h("span", { class: "mono" }, `${row.live.tunnelProto} · ${row.live.natType}`)
        : "-",
  },
  {
    title: "最后在线",
    key: "lastSeenAt",
    width: 165,
    render: (row) => (row.lastSeenAt ? formatTime(row.lastSeenAt) : "-"),
  },
  {
    title: "操作",
    key: "actions",
    width: 280,
    fixed: "right",
    render: (row) => {
      const actions = [
        h(
          NButton,
          {
            size: "tiny",
            quaternary: true,
            type: "primary",
            onClick: () => openJoin(row),
          },
          { icon: () => h(NIcon, { component: LinkOutline }), default: () => "接入" },
        ),
      ];
      if (canManage.value) {
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              onClick: () => openEditNode(row),
            },
            { icon: () => h(NIcon, { component: CreateOutline }), default: () => "编辑" },
          ),
        );
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              onClick: () => rotateCredential(row),
            },
            { icon: () => h(NIcon, { component: ShieldOutline }), default: () => "换凭据" },
          ),
        );
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              type: "error",
              onClick: () => removeNode(row),
            },
            { icon: () => h(NIcon, { component: TrashOutline }), default: () => "删除" },
          ),
        );
      }
      return h("div", { class: "row-actions" }, actions);
    },
  },
]);

const credentialColumns = computed<DataTableColumns<Credential>>(() => [
  {
    title: "凭据 ID",
    key: "credentialId",
    render: (row) => h("span", { class: "mono" }, row.credentialId),
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
    width: 140,
    render: (row) =>
      row.status === "active" ? formatDuration(row.remainingSeconds) : credentialStatusLabel(row.status),
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
    width: 160,
    render: (row) =>
      h("span", {}, [
        row.allowRelay ? "中继 " : "",
        row.reusable ? "复用 " : "独占 ",
        row.groups.length ? `分组:${row.groups.join(",")}` : "",
      ]),
  },
  {
    title: "操作",
    key: "actions",
    width: 160,
    fixed: "right",
    render: (row) => {
      if (!canManage.value) return h("span", {}, "-");
      const actions: ReturnType<typeof h>[] = [];
      if (row.status === "active") {
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              type: "warning",
              onClick: () => revokeCredential(row),
            },
            { default: () => "撤销" },
          ),
        );
      }
      actions.push(
        h(
          NButton,
          {
            size: "tiny",
            quaternary: true,
            type: "error",
            onClick: () => deleteCredential(row),
          },
          { default: () => "删除" },
        ),
      );
      return h("div", { class: "row-actions" }, actions);
    },
  },
]);

const routeColumns: DataTableColumns<RouteListItem> = [
  { title: "虚拟 IP", key: "ipv4", width: 150 },
  { title: "主机名", key: "hostname" },
  { title: "代理网段", key: "proxy_cidrs" },
  { title: "下一跳", key: "next_hop_ipv4", width: 150 },
  { title: "跳数", key: "path_len", width: 80 },
  { title: "路径延迟", key: "path_latency", width: 100, render: (r) => `${r.path_latency} ms` },
  { title: "版本", key: "version", width: 100 },
];

const tabPanes = computed<Array<{ name: string; tab: string }>>(() => [
  { name: "nodes", tab: `节点 (${nodes.value.length})` },
  { name: "overview", tab: "概览" },
  { name: "topology", tab: "连接拓扑" },
  { name: "credentials", tab: `凭据 (${credentials.value.length})` },
  { name: "routes", tab: "路由表" },
  { name: "logs", tab: "运行日志" },
]);

const livePeerCount = computed(() =>
  Math.max(0, (live.value?.peers.length ?? 1) - 1),
);

onMounted(() => {
  void loadAll();
  // 自动刷新节点连接情况
  refreshTimer = window.setInterval(() => {
    if (activeTab.value === "nodes") void loadTab("nodes");
    else if (activeTab.value === "overview") void loadTab("overview");
    else if (activeTab.value === "topology") void loadTab("topology");
  }, 15000);
});
onUnmounted(() => {
  if (refreshTimer) window.clearInterval(refreshTimer);
});
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div class="title-row">
        <n-button quaternary circle size="small" @click="router.push({ name: 'networks' })">
          <template #icon><n-icon :component="ArrowBackOutline" /></template>
        </n-button>
        <div>
          <h1 class="page-title">
            {{ network?.name ?? "网络详情" }}
            <StatusTag
              v-if="network"
              :kind="'network'"
              :status="network.status"
              style="margin-left: 10px"
            />
          </h1>
          <div class="page-subtitle mono">
            {{ network?.networkName }} · {{ network?.ipv4 }} ·
            RPC {{ network?.rpcPort }}
          </div>
        </div>
      </div>
      <div class="header-actions">
        <n-button v-if="canManage" size="small" @click="showEditModal = true">
          <template #icon><n-icon :component="CreateOutline" /></template>
          编辑
        </n-button>
        <n-button v-if="canManage" size="small" @click="showShareModal = true">
          <template #icon><n-icon :component="ShareSocialOutline" /></template>
          分享
        </n-button>
        <n-button size="small" @click="loadAll">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          刷新
        </n-button>
        <n-button
          v-if="canManage && network?.status !== 'running'"
          size="small"
          type="success"
          :loading="actionLoading === 'start'"
          @click="doAction('start')"
        >
          <template #icon><n-icon :component="PlayOutline" /></template>
          启动
        </n-button>
        <template v-else-if="canManage">
          <n-button
            size="small"
            type="warning"
            :loading="actionLoading === 'stop'"
            @click="doAction('stop')"
          >
            <template #icon><n-icon :component="StopOutline" /></template>
            停止
          </n-button>
          <n-button
            size="small"
            :loading="actionLoading === 'restart'"
            @click="doAction('restart')"
          >
            <template #icon><n-icon :component="RefreshOutline" /></template>
            重启
          </n-button>
        </template>
        <n-button v-if="canManage" size="small" type="error" ghost @click="removeNetwork">
          <template #icon><n-icon :component="TrashOutline" /></template>
          删除网络
        </n-button>
      </div>
    </div>

    <n-spin :show="loading">
      <n-card size="small">
        <n-tabs v-model:value="activeTab" type="line" animated>
          <n-tab-pane
            v-for="pane in tabPanes"
            :key="pane.name"
            :name="pane.name"
            :tab="pane.tab"
          >
            <!-- 概览 -->
            <template v-if="pane.name === 'overview'">
              <n-grid :cols="24" :x-gap="16" :y-gap="16">
                <n-gi :span="12">
                  <n-descriptions title="网络信息" :column="1" size="small" bordered>
                    <n-descriptions-item label="EasyTier 网络标识">
                      {{ network?.networkName }}
                    </n-descriptions-item>
                    <n-descriptions-item label="虚拟网段">{{ network?.ipv4 }}</n-descriptions-item>
                    <n-descriptions-item label="共享节点主机名">
                      {{ network?.hostname }}
                    </n-descriptions-item>
                    <n-descriptions-item label="监听端口">
                      {{ network?.listenPort }} (RPC {{ network?.rpcPort }})
                    </n-descriptions-item>
                    <n-descriptions-item label="安全模式">
                      {{ network?.secureMode ? "已开启（Noise 握手）" : "关闭" }}
                    </n-descriptions-item>
                    <n-descriptions-item label="公共共享节点">
                      <span class="mono">{{ network?.externalNode || "-" }}</span>
                    </n-descriptions-item>
                    <n-descriptions-item label="自动启动">
                      {{ network?.autoStart ? "是" : "否" }}
                    </n-descriptions-item>
                  </n-descriptions>
                </n-gi>
                <n-gi :span="12">
                  <n-descriptions title="实时状态" :column="1" size="small" bordered>
                    <n-descriptions-item label="RPC 连通">
                      <n-tag :type="live?.online ? 'success' : 'error'" size="small" :bordered="false">
                        {{ live?.online ? "已连接" : "不可用" }}
                      </n-tag>
                    </n-descriptions-item>
                    <n-descriptions-item label="在线节点数">{{ livePeerCount }}</n-descriptions-item>
                    <n-descriptions-item label="本节点虚拟 IP">
                      {{ (live?.nodeInfo as any)?.ipv4_addr ?? network?.ipv4 }}
                    </n-descriptions-item>
                    <n-descriptions-item label="本节点 Peer ID">
                      {{ (live?.nodeInfo as any)?.peer_id ?? "-" }}
                    </n-descriptions-item>
                    <n-descriptions-item label="共享节点公钥">
                      <CopyText :text="network?.sharedNodePublicKey ?? ''" masked />
                    </n-descriptions-item>
                    <n-descriptions-item label="日志级别">
                      <n-tag size="small" :bordered="false">info</n-tag>
                    </n-descriptions-item>
                  </n-descriptions>
                  <n-alert
                    v-if="live?.error && !live.online"
                    type="warning"
                    :show-icon="true"
                    style="margin-top: 12px"
                  >
                    {{ live.error }}
                  </n-alert>
                </n-gi>
                <n-gi :span="24">
                  <n-collapse>
                    <n-collapse-item title="生成的配置文件 (config.toml)" name="config">
                      <template #header-extra>
                        <CopyText :text="configText" />
                      </template>
                      <pre class="code-block mono">{{ configText }}</pre>
                    </n-collapse-item>
                  </n-collapse>
                </n-gi>
              </n-grid>
            </template>

            <!-- 节点 -->
            <template v-else-if="pane.name === 'nodes'">
              <div class="node-summary" :class="{ online: nodeSummary.online > 0 }">
                <div class="ns-item">
                  <span class="ns-value">{{ nodeSummary.total }}</span>
                  <span class="ns-label">网络内节点</span>
                </div>
                <div class="ns-item">
                  <span class="ns-value online">{{ nodeSummary.online }}</span>
                  <span class="ns-label">在线</span>
                </div>
                <div class="ns-item">
                  <span class="ns-value offline">{{ nodeSummary.offline }}</span>
                  <span class="ns-label">离线 / 未接入</span>
                </div>
                <div class="ns-item">
                  <n-tag
                    :type="live?.online ? 'success' : 'error'"
                    size="small"
                    round
                    :bordered="false"
                  >
                    {{ live?.online ? "共享节点在线" : "共享节点离线" }}
                  </n-tag>
                </div>
                <span class="ns-hint">每 15 秒自动刷新</span>
              </div>
              <div class="tab-toolbar">
                <n-button v-if="canManage" size="small" type="primary" @click="openCreateNode">
                  <template #icon><n-icon :component="AddOutline" /></template>
                  新建节点
                </n-button>
                <n-button size="small" @click="loadTab('nodes')">
                  <template #icon><n-icon :component="RefreshOutline" /></template>
                  刷新
                </n-button>
              </div>
              <n-data-table
                :columns="nodeColumns"
                :data="nodes"
                :scroll-x="1050"
                size="small"
                :row-key="(row: NodeItem) => row.id"
              />
            </template>

            <!-- 拓扑 -->
            <template v-else-if="pane.name === 'topology'">
              <div class="tab-toolbar">
                <n-button size="small" @click="loadTab('topology')">
                  <template #icon><n-icon :component="RefreshOutline" /></template>
                  刷新拓扑
                </n-button>
                <span v-if="topology" class="tab-hint">
                  更新于 {{ formatTime(topology.updatedAt) }}
                </span>
              </div>
              <TopologyGraph :topology="topology" />
            </template>

            <!-- 凭据 -->
            <template v-else-if="pane.name === 'credentials'">
              <div class="tab-toolbar">
                <n-button v-if="canManage" size="small" type="primary" @click="showCredModal = true">
                  <template #icon><n-icon :component="KeyOutline" /></template>
                  签发临时凭据
                </n-button>
                <n-button size="small" @click="loadTab('credentials')">
                  <template #icon><n-icon :component="RefreshOutline" /></template>
                  刷新
                </n-button>
              </div>
              <n-data-table
                :columns="credentialColumns"
                :data="credentials"
                :scroll-x="900"
                size="small"
                :row-key="(row: Credential) => row.id"
              />
            </template>

            <!-- 路由 -->
            <template v-else-if="pane.name === 'routes'">
              <n-data-table
                :columns="routeColumns"
                :data="routes"
                :scroll-x="900"
                size="small"
              />
            </template>

            <!-- 日志 -->
            <template v-else-if="pane.name === 'logs'">
              <div class="tab-toolbar">
                <n-tag :type="connected ? 'success' : 'default'" size="small" :bordered="false">
                  {{ connected ? "实时日志已连接" : "实时日志未连接" }}
                </n-tag>
                <span class="tab-hint">日志同时写入 data/logs/&lt;网络ID&gt;/</span>
              </div>
              <LogViewer :lines="allLogs" :height="520" />
            </template>
          </n-tab-pane>
        </n-tabs>
      </n-card>
    </n-spin>

    <NetworkFormModal v-model:show="showEditModal" :network="network" @saved="loadAll" />
    <NetworkShareModal
      v-model:show="showShareModal"
      :network="network"
      @transferred="loadAll"
    />
    <NodeFormModal
      v-model:show="showNodeModal"
      :networks="network ? [network] : []"
      :network-id="networkId"
      :node="editingNode"
      @saved="onNodeSaved"
    />
    <NodeJoinModal v-model:show="showJoinModal" :node-id="joinNodeId" />
    <RotateCredentialModal
      v-model:show="showRotateModal"
      :node="rotateNode"
      @rotated="onNodeRotated"
    />
    <CredentialFormModal
      v-model:show="showCredModal"
      :networks="network ? [network] : []"
      :network-id="networkId"
      @created="onCredentialCreated"
    />
  </div>
</template>

<style scoped>
.title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.header-actions {
  display: flex;
  gap: 8px;
}
.tab-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}
.tab-hint {
  font-size: 12px;
  opacity: 0.6;
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
.node-summary {
  display: flex;
  align-items: center;
  gap: 26px;
  padding: 14px 18px;
  margin-bottom: 14px;
  border-radius: 10px;
  border: 1px solid rgba(128, 128, 128, 0.18);
  background: rgba(128, 128, 128, 0.04);
  flex-wrap: wrap;
}
.node-summary.online {
  border-color: rgba(22, 163, 74, 0.35);
}
.ns-item {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
}
.ns-value {
  font-size: 22px;
  font-weight: 700;
}
.ns-value.online {
  color: #16a34a;
}
.ns-value.offline {
  color: #94a3b8;
}
.ns-label {
  font-size: 12px;
  opacity: 0.6;
}
.ns-hint {
  margin-left: auto;
  font-size: 12px;
  opacity: 0.5;
}
</style>
