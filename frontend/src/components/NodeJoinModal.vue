<script setup lang="ts">
import { computed, ref, watch } from "vue";
import QRCode from "qrcode";
import { DownloadOutline } from "@vicons/ionicons5";
import CopyText from "@/components/CopyText.vue";
import { nodeApi } from "@/api";
import { extractError } from "@/api/client";
import { message } from "@/utils/feedback";
import { formatTime } from "@/utils/format";
import type { JoinInfo } from "@/types";

const props = defineProps<{
  show: boolean;
  nodeId?: string | null;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
}>();

const visible = computed({
  get: () => props.show,
  set: (v) => emit("update:show", v),
});

const loading = ref(false);
const info = ref<JoinInfo | null>(null);
const qrDataUrl = ref("");
const tab = ref<"command" | "config">("command");
// 可选的监听端口覆盖；null 表示使用节点保存的监听器
const listenPort = ref<number | null>(null);

// EasyTier 2.x 临时凭据必须通过 --credential 传入，单独用配置文件无法完成凭据认证
const configRunCommand = computed(() => {
  if (!info.value) return "";
  const core = info.value.command.split(" ")[0] || "easytier-core";
  return `${core} -c <config.toml 路径> --credential ${info.value.credential.secret}`;
});
const peer = ref<string>("");

async function loadJoin() {
  if (!props.nodeId) return;
  loading.value = true;
  info.value = null;
  qrDataUrl.value = "";
  try {
    const data = await nodeApi.join(
      props.nodeId,
      listenPort.value ?? undefined,
      peer.value || undefined,
    );
    info.value = data;
    peer.value = data.peer;
    qrDataUrl.value = await QRCode.toDataURL(data.command, {
      width: 240,
      margin: 1,
      errorCorrectionLevel: "M",
    });
  } catch (err) {
    message.error(extractError(err));
    visible.value = false;
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.show,
  (v) => {
    if (!v) return;
    listenPort.value = null;
    peer.value = "";
    void loadJoin();
  },
);

function downloadConfig() {
  if (!info.value) return;
  const blob = new Blob([info.value.configToml], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${info.value.node.hostname || "easytier-node"}.toml`;
  a.click();
  URL.revokeObjectURL(url);
}
</script>

<template>
  <n-modal
    v-model:show="visible"
    preset="card"
    title="节点接入信息"
    style="width: 760px; max-width: 95vw"
    :mask-closable="false"
  >
    <n-spin :show="loading">
      <template v-if="info">
        <n-alert type="warning" :show-icon="true" class="mb">
          凭据密钥等同于密码，请仅通过安全渠道发送给对应设备。凭据有效期至
          <strong>{{ formatTime(info.credential.expiresAt) }}</strong>，撤销后即可立即禁止接入。
        </n-alert>

        <n-descriptions :column="2" size="small" bordered class="mb">
          <n-descriptions-item label="节点名称">{{ info.node.name }}</n-descriptions-item>
          <n-descriptions-item label="虚拟 IP">
            {{ info.node.ipv4 || "DHCP 自动分配" }}
          </n-descriptions-item>
          <n-descriptions-item label="网络">{{ info.network.name }}</n-descriptions-item>
          <n-descriptions-item label="连接节点">
            <span class="mono">{{ info.peer }}</span>
          </n-descriptions-item>
          <n-descriptions-item label="连接地址 (peer)" :span="2">
            <div class="peer-edit">
              <n-input
                v-model:value="peer"
                size="tiny"
                placeholder="例如 tcp://<共享节点IP或域名>:11011"
                style="max-width: 360px"
              />
              <n-button size="tiny" type="primary" ghost @click="loadJoin">重新生成</n-button>
              <span class="port-hint">
                客户端将连接此地址加入网络；建议填写共享节点可达地址（同局域网用宿主 IP，公网需端口映射）
              </span>
            </div>
          </n-descriptions-item>
          <n-descriptions-item label="监听器 (listeners)" :span="2">
            <span class="mono">{{ info.listeners.join("    ") }}</span>
          </n-descriptions-item>
          <n-descriptions-item label="覆盖监听端口（可选）">
            <div class="port-edit">
              <n-input-number
                v-model:value="listenPort"
                size="tiny"
                :min="1"
                :max="65535"
                clearable
                placeholder="留空使用节点配置"
                style="width: 150px"
              />
              <n-button size="tiny" type="primary" ghost @click="loadJoin">应用</n-button>
              <span class="port-hint">留空则使用节点保存的监听器</span>
            </div>
          </n-descriptions-item>
          <n-descriptions-item label="共享节点公钥">
            <CopyText :text="info.sharedNodePublicKey" masked />
          </n-descriptions-item>
        </n-descriptions>

        <n-grid :cols="24" :x-gap="16">
          <n-gi :span="16">
            <n-tabs v-model:value="tab" type="segment" size="small">
              <n-tab-pane name="command" tab="一键运行命令">
                <div class="code-block mono">{{ info.command }}</div>
              </n-tab-pane>
              <n-tab-pane name="config" tab="配置文件">
                <n-alert type="warning" :show-icon="true" class="mb">
                  EasyTier 的<strong>临时凭据必须通过命令行 --credential 传入</strong>；
                  仅用配置文件（<span class="mono">[secure_mode] local_private_key</span>）无法完成凭据认证。
                  请使用下方命令启动，或直接用「一键运行命令」标签。
                </n-alert>
                <n-form-item label="使用配置文件启动（推荐）" :show-feedback="false">
                  <CopyText :text="configRunCommand" />
                </n-form-item>
                <div class="code-block mono">{{ info.configToml }}</div>
                <n-button size="small" class="mt-sm" @click="downloadConfig">
                  <template #icon><n-icon :component="DownloadOutline" /></template>
                  下载 config.toml
                </n-button>
              </n-tab-pane>
            </n-tabs>
            <div class="copy-row">
              <CopyText
                :text="tab === 'config' ? info.configToml : info.command"
                label="复制内容"
              />
            </div>
          </n-gi>
          <n-gi :span="8">
            <div class="qr-box">
              <img v-if="qrDataUrl" :src="qrDataUrl" alt="接入命令二维码" width="220" />
              <div class="qr-hint">扫码在移动端查看接入命令</div>
            </div>
          </n-gi>
        </n-grid>
      </template>
    </n-spin>

    <template #footer>
      <div class="modal-footer">
        <n-button @click="visible = false">关闭</n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.mb {
  margin-bottom: 14px;
}
.mt-sm {
  margin-top: 8px;
}
.copy-row {
  margin-top: 10px;
}
.qr-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding-top: 8px;
}
.qr-box img {
  border-radius: 10px;
  background: #fff;
  padding: 6px;
}
.qr-hint {
  font-size: 12px;
  opacity: 0.6;
}
.port-edit {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.peer-edit {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  width: 100%;
}
.port-hint {
  font-size: 11px;
  opacity: 0.55;
}
.modal-footer {
  display: flex;
  justify-content: flex-end;
}
</style>
