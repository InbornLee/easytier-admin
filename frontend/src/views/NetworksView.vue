<script setup lang="ts">
import { computed, h, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import type { DataTableColumns } from "naive-ui";
import { NButton, NIcon } from "naive-ui";
import {
  AddOutline,
  PlayOutline,
  StopOutline,
  RefreshOutline,
  TrashOutline,
  CreateOutline,
  EyeOutline,
  ShareSocialOutline,
} from "@vicons/ionicons5";
import StatusTag from "@/components/StatusTag.vue";
import NetworkFormModal from "@/components/NetworkFormModal.vue";
import NetworkShareModal from "@/components/NetworkShareModal.vue";
import { networkApi } from "@/api";
import { extractError } from "@/api/client";
import { message, dialog } from "@/utils/feedback";
import { formatTime } from "@/utils/format";
import { useAuthStore } from "@/stores/auth";
import type { Network } from "@/types";

const router = useRouter();
const auth = useAuthStore();
const { t } = useI18n();
const loading = ref(false);
const networks = ref<Network[]>([]);
const actionLoading = ref<string | null>(null);

const showModal = ref(false);
const editing = ref<Network | null>(null);
const showShareModal = ref(false);
const sharing = ref<Network | null>(null);

function canManage(row: Network): boolean {
  return (
    auth.user?.role === "admin" || row.access === "owner" || row.access === "manage"
  );
}

function accessLabel(row: Network): string {
  if (auth.user?.role === "admin") return t("role.admin");
  switch (row.access) {
    case "owner":
      return t("networks.accessOwner");
    case "manage":
      return t("networks.accessManage");
    default:
      return t("networks.accessReadonly");
  }
}

function openShare(row: Network) {
  sharing.value = row;
  showShareModal.value = true;
}

function openCreate() {
  editing.value = null;
  showModal.value = true;
}

function openEdit(row: Network) {
  editing.value = row;
  showModal.value = true;
}

function openDetail(row: Network) {
  router.push({ name: "network-detail", params: { id: row.id } });
}

const columns = computed<DataTableColumns<Network>>(() => [
  {
    title: t("networks.colNetwork"),
    key: "name",
    render: (row) =>
      h("div", { class: "cell-network" }, [
        h("div", { class: "cell-name" }, row.name),
        h(
          "div",
          { class: "cell-sub" },
          t("networks.identifier", { networkName: row.networkName, ipv4: row.ipv4 }),
        ),
      ]),
  },
  {
    title: t("common.status"),
    key: "status",
    width: 100,
    render: (row) => h(StatusTag, { kind: "network", status: row.status }),
  },
  {
    title: t("networks.colListenPort"),
    key: "listenPort",
    width: 120,
    render: (row) => h("span", { class: "mono" }, `${row.listenPort}/${row.rpcPort}`),
  },
  {
    title: t("networks.colSecureMode"),
    key: "secureMode",
    width: 100,
    render: (row) => (row.secureMode ? t("common.enabled") : t("common.disabled")),
  },
  {
    title: t("networks.colAutoStart"),
    key: "autoStart",
    width: 90,
    render: (row) => (row.autoStart ? t("common.yes") : t("common.no")),
  },
  {
    title: t("networks.colAccess"),
    key: "access",
    width: 100,
    render: (row) => accessLabel(row),
  },
  {
    title: t("networks.colCreatedAt"),
    key: "createdAt",
    width: 170,
    render: (row) => formatTime(row.createdAt),
  },
  {
    title: t("common.actions"),
    key: "actions",
    width: 320,
    fixed: "right",
    render: (row) => {
      const manage = canManage(row);
      const stop = (e: MouseEvent) => e.stopPropagation();
      const actions = [
        h(
          NButton,
          {
            size: "tiny",
            quaternary: true,
            onClick: (e: MouseEvent) => {
              stop(e);
              openDetail(row);
            },
          },
          { icon: () => h(NIcon, { component: EyeOutline }), default: () => t("networks.viewNodes") },
        ),
      ];
      if (manage) {
        actions.push(
          row.status === "running"
            ? h(
                NButton,
                {
                  size: "tiny",
                  quaternary: true,
                  type: "warning",
                  loading: actionLoading.value === `${row.id}:stop`,
                  onClick: (e: MouseEvent) => {
                    stop(e);
                    doAction(row, "stop");
                  },
                },
                { icon: () => h(NIcon, { component: StopOutline }) },
              )
            : h(
                NButton,
                {
                  size: "tiny",
                  quaternary: true,
                  type: "success",
                  loading: actionLoading.value === `${row.id}:start`,
                  onClick: (e: MouseEvent) => {
                    stop(e);
                    doAction(row, "start");
                  },
                },
                { icon: () => h(NIcon, { component: PlayOutline }) },
              ),
        );
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              loading: actionLoading.value === `${row.id}:restart`,
              onClick: (e: MouseEvent) => {
                stop(e);
                doAction(row, "restart");
              },
            },
            { icon: () => h(NIcon, { component: RefreshOutline }) },
          ),
        );
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              onClick: (e: MouseEvent) => {
                stop(e);
                openShare(row);
              },
            },
            {
              icon: () => h(NIcon, { component: ShareSocialOutline }),
              default: () => t("networks.share"),
            },
          ),
        );
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              onClick: (e: MouseEvent) => {
                stop(e);
                openEdit(row);
              },
            },
            { icon: () => h(NIcon, { component: CreateOutline }) },
          ),
        );
        actions.push(
          h(
            NButton,
            {
              size: "tiny",
              quaternary: true,
              type: "error",
              onClick: (e: MouseEvent) => {
                stop(e);
                removeNetwork(row);
              },
            },
            { icon: () => h(NIcon, { component: TrashOutline }) },
          ),
        );
      }
      return h("div", { class: "row-actions" }, actions);
    },
  },
]);

