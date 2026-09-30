<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
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

const ttlOptions = [
  { label: "1 小时", value: 3600 },
  { label: "1 天", value: 86400 },
  { label: "7 天", value: 7 * 86400 },
  { label: "30 天", value: 30 * 86400 },
  { label: "90 天", value: 90 * 86400 },
  { label: "365 天", value: 365 * 86400 },
];

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
    message.warning("请选择网络");
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
    message.success("凭据签发成功");
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
    :title="title ?? '签发临时凭据'"
    style="width: 600px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-form ref="formRef" :model="model" label-placement="top">
      <n-form-item label="所属网络" required>
        <n-select
          v-model:value="model.networkId"
          :options="networks.map((n) => ({ label: n.name, value: n.id }))"
          placeholder="选择网络"
          :disabled="!!networkId"
        />
      </n-form-item>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi label="有效期">
          <n-select v-model:value="model.ttlSeconds" :options="ttlOptions" />
        </n-form-item-gi>
        <n-form-item-gi label="允许中继流量">
          <n-switch v-model:value="model.allowRelay" />
        </n-form-item-gi>
        <n-form-item-gi label="允许多设备复用">
          <n-switch v-model:value="model.reusable" />
        </n-form-item-gi>
        <n-form-item-gi label="ACL 分组">
          <n-dynamic-tags v-model:value="model.groups" size="small" />
        </n-form-item-gi>
      </n-grid>
      <n-form-item label="允许代理的网段 (allowed proxy cidrs)">
        <n-dynamic-tags v-model:value="model.allowedProxyCidrs" size="small" />
      </n-form-item>
      <n-alert type="info" :show-icon="true">
        凭据用于临时设备在不接触网络主密钥的情况下接入网络，支持到期自动失效与手动撤销。
      </n-alert>
    </n-form>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">取消</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">签发临时凭据</n-button>
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
