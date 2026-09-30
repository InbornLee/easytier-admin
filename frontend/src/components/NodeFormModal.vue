<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
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

const ttlOptions = [
  { label: "1 天", value: 86400 },
  { label: "7 天", value: 7 * 86400 },
  { label: "30 天", value: 30 * 86400 },
  { label: "90 天", value: 90 * 86400 },
  { label: "365 天", value: 365 * 86400 },
];

const rules: FormRules = {
  networkId: [{ required: true, message: "请选择所属网络", trigger: ["change", "blur"] }],
  name: [{ required: true, message: "请输入节点名称", trigger: ["blur", "input"] }],
};

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
      message.success("节点已更新");
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
      message.success("节点已创建，并已签发接入凭据");
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
    :title="node ? '编辑节点' : '新建节点'"
    style="width: 640px; max-width: 94vw"
    :mask-closable="false"
  >
    <n-form ref="formRef" :model="model" :rules="rules" label-placement="top">
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi v-if="!node" label="所属网络" path="networkId">
          <n-select
            v-model:value="model.networkId"
            :options="networks.map((n) => ({ label: n.name, value: n.id }))"
            placeholder="选择网络"
            :disabled="!!networkId"
          />
        </n-form-item-gi>
        <n-form-item-gi label="节点名称" path="name">
          <n-input v-model:value="model.name" placeholder="例如：家中 NAS" />
        </n-form-item-gi>
        <n-form-item-gi label="主机名 (hostname)">
          <n-input v-model:value="model.hostname" placeholder="留空自动生成" />
        </n-form-item-gi>
      </n-grid>

      <n-form-item label="虚拟 IP 分配">
        <div class="ip-row">
          <n-switch v-model:value="model.autoIp">
            <template #checked>自动分配</template>
            <template #unchecked>手动指定</template>
          </n-switch>
          <n-input
            v-if="!model.autoIp"
            v-model:value="model.ipv4"
            placeholder="例如：10.126.126.10"
            style="width: 220px"
          />
        </div>
      </n-form-item>

      <n-form-item label="监听器 (listeners)">
        <div class="stack">
          <n-dynamic-tags v-model:value="model.listeners" />
          <n-button size="tiny" quaternary @click="model.listeners = [...DEFAULT_LISTENERS]">
            恢复默认 (tcp/udp 11010)
          </n-button>
          <span class="hint">
            节点本机监听的地址；默认 tcp/udp 11010。若端口被占用请修改，或留空使用默认。
          </span>
        </div>
      </n-form-item>

      <n-form-item label="子网代理 (proxy networks)">
        <n-dynamic-tags v-model:value="model.proxyNetworks" />
      </n-form-item>

      <n-form-item label="描述">
        <n-input
          v-model:value="model.description"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 3 }"
          placeholder="可选"
        />
      </n-form-item>

      <template v-if="!node">
        <n-divider title-placement="left" style="margin: 4px 0 12px">
          接入凭据
        </n-divider>
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
      </template>
    </n-form>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">取消</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">
          {{ node ? "保存" : "创建并签发凭据" }}
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