function rowProps(row: Network) {
  return {
    style: "cursor: pointer;",
    onClick: () => openDetail(row),
  };
}

async function load() {
  loading.value = true;
  try {
    networks.value = await networkApi.list();
  } catch (err) {
    message.error(extractError(err));
  } finally {
    loading.value = false;
  }
}

async function doAction(row: Network, action: "start" | "stop" | "restart") {
  actionLoading.value = `${row.id}:${action}`;
  try {
    await networkApi[action](row.id);
    message.success(
      action === "start"
        ? t("networks.started")
        : action === "stop"
          ? t("networks.stopped")
          : t("networks.restarted"),
    );
    await load();
  } catch (err) {
    message.error(extractError(err));
  } finally {
    actionLoading.value = null;
  }
}

function removeNetwork(row: Network) {
  dialog.error({
    title: t("action.networkDelete"),
    content: t("networks.deleteConfirm", { name: row.name }),
    positiveText: t("common.delete"),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      try {
        await networkApi.remove(row.id);
        message.success(t("networks.deleted"));
        await load();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

async function onSaved() {
  await load();
}

async function onShareTransferred() {
  await load();
  if (sharing.value) {
    const updated = networks.value.find((n) => n.id === sharing.value!.id);
    if (updated) sharing.value = updated;
  }
}

onMounted(load);
</script>

<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h1 class="page-title">{{ t("nav.networks") }}</h1>
        <div class="page-subtitle">{{ t("networks.subtitle") }}</div>
      </div>
      <div class="header-actions">
        <n-button size="small" @click="load">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          {{ t("common.refresh") }}
        </n-button>
        <n-button size="small" type="primary" @click="openCreate">
          <template #icon><n-icon :component="AddOutline" /></template>
          {{ t("networks.newNetwork") }}
        </n-button>
      </div>
    </div>

    <n-card size="small">
      <n-data-table
        :columns="columns"
        :data="networks"
        :loading="loading"
        :row-key="(row: Network) => row.id"
        :row-props="rowProps"
        :scroll-x="1100"
        size="small"
      />
    </n-card>

    <NetworkFormModal v-model:show="showModal" :network="editing" @saved="onSaved" />
    <NetworkShareModal
      v-model:show="showShareModal"
      :network="sharing"
      @transferred="onShareTransferred"
    />
  </div>
</template>

<style scoped>
.header-actions {
  display: flex;
  gap: 8px;
}
.cell-name {
  font-weight: 600;
}
.cell-sub {
  font-size: 12px;
  opacity: 0.55;
}
.row-actions {
  display: flex;
  gap: 2px;
}
</style>
