<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
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
const { t } = useI18n();
const loading = ref(true);
const summary = ref<DashboardSummary | null>(null);
const traffic = ref<TrafficPoint[]>([]);
const binaries = ref<SystemBinaries | null>(null);

async function load() {
  loading.value = true;
  try {
    const [s, tr, b] = await Promise.all([
      dashboardApi.summary(),
      dashboardApi.traffic(undefined, 1),
      systemApi.binaries(),
    ]);
    summary.value = s;
    traffic.value = tr;
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
        <h1 class="page-title">{{ t("dashboard.title") }}</h1>
        <div class="page-subtitle">{{ t("dashboard.subtitle") }}</div>
      </div>
      <div class="header-actions">
        <n-button size="small" @click="load">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          {{ t("common.refresh") }}
        </n-button>
        <n-button size="small" type="primary" @click="router.push({ name: 'networks' })">
          <template #icon><n-icon :component="AddOutline" /></template>
          {{ t("dashboard.newNetwork") }}
        </n-button>
      </div>
    </div>

    <n-alert
      v-if="binaries && (!binaries.core.available || !binaries.cli.available)"
      type="warning"
      class="mb"
      :title="t('dashboard.binaryWarningTitle')"
    >
      <div v-html="t('dashboard.binaryWarningBody')" />
    </n-alert>

    <div class="stat-grid">
      <StatCard
        :label="t('dashboard.statNetworks')"
        :value="summary?.networks.total ?? 0"
        :hint="
          t('dashboard.statNetworksHint', {
            running: summary?.networks.running ?? 0,
            error: summary?.networks.error ?? 0,
          })
        "
        color="#2563eb"
        :icon="GitNetworkOutline"
        :loading="loading"
      />
      <StatCard
        :label="t('dashboard.statNodes')"
        :value="summary?.nodes.total ?? 0"
        :hint="
          t('dashboard.statNodesHint', {
            online: summary?.nodes.online ?? 0,
            offline: summary?.nodes.offline ?? 0,
          })
        "
        color="#0d9488"
        :icon="ServerOutline"
        :loading="loading"
      />
      <StatCard
        :label="t('dashboard.statCredentials')"
        :value="summary?.credentials.active ?? 0"
        :hint="t('dashboard.statCredentialsHint', { soon: summary?.credentials.expiringSoon ?? 0 })"
        color="#7c3aed"
        :icon="KeyOutline"
        :loading="loading"
      />
      <StatCard
        :label="t('dashboard.statErrors')"
        :value="summary?.recentErrors.length ?? 0"
        :hint="t('dashboard.statErrorsHint')"
        color="#e5484d"
        :icon="WarningOutline"
        :loading="loading"
      />
    </div>

    <n-grid :cols="24" :x-gap="16" :y-gap="16" class="mt">
      <n-gi :span="16">
        <n-card :title="t('dashboard.trafficTitle')" size="small">
          <TrafficChart :series="traffic" :height="300" />
        </n-card>
      </n-gi>
      <n-gi :span="8">
        <n-card :title="t('dashboard.recentAuditTitle')" size="small" class="h-full">
          <n-empty
            v-if="!summary?.recentAudit.length"
            :description="t('dashboard.recentAuditEmpty')"
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
                  {{ item.username ?? t("common.system") }} · {{ item.resourceType ?? "" }}
                </span>
              </div>
            </n-timeline-item>
          </n-timeline>
        </n-card>
      </n-gi>
      <n-gi :span="24">
        <n-card :title="t('dashboard.recentErrorsTitle')" size="small">
          <n-empty v-if="!summary?.recentErrors.length" :description="t('dashboard.recentErrorsEmpty')" />
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
