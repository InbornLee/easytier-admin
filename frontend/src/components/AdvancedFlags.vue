<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { NodeAdvancedFlags } from "@/types";

const { t } = useI18n();

// 与父级 model.flags 双向绑定；属性级修改会直接反映到父级的响应式对象上。
const flags = defineModel<NodeAdvancedFlags>({ required: true });

const protocolOptions = [
  { label: "tcp", value: "tcp" },
  { label: "udp", value: "udp" },
  { label: "wg", value: "wg" },
  { label: "quic", value: "quic" },
  { label: "ws", value: "ws" },
  { label: "wss", value: "wss" },
];

const encryptionOptions = [
  { label: t("nodes.adv.encryptionDefault"), value: "" },
  { label: "aes-gcm", value: "aes-gcm" },
  { label: "aes-gcm-256", value: "aes-gcm-256" },
  { label: "chacha20", value: "chacha20" },
  { label: "xor", value: "xor" },
  { label: "openssl-aes128-gcm", value: "openssl-aes128-gcm" },
  { label: "openssl-aes256-gcm", value: "openssl-aes256-gcm" },
  { label: "openssl-chacha20", value: "openssl-chacha20" },
];
</script>

<template>
  <div class="adv">
    <div class="group-title">{{ t("nodes.adv.groupTransport") }}</div>
    <n-grid :cols="3" :x-gap="14">
      <n-form-item-gi :label="t('nodes.adv.defaultProtocol')">
        <n-select v-model:value="flags.default_protocol" :options="protocolOptions" />
      </n-form-item-gi>
      <n-form-item-gi label="MTU">
        <n-input-number v-model:value="flags.mtu" :min="576" :max="9000" style="width: 100%" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.devName')">
        <n-input v-model:value="flags.dev_name" :placeholder="t('nodes.adv.devNamePlaceholder')" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.enableEncryption')">
        <n-switch v-model:value="flags.enable_encryption" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.encryptionAlgorithm')">
        <n-select v-model:value="flags.encryption_algorithm" :options="encryptionOptions" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.enableIpv6')">
        <n-switch v-model:value="flags.enable_ipv6" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.useSmoltcp')">
        <n-switch v-model:value="flags.use_smoltcp" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.noTun')">
        <n-switch v-model:value="flags.no_tun" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.bindDevice')">
        <n-switch v-model:value="flags.bind_device" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.latencyFirst')">
        <n-switch v-model:value="flags.latency_first" />
      </n-form-item-gi>
    </n-grid>

    <div class="group-title">{{ t("nodes.adv.groupP2p") }}</div>
    <n-grid :cols="3" :x-gap="14">
      <n-form-item-gi :label="t('nodes.adv.disableP2p')">
        <n-switch v-model:value="flags.disable_p2p" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.p2pOnly')">
        <n-switch v-model:value="flags.p2p_only" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.lazyP2p')">
        <n-switch v-model:value="flags.lazy_p2p" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.needP2p')">
        <n-switch v-model:value="flags.need_p2p" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableTcpHole')">
        <n-switch v-model:value="flags.disable_tcp_hole_punching" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableUdpHole')">
        <n-switch v-model:value="flags.disable_udp_hole_punching" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableSymHole')">
        <n-switch v-model:value="flags.disable_sym_hole_punching" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableUpnp')">
        <n-switch v-model:value="flags.disable_upnp" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.multiThread')">
        <n-switch v-model:value="flags.multi_thread" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.multiThreadCount')">
        <n-input-number
          v-model:value="flags.multi_thread_count"
          :min="0"
          :max="64"
          :disabled="!flags.multi_thread"
          style="width: 100%"
        />
      </n-form-item-gi>
    </n-grid>

    <div class="group-title">{{ t("nodes.adv.groupAccel") }}</div>
    <n-grid :cols="3" :x-gap="14">
      <n-form-item-gi :label="t('nodes.adv.enableKcpProxy')">
        <n-switch v-model:value="flags.enable_kcp_proxy" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableKcpInput')">
        <n-switch v-model:value="flags.disable_kcp_input" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableRelayKcp')">
        <n-switch v-model:value="flags.disable_relay_kcp" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.enableQuicProxy')">
        <n-switch v-model:value="flags.enable_quic_proxy" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableQuicInput')">
        <n-switch v-model:value="flags.disable_quic_input" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableRelayQuic')">
        <n-switch v-model:value="flags.disable_relay_quic" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.quicListenPort')">
        <n-input-number
          v-model:value="flags.quic_listen_port"
          :min="0"
          :max="65535"
          style="width: 100%"
        />
      </n-form-item-gi>
    </n-grid>

    <div class="group-title">{{ t("nodes.adv.groupRelay") }}</div>
    <n-grid :cols="2" :x-gap="14">
      <n-form-item-gi :label="t('nodes.adv.relayWhitelist')">
        <n-input
          v-model:value="flags.relay_network_whitelist"
          :placeholder="t('nodes.adv.relayWhitelistPlaceholder')"
        />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.relayAllPeerRpc')">
        <n-switch v-model:value="flags.relay_all_peer_rpc" />
      </n-form-item-gi>
    </n-grid>
    <span class="hint">{{ t("nodes.adv.relayWhitelistHint") }}</span>
    <n-grid :cols="3" :x-gap="14">
      <n-form-item-gi :label="t('nodes.adv.enableExitNode')">
        <n-switch v-model:value="flags.enable_exit_node" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.proxyForwardBySystem')">
        <n-switch v-model:value="flags.proxy_forward_by_system" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.privateMode')">
        <n-switch v-model:value="flags.private_mode" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.disableRelayData')">
        <n-switch v-model:value="flags.disable_relay_data" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.foreignRelayBpsLimit')">
        <n-input-number
          v-model:value="flags.foreign_relay_bps_limit"
          :min="0"
          style="width: 100%"
        />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.instanceRecvBpsLimit')">
        <n-input-number
          v-model:value="flags.instance_recv_bps_limit"
          :min="0"
          style="width: 100%"
        />
      </n-form-item-gi>
    </n-grid>

    <div class="group-title">{{ t("nodes.adv.groupOther") }}</div>
    <n-grid :cols="3" :x-gap="14">
      <n-form-item-gi :label="t('nodes.adv.acceptDns')">
        <n-switch v-model:value="flags.accept_dns" />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.tldDnsZone')">
        <n-input
          v-model:value="flags.tld_dns_zone"
          :disabled="!flags.accept_dns"
          placeholder="et.net."
        />
      </n-form-item-gi>
      <n-form-item-gi :label="t('nodes.adv.udpBroadcastRelay')">
        <n-switch v-model:value="flags.enable_udp_broadcast_relay" />
      </n-form-item-gi>
    </n-grid>
  </div>
</template>

<style scoped>
.adv {
  width: 100%;
}
.group-title {
  margin: 10px 0 8px;
  font-size: 12px;
  font-weight: 600;
  opacity: 0.75;
}
.hint {
  display: block;
  margin: -4px 0 10px;
  font-size: 12px;
  opacity: 0.6;
}
</style>
