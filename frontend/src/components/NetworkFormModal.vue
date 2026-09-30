<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { FormInst, FormRules } from "naive-ui";
import { networkApi, type NetworkPayload } from "@/api";
import { extractError } from "@/api/client";
import { message } from "@/utils/feedback";
import type { EasyTierFlags, Network } from "@/types";

const props = defineProps<{
  show: boolean;
  network?: Network | null;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "saved", network: Network): void;
}>();

const visible = computed({
  get: () => props.show,
  set: (v) => emit("update:show", v),
});

const { t } = useI18n();
const formRef = ref<FormInst | null>(null);
const submitting = ref(false);
const advanced = ref(false);

const DEFAULT_FLAGS: EasyTierFlags = {
  default_protocol: "tcp",
  dev_name: "",
  enable_encryption: true,
  enable_ipv6: true,
  mtu: 1380,
  latency_first: false,
  enable_exit_node: false,
  no_tun: false,
  use_smoltcp: false,
  relay_network_whitelist: "*",
  disable_p2p: false,
  relay_all_peer_rpc: false,
  disable_udp_hole_punching: false,
  disable_tcp_hole_punching: false,
  private_mode: false,
};

interface FormModel {
  name: string;
  description: string;
  networkName: string;
  networkSecret: string;
  generateSecret: boolean;
  ipv4: string;
  dhcp: boolean;
  hostname: string;
  instanceName: string;
  externalNode: string;
  listeners: string[];
  mappedListeners: string[];
  peers: string[];
  listenPort: number;
  rpcPort: number;
  autoPorts: boolean;
  secureMode: boolean;
  autoStart: boolean;
  startNow: boolean;
  flags: EasyTierFlags;
}

const model = reactive<FormModel>({
  name: "",
  description: "",
  networkName: "",
  networkSecret: "",
  generateSecret: true,
  ipv4: "10.126.126.1/24",
  dhcp: false,
  hostname: "",
  instanceName: "",
  externalNode: "",
  listeners: [],
  mappedListeners: [],
  peers: [],
  listenPort: 11010,
  rpcPort: 15888,
  autoPorts: true,
  secureMode: true,
  autoStart: false,
  startNow: true,
  flags: { ...DEFAULT_FLAGS },
});

const rules = computed<FormRules>(() => ({
  name: [
    { required: true, message: t("networkForm.form.ruleNameRequired"), trigger: ["blur", "input"] },
  ],
  ipv4: [
    {
      validator: (_r, value: string) => {
        if (model.dhcp) return true;
        return /^\d{1,3}(\.\d{1,3}){3}\/\d{1,2}$/.test(value || "");
      },
      message: t("networkForm.form.ruleIpv4Format"),
      trigger: ["blur"],
    },
  ],
}));

const isEdit = computed(() => !!props.network);
const portsDisabled = computed(
  () => isEdit.value && props.network?.status === "running",
);

function defaultListeners(port: number): string[] {
  return [
    `tcp://0.0.0.0:${port}`,
    `udp://0.0.0.0:${port}`,
    `ws://0.0.0.0:${port + 1}/`,
  ];
}

// 监听器是否被用户手动改过；未手动改过时跟随“监听端口”自动同步
const listenersCustom = ref(false);

function listenersEqualDefault(list: string[]): boolean {
  const def = defaultListeners(model.listenPort || 11010);
  if (list.length !== def.length) return false;
  return [...list].sort().join("|") === [...def].sort().join("|");
}

function fillDefaultListeners() {
  model.listeners = defaultListeners(model.listenPort || 11010);
  listenersCustom.value = false;
}

// 监听端口变化时，若监听器未被手动改过，则同步重建
watch(
  () => model.listenPort,
  (port) => {
    if (isEdit.value && props.network?.status === "running") return;
    if (listenersCustom.value) return;
    model.listeners = defaultListeners(port || 11010);
  },
);

// 监听器被手动修改后不再自动覆盖（清空则恢复自动）
watch(
  () => [...model.listeners],
  (list) => {
    if (list.length === 0) {
      listenersCustom.value = false;
      return;
    }
    listenersCustom.value = !listenersEqualDefault(list);
  },
);

