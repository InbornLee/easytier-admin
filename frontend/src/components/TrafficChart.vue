<script setup lang="ts">
import { computed } from "vue";
import VChart from "vue-echarts";
import { isDark } from "@/utils/theme";
import { formatBytes } from "@/utils/format";
import type { TrafficPoint } from "@/types";

const props = defineProps<{
  series: TrafficPoint[];
  title?: string;
  height?: number;
}>();

const option = computed(() => {
  const dark = isDark.value;
  const series = props.series ?? [];
  const textColor = dark ? "#d0d5dd" : "#475467";
  const splitColor = dark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.06)";
  return {
    tooltip: {
      trigger: "axis",
      valueFormatter: (v: number) => formatBytes(v),
    },
    legend: {
      data: ["接收", "发送"],
      textStyle: { color: textColor },
      right: 10,
      top: 0,
      icon: "roundRect",
    },
    grid: { left: 8, right: 16, top: 38, bottom: 8, containLabel: true },
    xAxis: {
      type: "time",
      axisLine: { lineStyle: { color: splitColor } },
      axisLabel: { color: textColor, hideOverlap: true },
      splitLine: { show: false },
    },
    yAxis: {
      type: "value",
      axisLabel: {
        color: textColor,
        formatter: (v: number) => formatBytes(v),
      },
      splitLine: { lineStyle: { color: splitColor } },
    },
    series: [
      {
        name: "接收",
        type: "line",
        smooth: true,
        showSymbol: false,
        areaStyle: { opacity: 0.12, color: "#2563eb" },
        lineStyle: { color: "#2563eb", width: 2 },
        itemStyle: { color: "#2563eb" },
        data: series.map((p) => [p.t, p.rx]),
      },
      {
        name: "发送",
        type: "line",
        smooth: true,
        showSymbol: false,
        areaStyle: { opacity: 0.12, color: "#0d9488" },
        lineStyle: { color: "#0d9488", width: 2 },
        itemStyle: { color: "#0d9488" },
        data: series.map((p) => [p.t, p.tx]),
      },
    ],
  };
});
</script>

<template>
  <div>
    <div v-if="title" class="chart-title">{{ title }}</div>
    <v-chart
      v-if="series.length"
      class="chart"
      :style="{ height: (height ?? 280) + 'px' }"
      :option="option"
      autoresize
    />
    <div v-else class="empty-block">
      <n-empty description="暂无流量数据（每 30 秒采样一次）" />
    </div>
  </div>
</template>

<style scoped>
.chart {
  width: 100%;
}
.chart-title {
  font-weight: 600;
  margin-bottom: 4px;
}
</style>
