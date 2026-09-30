<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import {
  GitNetworkOutline,
  ServerOutline,
  KeyOutline,
  WarningOutline,
  AddOutline,
  RefreshOutline,
} from "@vicons/ionicons5";
import StatCard from "@/components/StatCard.vue";
import TrafficChart from "@/components/TrafficChart.vue";
import { dashboardApi, systemApi } from "@/api";
import { extractError } from "@/api/client";
import { message } from "@/utils/feedback";
import { actionLabel, formatRelative, formatTime } from "@/utils/format";
import type { DashboardSummary, SystemBinaries, TrafficPoint } from "@/types";

const router = useRouter();
const loading = ref(true);
const summary = ref<DashboardSummary | null>(null);
const traffic = ref<TrafficPoint[]>([]);
const binaries = ref<SystemBinaries | null>(null);

async function load() {
  loading.value = true;
  try {
    const [s, t, b] = await Promise.all([
      dashboardApi.summary(),
      dashboardApi.traffic(undefined, 1),
      systemApi.binaries(),
    ]);
    summary.value = s;
    traffic.value = t;
    binaries.value = b;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h1 class="page-title">概览</h1>
        <div class="page-subtitle">EasyTier 网络运行状态一览</div>
      </div>
      <div class="header-actions">
        <n-button size="small" @click="load">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          刷新
        </n-button>
        <n-button size="small" type="primary" @click="router.push({ name: 'networks' })">
          <template #icon><n-icon :component="AddOutline" /></template>
          新建网络
        </n-button>
      </div>
    </div>

    <n-alert
      v-if="binaries && (!binaries.core.available || !binaries.cli.available)"
      type="warning"
      class="mb"
      title="EasyTier 二进制不可用"
    >
      <div>
        控制台未检测到 <span class="mono">easytier-core</span> 或
        <span class="mono">easytier-cli</span>，网络实例将无法启动、状态与凭据功能不可用。
        请在「系统设置」中配置正确的二进制路径。
      </div>
    </n-alert>

    <div class="stat-grid">
      <StatCard
        label="网络总数"
        :value="summary?.networks.total ?? 0"
        :hint="`${summary?.networks.running ?? 0} 个运行中 · ${summary?.networks.error ?? 0} 个异常`"
        color="#2563eb"
        :icon="GitNetworkOutline"
        :loading="loading"
      />
      <StatCard
        label="节点总数"
        :value="summary?.nodes.total ?? 0"
        :hint="`${summary?.nodes.online ?? 0} 在线 · ${summary?.nodes.offline ?? 0} 离线`"
        color="#0d9488"
        :icon="ServerOutline"
        :loading="loading"
      />
      <StatCard
        label="有效凭据"
        :value="summary?.credentials.active ?? 0"
        :hint="`${summary?.credentials.expiringSoon ?? 0} 个 24 小时内过期`"
        color="#7c3aed"
        :icon="KeyOutline"
        :loading="loading"
      />
      <StatCard
        label="近 24h 错误日志"
        :value="summary?.recentErrors.length ?? 0"
        hint="来自节点运行日志"
        color="#e5484d"
        :icon="WarningOutline"
        :loading="loading"
      />
    </div>

    <n-grid :cols="24" :x-gap="16" :y-gap="16" class="mt">
      <n-gi :span="16">
        <n-card title="全局流量（近 1 小时）" size="small">
          <TrafficChart :series="traffic" :height="300" />
        </n-card>
      </n-gi>
      <n-gi :span="8">
        <n-card title="最近操作" size="small" class="h-full">
          <n-empty
            v-if="!summary?.recentAudit.length"
            description="暂无操作记录"
            style="margin: 30px 0"
          />
          <n-timeline v-else>
            <n-timeline-item
              v-for="item in summary.recentAudit"
              :key="item.id"
              :time="formatRelative(item.createdAt)"
              type="info"
            >
              <div class="audit-line">
                <strong>{{ actionLabel(item.action) }}</strong>
                <span class="audit-meta">
                  {{ item.username ?? "系统" }} · {{ item.resourceType ?? "" }}
                </span>
              </div>
            </n-timeline-item>
          </n-timeline>
        </n-card>
      </n-gi>
      <n-gi :span="24">
        <n-card title="最近错误日志" size="small">
          <n-empty v-if="!summary?.recentErrors.length" description="暂无错误日志" />
          <n-list v-else hoverable>
            <n-list-item v-for="item in summary.recentErrors" :key="item.id">
              <div class="err-line">
                <n-tag type="error" size="tiny" :bordered="false">ERROR</n-tag>
                <span class="mono err-msg">{{ item.message }}</span>
                <span class="err-time">{{ formatTime(item.createdAt) }}</span>
              </div>
            </n-list-item>
          </n-list>
        </n-card>
      </n-gi>
    </n-grid>
  </div>
</template>

<style scoped>
.header-actions {
  display: flex;
  gap: 8px;
}
.mb {
  margin-bottom: 16px;
}
.mt {
  margin-top: 16px;
}
.h-full {
  height: 100%;
}
.audit-line {
  display: flex;
  flex-direction: column;
}
.audit-meta {
  font-size: 12px;
  opacity: 0.6;
}
.err-line {
  display: flex;
  align-items: center;
  gap: 10px;
}
.err-msg {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.err-time {
  font-size: 12px;
  opacity: 0.55;
  flex-shrink: 0;
}
</style>
