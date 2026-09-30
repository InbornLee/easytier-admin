<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { FormInst } from "naive-ui";
import { credentialApi } from "@/api";
import { extractError } from "@/api/client";
import { message } from "@/utils/feedback";
import type { Credential, Network } from "@/types";

const props = defineProps<{
  show: boolean;
  networks: Network[];
  networkId?: string;
  title?: string;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "created", payload: { credential: Credential; secret: string }): void;
}>();

const { t } = useI18n();

const visible = computed({
  get: () => props.show,
  set: (v) => emit("update:show", v),
});

const formRef = ref<FormInst | null>(null);
const submitting = ref(false);

const model = reactive({
  networkId: "",
  ttlSeconds: 7 * 24 * 3600,
  groups: [] as string[],
  allowRelay: false,
  reusable: true,
  allowedProxyCidrs: [] as string[],
});

const ttlOptions = computed(() => [
  { label: t("credentials.ttl1Hour"), value: 3600 },
  { label: t("credentials.ttl1Day"), value: 86400 },
  { label: t("credentials.ttl7Days"), value: 7 * 86400 },
  { label: t("credentials.ttl30Days"), value: 30 * 86400 },
  { label: t("credentials.ttl90Days"), value: 90 * 86400 },
  { label: t("credentials.ttl365Days"), value: 365 * 86400 },
]);

watch(
  () => props.show,
  (v) => {
    if (!v) return;
    Object.assign(model, {
      networkId: props.networkId ?? props.networks[0]?.id ?? "",
      ttlSeconds: 7 * 24 * 3600,
      groups: [],
      allowRelay: false,
      reusable: true,
      allowedProxyCidrs: [],
    });
  },
);

async function submit() {
  if (!model.networkId) {
    message.warning(t("credentials.selectNetwork"));
    return;
  }
  submitting.value = true;
  try {
    const result = await credentialApi.create({
      networkId: model.networkId,
      ttlSeconds: model.ttlSeconds,
      groups: model.groups,
      allowRelay: model.allowRelay,
      reusable: model.reusable,
      allowedProxyCidrs: model.allowedProxyCidrs,
    });
    message.success(t("credentials.issueSuccess"));
    emit("created", result);
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
    :title="title ?? t('credentials.issue')"
    style="width: 600px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-form ref="formRef" :model="model" label-placement="top">
      <n-form-item :label="t('credentials.network')" required>
        <n-select
          v-model:value="model.networkId"
          :options="networks.map((n) => ({ label: n.name, value: n.id }))"
          :placeholder="t('credentials.selectNetworkPlaceholder')"
          :disabled="!!networkId"
        />
      </n-form-item>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi :label="t('credentials.formTtl')">
          <n-select v-model:value="model.ttlSeconds" :options="ttlOptions" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('credentials.formAllowRelay')">
          <n-switch v-model:value="model.allowRelay" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('credentials.formReusable')">
          <n-switch v-model:value="model.reusable" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('credentials.formGroups')">
          <n-dynamic-tags v-model:value="model.groups" size="small" />
        </n-form-item-gi>
      </n-grid>
      <n-form-item :label="t('credentials.formAllowedProxyCidrs')">
        <n-dynamic-tags v-model:value="model.allowedProxyCidrs" size="small" />
      </n-form-item>
      <n-alert type="info" :show-icon="true">
        {{ t("credentials.formHint") }}
      </n-alert>
    </n-form>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">{{ t("common.cancel") }}</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">{{ t("credentials.issue") }}</n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
