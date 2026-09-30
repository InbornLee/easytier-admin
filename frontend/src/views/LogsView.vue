<script setup lang="ts">
import { computed, h, onMounted, ref, watch } from "vue";
import type { DataTableColumns } from "naive-ui";
import { RefreshOutline, TrashOutline, SearchOutline } from "@vicons/ionicons5";
import LogViewer, { type LogLine } from "@/components/LogViewer.vue";
import { logApi, networkApi } from "@/api";
import { extractError } from "@/api/client";
import { message, dialog } from "@/utils/feedback";
import { actionLabel, formatTime } from "@/utils/format";
import { useLogStream } from "@/composables/useLogStream";
import type { AuditLog, Network, NodeLog } from "@/types";

const activeTab = ref("audit");

// ---------------- 审计日志 ----------------
const auditLoading = ref(false);
const auditItems = ref<AuditLog[]>([]);
const auditPage = ref(1);
const auditPageSize = ref(20);
const auditTotal = ref(0);
const auditSearch = ref("");

async function loadAudit() {
  auditLoading.value = true;
  try {
    const res = await logApi.audit({
      page: auditPage.value,
      pageSize: auditPageSize.value,
      search: auditSearch.value || undefined,
    });
    auditItems.value = res.items;
    auditTotal.value = res.total;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    auditLoading.value = false;
  }
}

const auditColumns: DataTableColumns<AuditLog> = [
  { title: "时间", key: "createdAt", width: 175, render: (r) => formatTime(r.createdAt) },
  { title: "操作者", key: "username", width: 130, render: (r) => r.username ?? "系统" },
  { title: "操作", key: "action", width: 170, render: (r) => actionLabel(r.action) },
  {
    title: "对象",
    key: "resource",
    width: 180,
    render: (r) =>
      r.resourceType ? `${r.resourceType}${r.resourceId ? `:${r.resourceId.slice(0, 8)}` : ""}` : "-",
  },
  { title: "IP", key: "ip", width: 140, render: (r) => r.ip ?? "-" },
  {
    title: "详情",
    key: "detail",
    render: (r) => (r.detail ? h("span", { class: "mono detail" }, JSON.stringify(r.detail)) : "-"),
  },
];

// ---------------- 节点日志 ----------------
const networks = ref<Network[]>([]);
const logNetworkId = ref<string | undefined>(undefined);
const logLevel = ref<string | null>(null);
const logSearch = ref("");
const logLoading = ref(false);
const logItems = ref<NodeLog[]>([]);
const logPage = ref(1);
const logPageSize = ref(50);
const logTotal = ref(0);
const liveOn = ref(false);

const networkIdRef = computed(() => logNetworkId.value);
const { lines: liveLines, connected } = useLogStream(networkIdRef, 800);

async function loadNodeLogs() {
  logLoading.value = true;
  try {
    const res = await logApi.nodes({
      page: logPage.value,
      pageSize: logPageSize.value,
      networkId: logNetworkId.value,
      level: logLevel.value ?? undefined,
      search: logSearch.value || undefined,
    });
    logItems.value = res.items;
    logTotal.value = res.total;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    logLoading.value = false;
  }
}

const liveFiltered = computed<LogLine[]>(() => {
  let lines = liveLines.value;
  if (logLevel.value) lines = lines.filter((l) => l.level === logLevel.value);
  if (logSearch.value)
    lines = lines.filter((l) => l.message.toLowerCase().includes(logSearch.value.toLowerCase()));
  return lines;
});

const logColumns: DataTableColumns<NodeLog> = [
  { title: "时间", key: "createdAt", width: 175, render: (r) => formatTime(r.createdAt) },
  {
    title: "级别",
    key: "level",
    width: 90,
    render: (r) =>
      h(
        "span",
        { class: `lv lv-${r.level}` },
        r.level.toUpperCase(),
      ),
  },
  { title: "网络", key: "networkId", width: 120, render: (r) => r.networkId?.slice(0, 10) ?? "-" },
  {
    title: "日志内容",
    key: "message",
    render: (r) => h("span", { class: "mono", style: "white-space: pre-wrap" }, r.message),
  },
];

