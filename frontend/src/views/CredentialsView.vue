<script setup lang="ts">
import { computed, h, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
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
const { t } = useI18n();
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
    title: t("credentials.revokeTitle"),
    content: t("credentials.revokeConfirm"),
    positiveText: t("credentials.revoke"),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      try {
        await credentialApi.revoke(cred.id);
        message.success(t("credentials.revoked"));
        await load();
      } catch (err) {
        message.error(extractError(err));
      }
    },
  });
}

function remove(cred: Credential) {
  dialog.error({
    title: t("credentials.deleteTitle"),
    content: t("credentials.deleteConfirm"),
    positiveText: t("common.delete"),
    negativeText: t("common.cancel"),
    onPositiveClick: async () => {
      try {
        await credentialApi.remove(cred.id);
        message.success(t("credentials.deleted"));
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
    message.info(t("credentials.noInvalid"));
    return;
  }
  dialog.warning({
    title: t("credentials.cleanupTitle"),
    content: t("credentials.cleanupConfirm", { n: invalid.length }),
    positiveText: t("credentials.cleanupAction"),
    negativeText: t("common.cancel"),
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
      message.success(t("credentials.cleaned", { n: ok }));
      await load();
    },
  });
}

const columns = computed<DataTableColumns<Credential>>(() => [
  {
    title: t("credentials.credentialId"),
    key: "credentialId",
    render: (row) => h("span", { class: "mono" }, row.credentialId),
  },
  {
    title: t("credentials.network"),
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
    width: 100,
    render: (row) => h(StatusTag, { kind: "credential", status: row.status }),
  },
  {
    title: t("credentials.remaining"),
    key: "remaining",
    width: 130,
    render: (row) =>
      row.status === "active" ? formatDuration(row.remainingSeconds) : "-",
  },
  {
    title: t("credentials.expiresAt"),
    key: "expiresAt",
    width: 170,
    render: (row) => formatTime(row.expiresAt),
  },
  {
    title: t("credentials.perms"),
    key: "perms",
    render: (row) =>
      [
        row.allowRelay ? t("credentials.relayAllowed") : t("credentials.relayDenied"),
        row.reusable ? t("credentials.reusableTag") : t("credentials.exclusive"),
        row.groups.length
          ? t("credentials.groupsPrefix", { list: row.groups.join(",") })
          : null,
        row.allowedProxyCidrs.length
          ? t("credentials.proxyPrefix", { list: row.allowedProxyCidrs.join(",") })
          : null,
      ]
        .filter(Boolean)
        .join(" · "),
  },
  {
    title: t("common.actions"),
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
              { default: () => t("credentials.revoke") },
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
        <h1 class="page-title">{{ t("credentials.title") }}</h1>
        <div class="page-subtitle">{{ t("credentials.subtitle") }}</div>
      </div>
      <div class="header-actions">
        <n-select
          v-model:value="filterNetwork"
          :options="networks.map((n) => ({ label: n.name, value: n.id }))"
          :placeholder="t('credentials.allNetworks')"
          clearable
          size="small"
          style="width: 170px"
        />
        <n-button size="small" @click="load">
          <template #icon><n-icon :component="RefreshOutline" /></template>
          {{ t("common.refresh") }}
        </n-button>
        <n-button size="small" @click="cleanupInvalid">
          <template #icon><n-icon :component="TrashOutline" /></template>
          {{ t("credentials.cleanup") }}
        </n-button>
        <n-button size="small" type="primary" @click="showCredModal = true">
          <template #icon><n-icon :component="AddOutline" /></template>
          {{ t("credentials.issue") }}
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
      :title="t('credentials.issued')"
      style="width: 560px; max-width: 94vw"
    >
      <n-alert type="warning" :show-icon="true" class="mb">
        {{ t("credentials.secretWarning") }}
      </n-alert>
      <n-descriptions v-if="secretCredential" :column="1" size="small" bordered class="mb">
        <n-descriptions-item :label="t('credentials.credentialId')">
          <span class="mono">{{ secretCredential.credentialId }}</span>
        </n-descriptions-item>
        <n-descriptions-item :label="t('credentials.expiresAt')">
          {{ formatTime(secretCredential.expiresAt) }}
        </n-descriptions-item>
        <n-descriptions-item :label="t('credentials.secretLabel')">
          <CopyText :text="secretValue" />
        </n-descriptions-item>
      </n-descriptions>
      <div class="code-block mono">{{ secretValue }}</div>
      <template #footer>
        <div class="modal-footer">
          <n-button type="primary" @click="secretModal = false">{{ t("credentials.savedSecret") }}</n-button>
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
