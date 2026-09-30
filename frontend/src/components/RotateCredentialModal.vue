<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { nodeApi } from "@/api";
import { extractError } from "@/api/client";
import { message } from "@/utils/feedback";
import type { Credential, NodeItem } from "@/types";

const props = defineProps<{
  show: boolean;
  node?: NodeItem | null;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "rotated", payload: { node: NodeItem; credential: Credential; secret: string }): void;
}>();

const { t } = useI18n();

const visible = computed({
  get: () => props.show,
  set: (v) => emit("update:show", v),
});

const submitting = ref(false);

const model = reactive({
  ttlSeconds: 7 * 24 * 3600,
  allowRelay: false,
  reusable: true,
  groups: [] as string[],
  allowedProxyCidrs: [] as string[],
});

const ttlOptions = computed(() => [
  { label: t("nodes.ttl1Hour"), value: 3600 },
  { label: t("nodes.ttl1Day"), value: 86400 },
  { label: t("nodes.ttl7Days"), value: 7 * 86400 },
  { label: t("nodes.ttl30Days"), value: 30 * 86400 },
  { label: t("nodes.ttl90Days"), value: 90 * 86400 },
  { label: t("nodes.ttl365Days"), value: 365 * 86400 },
]);

watch(
  () => props.show,
  (v) => {
    if (!v) return;
    Object.assign(model, {
      ttlSeconds: 7 * 24 * 3600,
      allowRelay: false,
      reusable: true,
      groups: [],
      allowedProxyCidrs: [],
    });
  },
);

async function submit() {
  if (!props.node) return;
  submitting.value = true;
  try {
    const result = await nodeApi.rotateCredential(props.node.id, {
      ttlSeconds: model.ttlSeconds,
      allowRelay: model.allowRelay,
      reusable: model.reusable,
      groups: model.groups,
      allowedProxyCidrs: model.allowedProxyCidrs,
    });
    message.success(t("nodes.rotated"));
    emit("rotated", {
      node: result.node,
      credential: result.credential,
      secret: result.credentialSecret,
    });
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
    :title="t('nodes.rotateTitle')"
    style="width: 600px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-alert type="warning" :show-icon="true" class="mb">
      {{ t("nodes.rotateWarningPrefix") }}<strong>{{ t("nodes.rotateWarningStrong") }}</strong>{{ t("nodes.rotateWarningSuffix") }}
      <template v-if="node">{{ t("nodes.targetNodePrefix") }}<strong>{{ node.name }}</strong>{{ t("nodes.targetNodeSuffix") }}</template>
    </n-alert>
    <n-form label-placement="top">
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
    </n-form>
    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">{{ t("common.cancel") }}</n-button>
        <n-button type="warning" :loading="submitting" @click="submit">
          {{ t("nodes.confirmRotate") }}
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.mb {
  margin-bottom: 12px;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