function clearLogs() {
  dialog.warning({
    title: "清空节点日志",
    content: logNetworkId.value ? "确定清空当前网络的运行日志？" : "确定清空全部节点运行日志？",
    positiveText: "清空",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await logApi.clearNodes(logNetworkId.value);
        message.success("已清空");
        await loadNodeLogs();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

const levelOptions = [
  { label: "全部级别", value: "" },
  { label: "TRACE", value: "trace" },
  { label: "DEBUG", value: "debug" },
  { label: "INFO", value: "info" },
  { label: "WARN", value: "warn" },
  { label: "ERROR", value: "error" },
];

watch([logNetworkId, logLevel, logSearch], () => {
  logPage.value = 1;
  loadNodeLogs();
});

onMounted(async () => {
  await loadAudit();
  try {
    networks.value = await networkApi.list();
  } catch {
    /* ignore */
  }
  await loadNodeLogs();
});
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h1 class="page-title">日志中心</h1>
        <div class="page-subtitle">控制台操作审计与 EasyTier 节点运行日志</div>
      </div>
    </div>

    <n-card size="small">
      <n-tabs v-model:value="activeTab" type="line" animated>
        <!-- 审计日志 -->
        <n-tab-pane name="audit" tab="控制台操作日志">
          <div class="toolbar">
            <n-input
              v-model:value="auditSearch"
              placeholder="搜索操作 / 对象 / 详情"
              size="small"
              clearable
              style="width: 260px"
              @keydown.enter="loadAudit"
            >
              <template #prefix><n-icon :component="SearchOutline" /></template>
            </n-input>
            <n-button size="small" type="primary" @click="loadAudit">查询</n-button>
            <n-button size="small" @click="loadAudit">
              <template #icon><n-icon :component="RefreshOutline" /></template>
              刷新
            </n-button>
          </div>
          <n-data-table
            :columns="auditColumns"
            :data="auditItems"
            :loading="auditLoading"
            size="small"
            :row-key="(r: AuditLog) => r.id"
            remote
          />
          <div class="pager">
            <n-pagination
              v-model:page="auditPage"
              v-model:page-size="auditPageSize"
              :item-count="auditTotal"
              :page-sizes="[10, 20, 50, 100]"
              show-size-picker
              @update:page="loadAudit"
              @update:page-size="() => { auditPage = 1; loadAudit(); }"
            />
          </div>
        </n-tab-pane>

        <!-- 节点运行日志 -->
        <n-tab-pane name="runtime" tab="节点运行日志">
          <div class="toolbar">
            <n-select
              v-model:value="logNetworkId"
              :options="networks.map((n) => ({ label: n.name, value: n.id }))"
              placeholder="全部网络"
              clearable
              size="small"
              style="width: 170px"
            />
            <n-select
              v-model:value="logLevel"
              :options="levelOptions"
              placeholder="全部级别"
              clearable
              size="small"
              style="width: 130px"
            />
            <n-input
              v-model:value="logSearch"
              placeholder="搜索日志内容"
              size="small"
              clearable
              style="width: 220px"
            >
              <template #prefix><n-icon :component="SearchOutline" /></template>
            </n-input>
            <div class="spacer" />
            <n-tag :type="connected ? 'success' : 'default'" size="small" :bordered="false">
              {{ connected ? "实时流已连接" : "实时流未连接" }}
            </n-tag>
            <n-switch v-model:value="liveOn" size="small">
              <template #checked>实时</template>
              <template #unchecked>历史</template>
            </n-switch>
            <n-button size="small" @click="loadNodeLogs">
              <template #icon><n-icon :component="RefreshOutline" /></template>
            </n-button>
            <n-button size="small" type="error" quaternary @click="clearLogs">
              <template #icon><n-icon :component="TrashOutline" /></template>
            </n-button>
          </div>

          <LogViewer v-if="liveOn" :lines="liveFiltered" :height="520" :show-filter="false" />
          <template v-else>
            <n-data-table
              :columns="logColumns"
              :data="logItems"
              :loading="logLoading"
              size="small"
              :row-key="(r: NodeLog) => r.id"
              :scroll-x="900"
            />
            <div class="pager">
              <n-pagination
                v-model:page="logPage"
                v-model:page-size="logPageSize"
                :item-count="logTotal"
                :page-sizes="[20, 50, 100, 200]"
                show-size-picker
                @update:page="loadNodeLogs"
                @update:page-size="() => { logPage = 1; loadNodeLogs(); }"
              />
            </div>
          </template>
        </n-tab-pane>
      </n-tabs>
    </n-card>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}
.spacer {
  flex: 1;
}
.pager {
  display: flex;
  justify-content: flex-end;
  margin-top: 14px;
}
.detail {
  font-size: 12px;
  opacity: 0.75;
}
.lv {
  font-weight: 600;
  font-size: 12px;
}
.lv-error {
  color: #e5484d;
}
.lv-warn {
  color: #f5a524;
}
.lv-info {
  color: #2563eb;
}
.lv-debug {
  color: #8b5cf6;
}
</style>
