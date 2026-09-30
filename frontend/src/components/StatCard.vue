<script setup lang="ts">
import type { Component } from "vue";

defineProps<{
  label: string;
  value: string | number;
  hint?: string;
  color?: string;
  icon?: Component;
  loading?: boolean;
}>();
</script>

<template>
  <n-card size="small" :bordered="true" class="stat-card">
    <div class="stat-inner">
      <div class="stat-icon" :style="{ background: (color ?? '#2563eb') + '1a', color: color ?? '#2563eb' }">
        <n-icon :size="22" :component="icon" />
      </div>
      <div class="stat-body">
        <div class="stat-label">{{ label }}</div>
        <n-skeleton v-if="loading" text style="width: 60px; height: 26px" />
        <div v-else class="stat-value">{{ value }}</div>
        <div v-if="hint" class="stat-hint">{{ hint }}</div>
      </div>
    </div>
  </n-card>
</template>

<style scoped>
.stat-card {
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}
.stat-card:hover {
  transform: translateY(-2px);
}
.stat-inner {
  display: flex;
  align-items: center;
  gap: 14px;
}
.stat-icon {
  width: 46px;
  height: 46px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.stat-body {
  min-width: 0;
}
.stat-label {
  font-size: 13px;
  opacity: 0.65;
}
.stat-value {
  font-size: 24px;
  font-weight: 700;
  line-height: 1.25;
}
.stat-hint {
  font-size: 12px;
  opacity: 0.55;
  margin-top: 2px;
}
</style>
