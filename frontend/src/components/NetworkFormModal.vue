<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
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

const rules: FormRules = {
  name: [{ required: true, message: "请输入网络名称", trigger: ["blur", "input"] }],
  ipv4: [
    {
      validator: (_r, value: string) => {
        if (model.dhcp) return true;
        return /^\d{1,3}(\.\d{1,3}){3}\/\d{1,2}$/.test(value || "");
      },
      message: "格式应为 CIDR，例如 10.126.126.1/24",
      trigger: ["blur"],
    },
  ],
};

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
      message.success("网络配置已更新");
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
      message.success("网络创建成功");
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
    :title="isEdit ? `编辑网络 · ${props.network?.name ?? ''}` : '新建网络'"
    style="width: 720px; max-width: 95vw"
    :mask-closable="false"
  >
    <n-form ref="formRef" :model="model" :rules="rules" label-placement="top">
      <n-divider title-placement="left" style="margin-top: 0">基础信息</n-divider>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi label="网络名称" path="name">
          <n-input v-model:value="model.name" placeholder="例如：办公网" />
        </n-form-item-gi>
        <n-form-item-gi label="EasyTier 网络标识 (network-name)">
          <n-input
            v-model:value="model.networkName"
            placeholder="留空则与网络名称相同，建议英文"
          />
        </n-form-item-gi>
      </n-grid>
      <n-form-item label="描述">
        <n-input
          v-model:value="model.description"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 3 }"
          placeholder="可选"
        />
      </n-form-item>

      <n-divider title-placement="left">网络与地址</n-divider>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi label="虚拟网段 (ipv4)" path="ipv4">
          <n-input
            v-model:value="model.ipv4"
            :disabled="model.dhcp"
            placeholder="10.126.126.1/24"
          />
        </n-form-item-gi>
        <n-form-item-gi label="DHCP 自动分配共享节点 IP">
          <n-switch v-model:value="model.dhcp" />
        </n-form-item-gi>
        <n-form-item-gi label="共享节点主机名 (hostname)">
          <n-input v-model:value="model.hostname" placeholder="留空自动生成" />
        </n-form-item-gi>
        <n-form-item-gi label="实例名 (instance-name)">
          <n-input v-model:value="model.instanceName" placeholder="留空自动生成" />
        </n-form-item-gi>
      </n-grid>

      <n-divider title-placement="left">连接配置</n-divider>
      <n-form-item label="公共共享节点 / 初始 Peer (external node)">
        <n-input
          v-model:value="model.externalNode"
          placeholder="可选，留空表示不设置初始 Peer；例如 tcp://<公共共享节点>:11010"
        />
      </n-form-item>
      <n-form-item label="监听器 (listeners)">
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
            按端口生成默认监听 (tcp/udp + ws)
          </n-button>
          <span v-if="!isEdit && model.autoPorts" class="hint">
            自动分配端口时，将由后端根据实际分配的端口生成默认监听器（tcp/udp + ws）。
          </span>
          <span v-else class="hint">
            监听器会随「监听端口」自动同步（tcp/udp:端口 + ws:端口+1）；手动编辑后将不再自动覆盖。
          </span>
        </div>
      </n-form-item>
      <n-grid :cols="2" :x-gap="14">
        <n-form-item-gi label="映射监听器 (mapped listeners)">
          <n-dynamic-tags v-model:value="model.mappedListeners" size="small" />
        </n-form-item-gi>
        <n-form-item-gi label="初始 Peer 列表">
          <n-dynamic-tags v-model:value="model.peers" size="small" />
        </n-form-item-gi>
      </n-grid>

      <n-divider title-placement="left">端口与安全</n-divider>
      <n-grid :cols="3" :x-gap="14">
        <n-form-item-gi v-if="!isEdit" label="端口分配">
          <n-switch v-model:value="model.autoPorts">
            <template #checked>自动分配</template>
            <template #unchecked>手动指定</template>
          </n-switch>
        </n-form-item-gi>
        <n-form-item-gi label="监听端口">
          <n-input-number
            v-model:value="model.listenPort"
            :min="1"
            :max="65535"
            :disabled="portsDisabled || (!isEdit && model.autoPorts)"
            style="width: 100%"
          />
        </n-form-item-gi>
        <n-form-item-gi label="RPC 端口">
          <n-input-number
            v-model:value="model.rpcPort"
            :min="1"
            :max="65535"
            :disabled="portsDisabled || (!isEdit && model.autoPorts)"
            style="width: 100%"
          />
        </n-form-item-gi>
        <n-form-item-gi label="安全模式">
          <n-switch v-model:value="model.secureMode" />
        </n-form-item-gi>
        <n-form-item-gi label="自动启动">
          <n-switch v-model:value="model.autoStart" />
        </n-form-item-gi>
      </n-grid>
      <n-alert v-if="!isEdit && model.autoPorts" type="info" :show-icon="true" class="mb">
        将由后端自动分配可用端口（从 <span class="mono">EASYTIER_LISTEN_PORT_START</span> /
        <span class="mono">EASYTIER_RPC_PORT_START</span> 起）。请确保这些端口已在 docker-compose 中映射。
      </n-alert>
      <n-alert v-if="portsDisabled" type="info" :show-icon="true" class="mb">
        网络运行中无法修改端口，请先停止网络。
      </n-alert>
      <n-form-item label="网络密钥 (network-secret)">
        <n-input-group v-if="!isEdit">
          <n-input
            v-model:value="model.networkSecret"
            :disabled="model.generateSecret"
            placeholder="留空自动生成高强度密钥"
            show-password-on="click"
            type="password"
          />
          <n-checkbox v-model:checked="model.generateSecret" class="secret-check">
            自动生成
          </n-checkbox>
        </n-input-group>
        <n-input
          v-else
          v-model:value="model.networkSecret"
          placeholder="留空表示不修改现有密钥"
          show-password-on="click"
          type="password"
        />
      </n-form-item>

      <n-divider title-placement="left">
        高级参数
        <n-button quaternary size="tiny" @click="advanced = !advanced">
          {{ advanced ? "收起" : "展开" }}
        </n-button>
      </n-divider>
      <template v-if="advanced">
        <n-grid :cols="3" :x-gap="14">
          <n-form-item-gi label="默认协议">
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
          <n-form-item-gi label="转发网络白名单">
            <n-input v-model:value="model.flags.relay_network_whitelist" />
          </n-form-item-gi>
        </n-grid>
        <n-grid :cols="3" :x-gap="14">
          <n-form-item-gi label="加密通信">
            <n-switch v-model:value="model.flags.enable_encryption" />
          </n-form-item-gi>
          <n-form-item-gi label="启用 IPv6">
            <n-switch v-model:value="model.flags.enable_ipv6" />
          </n-form-item-gi>
          <n-form-item-gi label="延迟优先">
            <n-switch v-model:value="model.flags.latency_first" />
          </n-form-item-gi>
          <n-form-item-gi label="允许作为出口节点">
            <n-switch v-model:value="model.flags.enable_exit_node" />
          </n-form-item-gi>
          <n-form-item-gi label="不创建 TUN 设备 (no-tun)">
            <n-switch v-model:value="model.flags.no_tun" />
          </n-form-item-gi>
          <n-form-item-gi label="私有模式 (private-mode)">
            <n-switch v-model:value="model.flags.private_mode" />
          </n-form-item-gi>
          <n-form-item-gi label="禁用 P2P">
            <n-switch v-model:value="model.flags.disable_p2p" />
          </n-form-item-gi>
          <n-form-item-gi label="禁用 UDP 打洞">
            <n-switch v-model:value="model.flags.disable_udp_hole_punching" />
          </n-form-item-gi>
          <n-form-item-gi label="禁用 TCP 打洞">
            <n-switch v-model:value="model.flags.disable_tcp_hole_punching" />
          </n-form-item-gi>
        </n-grid>
        <n-form-item v-if="!isEdit" label="创建后立即启动">
          <n-switch v-model:value="model.startNow" />
        </n-form-item>
      </template>
    </n-form>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">取消</n-button>
        <n-button type="primary" :loading="submitting" @click="submit">
          {{ isEdit ? "保存修改" : "创建网络" }}
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
