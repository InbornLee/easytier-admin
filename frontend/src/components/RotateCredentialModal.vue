<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
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
    message.success("凭据已更换，请查看新的接入信息");
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
    title="更换节点凭据"
    style="width: 600px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-alert type="warning" :show-icon="true" class="mb">
      更换后将<strong>立即撤销原凭据</strong>，使用原凭据的设备会掉线，需使用新凭据重新接入。
      <template v-if="node">目标节点：<strong>{{ node.name }}</strong>。</template>
    </n-alert>
    <n-form label-placement="top">
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi label="凭据有效期">
          <n-select v-model:value="model.ttlSeconds" :options="ttlOptions" />
        </n-form-item-gi>
        <n-form-item-gi label="允许中继流量">
          <n-switch v-model:value="model.allowRelay" />
        </n-form-item-gi>
        <n-form-item-gi label="允许多设备复用">
          <n-switch v-model:value="model.reusable" />
        </n-form-item-gi>
        <n-form-item-gi label="ACL 分组 (groups)">
          <n-dynamic-tags v-model:value="model.groups" size="small" />
        </n-form-item-gi>
      </n-grid>
      <n-form-item label="允许代理的网段 (allowed proxy cidrs)">
        <n-dynamic-tags v-model:value="model.allowedProxyCidrs" size="small" />
      </n-form-item>
    </n-form>
    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">取消</n-button>
        <n-button type="warning" :loading="submitting" @click="submit">确认更换</n-button>
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
