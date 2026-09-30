<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { formatTime } from "@/utils/format";

export interface LogLine {
  networkId?: string | null;
  level: string;
  message: string;
  createdAt: number;
}

const props = withDefaults(
  defineProps<{
    lines: LogLine[];
    height?: number;
    showFilter?: boolean;
    autoScroll?: boolean;
  }>(),
  { height: 460, showFilter: true, autoScroll: true },
);

const levelFilter = ref<string | null>(null);
const search = ref("");
const scroller = ref<HTMLElement | null>(null);

const filtered = computed(() =>
  props.lines.filter((l) => {
    if (levelFilter.value && l.level !== levelFilter.value) return false;
    if (search.value && !l.message.toLowerCase().includes(search.value.toLowerCase()))
      return false;
    return true;
  }),
);

const levelOptions = [
  { label: "全部级别", value: "" },
  { label: "TRACE", value: "trace" },
  { label: "DEBUG", value: "debug" },
  { label: "INFO", value: "info" },
  { label: "WARN", value: "warn" },
  { label: "ERROR", value: "error" },
];

function levelClass(level: string) {
  return `lv-${level}`;
}

function scrollToBottom() {
  if (!props.autoScroll) return;
  nextTick(() => {
    if (scroller.value) scroller.value.scrollTop = scroller.value.scrollHeight;
  });
}

watch(
  () => filtered.value.length,
  () => scrollToBottom(),
);
watch(() => props.lines, () => scrollToBottom(), { deep: false });
</script>

<template>
  <div class="log-viewer">
    <div v-if="showFilter" class="log-toolbar">
      <n-select
        v-model:value="levelFilter"
        :options="levelOptions"
        size="small"
        style="width: 130px"
        clearable
        placeholder="全部级别"
      />
      <n-input
        v-model:value="search"
        size="small"
        placeholder="搜索日志内容"
        clearable
        style="width: 220px"
      />
      <div class="log-count">{{ filtered.length }} 条</div>
    </div>
    <div ref="scroller" class="log-body" :style="{ height: height + 'px' }">
      <div v-if="!filtered.length" class="log-empty">暂无日志</div>
      <div v-for="(line, idx) in filtered" :key="idx" class="log-line">
        <span class="log-time">{{ formatTime(line.createdAt) }}</span>
        <span class="log-level" :class="levelClass(line.level)">
          {{ line.level.toUpperCase() }}
        </span>
        <span class="log-msg">{{ line.message }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.log-viewer {
  border: 1px solid rgba(128, 128, 128, 0.18);
  border-radius: 10px;
  overflow: hidden;
}
.log-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.15);
  background: rgba(128, 128, 128, 0.04);
}
.log-count {
  margin-left: auto;
  font-size: 12px;
  opacity: 0.6;
}
.log-body {
  overflow: auto;
  padding: 6px 0;
  font-family: "JetBrains Mono", "Fira Code", Consolas, monospace;
  font-size: 12.5px;
  line-height: 1.65;
}
.log-empty {
  text-align: center;
  padding: 40px 0;
  opacity: 0.5;
}
.log-line {
  display: flex;
  gap: 10px;
  padding: 1px 12px;
  white-space: pre-wrap;
  word-break: break-all;
}
.log-line:hover {
  background: rgba(128, 128, 128, 0.08);
}
.log-time {
  opacity: 0.5;
  flex-shrink: 0;
}
.log-level {
  flex-shrink: 0;
  width: 48px;
  font-weight: 600;
}
.lv-error {
  color: #e5484d;
}
.lv-warn {
  color: #f5a524;
}
.lv-info {
  color: #2563eb;
}
.lv-debug {
  color: #8b5cf6;
}
.lv-trace {
  opacity: 0.6;
}
.log-msg {
  flex: 1;
  min-width: 0;
}
</style>