// 切换“自动分配 / 手动指定”时同步监听器：自动则清空（由后端生成），手动则按端口生成
watch(
  () => model.autoPorts,
  (auto) => {
    if (isEdit.value) return;
    if (auto) {
      model.listeners = [];
      listenersCustom.value = false;
    } else if (model.listeners.length === 0) {
      model.listeners = defaultListeners(model.listenPort || 11010);
      listenersCustom.value = false;
    }
  },
);

watch(
  () => props.show,
  (v) => {
    if (!v) return;
    advanced.value = false;
    if (props.network) {
      const n = props.network;
      Object.assign(model, {
        name: n.name,
        description: n.description ?? "",
        networkName: n.networkName,
        networkSecret: "",
        generateSecret: false,
        ipv4: n.ipv4,
        dhcp: n.dhcp,
        hostname: n.hostname,
        instanceName: n.instanceName,
        externalNode: n.externalNode ?? "",
        listeners: [...n.listeners],
        mappedListeners: [...n.mappedListeners],
        peers: [...n.peers],
        listenPort: n.listenPort,
        rpcPort: n.rpcPort,
        autoPorts: false,
        secureMode: n.secureMode,
        autoStart: n.autoStart,
        startNow: false,
        flags: { ...DEFAULT_FLAGS, ...n.flags },
      });
    } else {
      Object.assign(model, {
        name: "",
        description: "",
        networkName: "",
        networkSecret: "",
        generateSecret: true,
        ipv4: "10.126.126.1/24",
        dhcp: false,
        hostname: "",
        instanceName: "",
        externalNode: "",
        listeners: [],
        mappedListeners: [],
        peers: [],
        listenPort: 11010,
        rpcPort: 15888,
        autoPorts: true,
        secureMode: true,
        autoStart: false,
        startNow: true,
        flags: { ...DEFAULT_FLAGS },
      });
    }
    // 记录监听器是否已被手动定制（同步设置，避免上面的 watcher 误覆盖）
    listenersCustom.value =
      model.listeners.length > 0 && !listenersEqualDefault(model.listeners);
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
    const providedListeners = model.listeners.filter(Boolean);
    let listeners = providedListeners;
    if (!isEdit.value) {
      // 新建：自动分配端口时由后端按实际分配结果生成监听器；
      // 手动指定端口且未填监听器时，按该端口生成默认监听器
      if (model.autoPorts) {
        listeners = [];
      } else if (listeners.length === 0) {
        listeners = defaultListeners(model.listenPort || 11010);
      }
    } else if (props.network) {
      // 编辑：端口变更且监听器仍为旧默认值时，按新端口自动重建
      const wasDefault =
        JSON.stringify([...providedListeners].sort()) ===
        JSON.stringify([...defaultListeners(props.network.listenPort)].sort());
      if (wasDefault && model.listenPort !== props.network.listenPort) {
        listeners = defaultListeners(model.listenPort);
      }
    }
    const payload: NetworkPayload = {
      name: model.name,
      description: model.description,
      networkName: model.networkName || model.name,
      ipv4: model.ipv4,
      dhcp: model.dhcp,
      hostname: model.hostname || undefined,
      instanceName: model.instanceName || undefined,
      externalNode: model.externalNode,
      listeners,
      mappedListeners: model.mappedListeners.filter(Boolean),
      peers: model.peers.filter(Boolean),
      secureMode: model.secureMode,
      autoStart: model.autoStart,
      flags: { ...model.flags },
    };
    let saved: Network;
    if (isEdit.value && props.network) {
      if (model.networkSecret) payload.networkSecret = model.networkSecret;
      if (!portsDisabled.value) {
        payload.listenPort = model.listenPort;
        payload.rpcPort = model.rpcPort;
      }
      saved = await networkApi.update(props.network.id, payload);
      message.success(t("networkForm.form.updated"));
    } else {
      if (!model.generateSecret && model.networkSecret)
        payload.networkSecret = model.networkSecret;
      payload.startNow = model.startNow;
      // 默认由后端自动分配端口，避免与容器端口映射不一致
      if (!model.autoPorts) {
        payload.listenPort = model.listenPort;
        payload.rpcPort = model.rpcPort;
      }
      saved = await networkApi.create(payload);
      message.success(t("networkForm.form.created"));
    }
    emit("saved", saved);
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
    :title="isEdit ? t('networkForm.form.editTitle', { name: props.network?.name ?? '' }) : t('networkForm.form.createTitle')"
    style="width: 720px; max-width: 95vw"
    :mask-closable="false"
  >
    <n-form ref="formRef" :model="model" :rules="rules" label-placement="top">
      <n-divider title-placement="left" style="margin-top: 0">{{ t("networkForm.form.sectionBasic") }}</n-divider>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi :label="t('networkForm.form.name')" path="name">
          <n-input v-model:value="model.name" :placeholder="t('networkForm.form.namePlaceholder')" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.networkId')">
          <n-input
            v-model:value="model.networkName"
            :placeholder="t('networkForm.form.networkIdPlaceholder')"
          />
        </n-form-item-gi>
      </n-grid>
      <n-form-item :label="t('networkForm.form.description')">
        <n-input
          v-model:value="model.description"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 3 }"
          :placeholder="t('common.optional')"
        />
      </n-form-item>

      <n-divider title-placement="left">{{ t("networkForm.form.sectionAddress") }}</n-divider>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi :label="t('networkForm.form.ipv4')" path="ipv4">
          <n-input
            v-model:value="model.ipv4"
            :disabled="model.dhcp"
            placeholder="10.126.126.1/24"
          />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.dhcp')">
          <n-switch v-model:value="model.dhcp" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.hostname')">
          <n-input v-model:value="model.hostname" :placeholder="t('networkForm.form.autoGeneratePlaceholder')" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.instanceName')">
          <n-input v-model:value="model.instanceName" :placeholder="t('networkForm.form.autoGeneratePlaceholder')" />
        </n-form-item-gi>
      </n-grid>

      <n-divider title-placement="left">{{ t("networkForm.form.sectionConnection") }}</n-divider>
      <n-form-item :label="t('networkForm.form.externalNode')">
        <n-input
          v-model:value="model.externalNode"
          :placeholder="t('networkForm.form.externalNodePlaceholder')"
        />
      </n-form-item>
      <n-form-item :label="t('networkForm.form.listeners')">
        <div class="stack">
          <n-dynamic-tags
            v-model:value="model.listeners"
            :disabled="!isEdit && model.autoPorts"
          />
          <n-button
            size="tiny"
            quaternary
            :disabled="!isEdit && model.autoPorts"
            @click="fillDefaultListeners"
          >
            {{ t("networkForm.form.generateDefaultListeners") }}
          </n-button>
          <span v-if="!isEdit && model.autoPorts" class="hint">
            {{ t("networkForm.form.autoListenersHint") }}
          </span>
          <span v-else class="hint">
            {{ t("networkForm.form.listenersHint") }}
          </span>
        </div>
      </n-form-item>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi :label="t('networkForm.form.mappedListeners')">
          <n-dynamic-tags v-model:value="model.mappedListeners" size="small" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.peers')">
          <n-dynamic-tags v-model:value="model.peers" size="small" />
        </n-form-item-gi>
      </n-grid>

      <n-divider title-placement="left">{{ t("networkForm.form.sectionPortSecurity") }}</n-divider>
      <n-grid :cols="3" :x-gap="14">
        <n-form-item-gi v-if="!isEdit" :label="t('networkForm.form.portAllocation')">
          <n-switch v-model:value="model.autoPorts">
            <template #checked>{{ t("networkForm.form.autoAllocate") }}</template>
            <template #unchecked>{{ t("networkForm.form.manualAllocate") }}</template>
          </n-switch>
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.listenPort')">
          <n-input-number
            v-model:value="model.listenPort"
            :min="1"
            :max="65535"
            :disabled="portsDisabled || (!isEdit && model.autoPorts)"
            style="width: 100%"
          />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.rpcPort')">
          <n-input-number
            v-model:value="model.rpcPort"
            :min="1"
            :max="65535"
            :disabled="portsDisabled || (!isEdit && model.autoPorts)"
            style="width: 100%"
          />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.secureMode')">
          <n-switch v-model:value="model.secureMode" />
        </n-form-item-gi>
        <n-form-item-gi :label="t('networkForm.form.autoStart')">
          <n-switch v-model:value="model.autoStart" />
        </n-form-item-gi>
      </n-grid>
      <n-alert v-if="!isEdit && model.autoPorts" type="info" :show-icon="true" class="mb">
        {{ t("networkForm.form.autoPortsAlertPrefix") }}<span class="mono">EASYTIER_LISTEN_PORT_START</span> /
        <span class="mono">EASYTIER_RPC_PORT_START</span>{{ t("networkForm.form.autoPortsAlertSuffix") }}
      </n-alert>
      <n-alert v-if="portsDisabled" type="info" :show-icon="true" class="mb">
        {{ t("networkForm.form.portsDisabledAlert") }}
      </n-alert>
      <n-form-item :label="t('networkForm.form.networkSecret')">
        <n-input-group v-if="!isEdit">
          <n-input
            v-model:value="model.networkSecret"
            :disabled="model.generateSecret"
            :placeholder="t('networkForm.form.secretPlaceholder')"
            show-password-on="click"
            type="password"
          />
          <n-checkbox v-model:checked="model.generateSecret" class="secret-check">
            {{ t("networkForm.form.autoGenerate") }}
          </n-checkbox>
        </n-input-group>
        <n-input
          v-else
          v-model:value="model.networkSecret"
          :placeholder="t('networkForm.form.secretEditPlaceholder')"
          show-password-on="click"
          type="password"
        />
      </n-form-item>

      <n-divider title-placement="left">
        {{ t("networkForm.form.sectionAdvanced") }}
        <n-button quaternary size="tiny" @click="advanced = !advanced">
          {{ advanced ? t("networkForm.form.collapse") : t("networkForm.form.expand") }}
        </n-button>
      </n-divider>
      <template v-if="advanced">
        <n-grid :cols="3" :x-gap="14">
          <n-form-item-gi :label="t('networkForm.form.defaultProtocol')">
            <n-select
              v-model:value="model.flags.default_protocol"
              :options="[
                { label: 'tcp', value: 'tcp' },
                { label: 'udp', value: 'udp' },
                { label: 'wg', value: 'wg' },
                { label: 'quic', value: 'quic' },
                { label: 'ws', value: 'ws' },
                { label: 'wss', value: 'wss' },
              ]"
            />
          </n-form-item-gi>
          <n-form-item-gi label="MTU">
            <n-input-number v-model:value="model.flags.mtu" :min="576" :max="9000" style="width: 100%" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.relayWhitelist')">
            <n-input v-model:value="model.flags.relay_network_whitelist" />
          </n-form-item-gi>
        </n-grid>
        <n-grid :cols="3" :x-gap="14">
          <n-form-item-gi :label="t('networkForm.form.enableEncryption')">
            <n-switch v-model:value="model.flags.enable_encryption" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.enableIpv6')">
            <n-switch v-model:value="model.flags.enable_ipv6" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.latencyFirst')">
            <n-switch v-model:value="model.flags.latency_first" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.exitNode')">
            <n-switch v-model:value="model.flags.enable_exit_node" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.noTun')">
            <n-switch v-model:value="model.flags.no_tun" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.privateMode')">
            <n-switch v-model:value="model.flags.private_mode" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.disableP2p')">
            <n-switch v-model:value="model.flags.disable_p2p" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.disableUdpHolePunching')">
            <n-switch v-model:value="model.flags.disable_udp_hole_punching" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('networkForm.form.disableTcpHolePunching')">
            <n-switch v-model:value="model.flags.disable_tcp_hole_punching" />
          </n-form-item-gi>
        </n-grid>
        <n-form-item v-if="!isEdit" :label="t('networkForm.form.startNow')">
          <n-switch v-model:value="model.startNow" />
        </n-form-item>
      </template>
    </n-form>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">{{ t("common.cancel") }}</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">
          {{ isEdit ? t("common.saveChanges") : t("networkForm.form.createNetwork") }}
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.stack {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}
.secret-check {
  margin-left: 10px;
  white-space: nowrap;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.mb {
  margin-bottom: 12px;
}
.hint {
  font-size: 12px;
  opacity: 0.6;
}
</style>
