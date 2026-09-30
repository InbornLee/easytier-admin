<script setup lang="ts">
import { computed } from "vue";
import {
  credentialStatusLabel,
  credentialStatusType,
  networkStatusLabel,
  networkStatusType,
} from "@/utils/format";

const props = defineProps<{
  kind: "network" | "credential" | "node";
  status: string;
}>();

const tagType = computed(() => {
  if (props.kind === "network") return networkStatusType(props.status);
  if (props.kind === "credential") return credentialStatusType(props.status);
  switch (props.status) {
    case "online":
      return "success" as const;
    case "offline":
      return "error" as const;
    default:
      return "default" as const;
  }
});

const label = computed(() => {
  if (props.kind === "network") return networkStatusLabel(props.status);
  if (props.kind === "credential") return credentialStatusLabel(props.status);
  switch (props.status) {
    case "online":
      return "在线";
    case "offline":
      return "离线";
    default:
      return "未知";
  }
});
</script>

<template>
  <n-tag :type="tagType" size="small" round :bordered="false">
    {{ label }}
  </n-tag>
</template>
