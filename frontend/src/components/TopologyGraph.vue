<script setup lang="ts">
import { computed } from "vue";
import { VueFlow, Handle, Position, type Node, type Edge } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import { Controls } from "@vue-flow/controls";
import { MiniMap } from "@vue-flow/minimap";
import type { Topology } from "@/types";

const props = defineProps<{
  topology: Topology | null;
  loading?: boolean;
}>();

const RADIUS = 260;

const nodes = computed<Node[]>(() => {
  const topo = props.topology;
  if (!topo) return [];
  const peers = topo.nodes.filter((n) => n.kind !== "local");
  return topo.nodes.map((n) => {
    if (n.kind === "local") {
      return {
        id: n.id,
        type: "topology",
        position: { x: 0, y: 0 },
        data: n,
      };
    }
    const idx = peers.findIndex((p) => p.id === n.id);
    const angle = (2 * Math.PI * idx) / Math.max(1, peers.length) - Math.PI / 2;
    return {
      id: n.id,
      type: "topology",
      position: {
        x: Math.cos(angle) * RADIUS,
        y: Math.sin(angle) * RADIUS,
      },
      data: n,
    };
  });
});

const edges = computed<Edge[]>(() => {
  const topo = props.topology;
  if (!topo) return [];
  return topo.edges.map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    label: e.label,
    animated: !e.relayed,
    type: "smoothstep",
    style: e.relayed
      ? { stroke: "#f59e0b", strokeDasharray: "6 4", strokeWidth: 1.6 }
      : { stroke: "#22c55e", strokeWidth: 2 },
    labelBgPadding: [4, 2] as [number, number],
    labelBgBorderRadius: 4,
  }));
});

function nodeColor(node: Topology["nodes"][number]) {
  if (!node.online) return "#94a3b8";
  if (node.kind === "local") return "#2563eb";
  if (node.kind === "client") return "#0d9488";
  return "#16a34a";
}
</script>

<template>
  <div class="topology-wrap">
    <n-spin :show="!!loading">
      <VueFlow
        v-if="topology && topology.nodes.length"
        :nodes="nodes"
        :edges="edges"
        :min-zoom="0.3"
        :max-zoom="1.8"
        fit-view-on-init
        :default-edge-options="{ type: 'smoothstep' }"
        class="flow"
      >
        <template #node-topology="nodeProps">
          <div
            class="topo-node"
            :class="{ offline: !nodeProps.data.online }"
            :style="{ borderColor: nodeColor(nodeProps.data) }"
          >
            <Handle type="target" :position="Position.Top" class="hidden-handle" />
            <div class="topo-node-head">
              <span class="dot" :style="{ background: nodeColor(nodeProps.data) }" />
              <span class="topo-name">{{ nodeProps.data.label }}</span>
            </div>
            <div class="topo-ip mono">{{ nodeProps.data.ipv4 || "-" }}</div>
            <div class="topo-meta">
              <span v-if="nodeProps.data.kind === 'local'">控制台共享节点</span>
              <template v-else>
                <span v-if="nodeProps.data.latMs != null">{{ nodeProps.data.latMs }} ms</span>
                <span v-else>{{ nodeProps.data.online ? "已连接" : "未连接" }}</span>
                <span v-if="nodeProps.data.tunnelProto && nodeProps.data.tunnelProto !== '-'">
                  · {{ nodeProps.data.tunnelProto }}
                </span>
              </template>
            </div>
            <Handle type="source" :position="Position.Bottom" class="hidden-handle" />
          </div>
        </template>
        <Background :gap="20" :size="1.4" pattern-color="#d0d5dd" />
        <Controls position="bottom-right" />
        <MiniMap pannable zoomable />
      </VueFlow>
      <div v-else class="empty-block">
        <n-empty description="暂无节点连接数据">
          <template #extra>
            <span class="empty-hint">
              {{ topology?.error || "请先启动网络实例，节点接入后此处将展示连接拓扑" }}
            </span>
          </template>
        </n-empty>
      </div>
    </n-spin>
  </div>
</template>

<style scoped>
.topology-wrap {
  height: 540px;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid rgba(128, 128, 128, 0.18);
}
.flow {
  height: 540px;
  width: 100%;
}
.topo-node {
  background: var(--n-card-color, #fff);
  border: 2px solid #94a3b8;
  border-radius: 10px;
  padding: 10px 14px;
  min-width: 150px;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08);
}
.topo-node.offline {
  opacity: 0.7;
  border-style: dashed;
}
.topo-node-head {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 650;
  font-size: 13px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.topo-name {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.topo-ip {
  margin-top: 3px;
  opacity: 0.7;
}
.topo-meta {
  margin-top: 3px;
  font-size: 11.5px;
  opacity: 0.6;
  display: flex;
  gap: 4px;
}
.hidden-handle {
  opacity: 0;
  pointer-events: none;
}
.empty-hint {
  font-size: 13px;
  opacity: 0.6;
}
</style>
