<script setup lang="ts">
import { useRouter } from "vue-router";
import { CompassOutline } from "@vicons/ionicons5";

const router = useRouter();
</script>

<template>
  <div class="nf">
    <n-result status="404" title="页面不存在" description="您访问的页面走丢了">
      <template #icon>
        <n-icon :component="CompassOutline" />
      </template>
      <template #footer>
        <n-button type="primary" @click="router.push('/')">返回首页</n-button>
      </template>
    </n-result>
  </div>
</template>

<style scoped>
.nf {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
