<script setup lang="ts">
import { computed, ref } from "vue";
import { CopyOutline, CheckmarkOutline } from "@vicons/ionicons5";
import { message } from "@/utils/feedback";
import { copyToClipboard } from "@/utils/clipboard";

const props = defineProps<{
  text: string;
  label?: string;
  masked?: boolean;
}>();

const copied = ref(false);

const display = computed(() => {
  if (!props.masked) return props.text;
  if (!props.text) return "-";
  if (props.text.length <= 12) return props.text;
  return `${props.text.slice(0, 6)}••••••${props.text.slice(-4)}`;
});

async function copy() {
  const ok = await copyToClipboard(props.text);
  if (ok) {
    copied.value = true;
    message.success("已复制到剪贴板");
    setTimeout(() => (copied.value = false), 1500);
  } else {
    message.error("复制失败，请手动选择文本复制");
  }
}
</script>

<template>
  <span class="copy-text">
    <span v-if="label" class="copy-label">{{ label }}</span>
    <span class="mono copy-value">{{ display || "-" }}</span>
    <n-button v-if="text" quaternary size="tiny" @click="copy">
      <template #icon>
        <n-icon :component="copied ? CheckmarkOutline : CopyOutline" />
      </template>
    </n-button>
  </span>
</template>

<style scoped>
.copy-text {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 100%;
}
.copy-label {
  opacity: 0.65;
  font-size: 13px;
}
.copy-value {
  word-break: break-all;
}
</style>
