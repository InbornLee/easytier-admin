<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { FormInst, FormRules } from "naive-ui";
import { nodeApi } from "@/api";
import { extractError } from "@/api/client";
import { message } from "@/utils/feedback";
import type { Network, NodeItem } from "@/types";

const props = defineProps<{
  show: boolean;
  networks: Network[];
  networkId?: string;
  node?: NodeItem | null;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "saved", payload: { node: NodeItem; credentialSecret?: string }): void;
}>();

const { t } = useI18n();

const visible = computed({
  get: () => props.show,
  set: (v) => emit("update:show", v),
});

const formRef = ref<FormInst | null>(null);
const submitting = ref(false);

const DEFAULT_LISTENERS = [
  "tcp://0.0.0.0:11010",
  "udp://0.0.0.0:11010",
];

const model = reactive({
  networkId: "",
  name: "",
  hostname: "",
  autoIp: true,
  ipv4: "",
  description: "",
  listeners: [...DEFAULT_LISTENERS] as string[],
  proxyNetworks: [] as string[],
  ttlSeconds: 7 * 24 * 3600,
  allowRelay: false,
  reusable: true,
  groups: [] as string[],
  allowedProxyCidrs: [] as string[],
});

const ttlOptions = computed(() => [
  { label: t("nodes.ttl1Day"), value: 86400 },
  { label: t("nodes.ttl7Days"), value: 7 * 86400 },
  { label: t("nodes.ttl30Days"), value: 30 * 86400 },
  { label: t("nodes.ttl90Days"), value: 90 * 86400 },
  { label: t("nodes.ttl365Days"), value: 365 * 86400 },
]);

const rules = computed<FormRules>(() => ({
  networkId: [
    { required: true, message: t("nodes.validationNetwork"), trigger: ["change", "blur"] },
  ],
  name: [
    { required: true, message: t("nodes.validationNodeName"), trigger: ["blur", "input"] },
  ],
}));

watch(
  () => props.show,
  (v) => {
    if (!v) return;
    if (props.node) {
      Object.assign(model, {
        networkId: props.node.networkId,
        name: props.node.name,
        hostname: props.node.hostname,
        autoIp: !props.node.ipv4,
        ipv4: props.node.ipv4 ?? "",
        description: props.node.description ?? "",
        listeners:
          props.node.listeners && props.node.listeners.length > 0
            ? [...props.node.listeners]
            : [...DEFAULT_LISTENERS],
        proxyNetworks: [...props.node.proxyNetworks],
      });
    } else {
      Object.assign(model, {
        networkId: props.networkId ?? props.networks[0]?.id ?? "",
        name: "",
        hostname: "",
        autoIp: true,
        ipv4: "",
        description: "",
        listeners: [...DEFAULT_LISTENERS],
        proxyNetworks: [],
        ttlSeconds: 7 * 86400,
        allowRelay: false,
        reusable: true,
        groups: [],
        allowedProxyCidrs: [],
      });
    }
  },
);

async function submit() {
  try {
    await formRef.value?.validate();
  } catch {
    return;
  }
  submitting.value = true;
  try {
    if (props.node) {
      await nodeApi.update(props.node.id, {
        name: model.name,
        hostname: model.hostname || undefined,
        ipv4: model.autoIp ? null : model.ipv4,
        description: model.description,
        listeners: model.listeners.filter(Boolean),
        proxyNetworks: model.proxyNetworks,
      });
      message.success(t("nodes.updated"));
      emit("saved", { node: props.node });
    } else {
      const result = await nodeApi.create({
        networkId: model.networkId,
        name: model.name,
        hostname: model.hostname || undefined,
        ipv4: model.autoIp ? undefined : model.ipv4,
        description: model.description,
        listeners: model.listeners.filter(Boolean),
        proxyNetworks: model.proxyNetworks,
        issueCredential: true,
        ttlSeconds: model.ttlSeconds,
        allowRelay: model.allowRelay,
        reusable: model.reusable,
        groups: model.groups,
        allowedProxyCidrs: model.allowedProxyCidrs,
      });
      message.success(t("nodes.created"));
      emit("saved", { node: result.node, credentialSecret: result.credentialSecret });
    }
    visible.value = false;
  } catch (err) {
    message.error(extractError(err));
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <n-modal
    v-model:show="visible"
    preset="card"
    :title="node ? t('nodes.editTitle') : t('nodes.createTitle')"
    style="width: 640px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-form ref="formRef" :model="model" :rules="rules" label-placement="top">
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi v-if="!node" :label="t('nodes.network')" path="networkId">
          <n-select
            v-model:value="model.networkId"
            :options="networks.map((n) => ({ label: n.name, value: n.id }))"
            :placeholder="t('nodes.selectNetwork')"
            :disabled="!!networkId"
          />
        </n-form-item-gi>
        <n-form-item-gi :label="t('nodes.nodeName')" path="name">
          <n-input v-model:value="model.name" :placeholder="t('nodes.nodeNamePlaceholder')" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('nodes.hostname')">
          <n-input v-model:value="model.hostname" :placeholder="t('nodes.hostnamePlaceholder')" />
        </n-form-item-gi>
      </n-grid>

      <n-form-item :label="t('nodes.virtualIp')">
        <div class="ip-row">
          <n-switch v-model:value="model.autoIp">
            <template #checked>{{ t("nodes.autoIp") }}</template>
            <template #unchecked>{{ t("nodes.manualIp") }}</template>
          </n-switch>
          <n-input
            v-if="!model.autoIp"
            v-model:value="model.ipv4"
            :placeholder="t('nodes.ipPlaceholder')"
            style="width: 220px"
          />
        </div>
      </n-form-item>

      <n-form-item :label="t('nodes.listeners')">
        <div class="stack">
          <n-dynamic-tags v-model:value="model.listeners" />
          <n-button size="tiny" quaternary @click="model.listeners = [...DEFAULT_LISTENERS]">
            {{ t("nodes.restoreDefaults") }}
          </n-button>
          <span class="hint">
            {{ t("nodes.listenersHint") }}
          </span>
        </div>
      </n-form-item>

      <n-form-item :label="t('nodes.proxyNetworks')">
        <n-dynamic-tags v-model:value="model.proxyNetworks" />
      </n-form-item>

      <n-form-item :label="t('nodes.description')">
        <n-input
          v-model:value="model.description"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 3 }"
          :placeholder="t('common.optional')"
        />
      </n-form-item>

      <template v-if="!node">
        <n-divider title-placement="left" style="margin: 4px 0 12px">
          {{ t("nodes.credentialSection") }}
        </n-divider>
        <n-grid :cols="2" :x-gap="14">
          <n-form-item-gi :label="t('nodes.credentialTtl')">
            <n-select v-model:value="model.ttlSeconds" :options="ttlOptions" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('nodes.allowRelay')">
            <n-switch v-model:value="model.allowRelay" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('nodes.reusable')">
            <n-switch v-model:value="model.reusable" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('nodes.aclGroups')">
            <n-dynamic-tags v-model:value="model.groups" size="small" />
          </n-form-item-gi>
        </n-grid>
        <n-form-item :label="t('nodes.allowedCidrs')">
          <n-dynamic-tags v-model:value="model.allowedProxyCidrs" size="small" />
        </n-form-item>
      </template>
    </n-form>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">{{ t("common.cancel") }}</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">
          {{ node ? t("common.save") : t("nodes.createAndIssue") }}
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.hint {
  font-size: 12px;
  opacity: 0.6;
}
.stack {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}
.ip-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
