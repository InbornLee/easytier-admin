<script setup lang="ts">
import { computed, h, ref } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";
import {
  NIcon,
  type MenuOption,
  type DropdownOption,
} from "naive-ui";
import {
  GridOutline,
  GitNetworkOutline,
  KeyOutline,
  DocumentTextOutline,
  SettingsOutline,
  MoonOutline,
  SunnyOutline,
  LogOutOutline,
  PersonCircleOutline,
  MenuOutline,
} from "@vicons/ionicons5";
import { useAuthStore } from "@/stores/auth";
import { isDark, toggleTheme } from "@/utils/theme";
import { dialog } from "@/utils/feedback";

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const collapsed = ref(false);

function renderIcon(icon: typeof GridOutline) {
  return () => h(NIcon, null, { default: () => h(icon) });
}

const menuOptions: MenuOption[] = [
  { label: "概览", key: "dashboard", icon: renderIcon(GridOutline) },
  { label: "网络管理", key: "networks", icon: renderIcon(GitNetworkOutline) },
  { label: "凭据管理", key: "credentials", icon: renderIcon(KeyOutline) },
  { label: "日志中心", key: "logs", icon: renderIcon(DocumentTextOutline) },
  { label: "系统设置", key: "settings", icon: renderIcon(SettingsOutline) },
];

const activeKey = computed(() => {
  const name = route.name as string;
  if (name === "network-detail") return "networks";
  return name;
});

function handleMenu(key: string) {
  router.push({ name: key });
}

const userOptions = computed<DropdownOption[]>(() => [
  {
    label: `${auth.user?.username ?? ""}（${roleLabel(auth.user?.role)}）`,
    key: "profile",
    icon: renderIcon(PersonCircleOutline),
  },
  { type: "divider", key: "d1" },
  { label: "修改密码", key: "password", icon: renderIcon(SettingsOutline) },
  { label: "退出登录", key: "logout", icon: renderIcon(LogOutOutline) },
]);

function roleLabel(role?: string) {
  switch (role) {
    case "admin":
      return "管理员";
    case "operator":
      return "运维";
    case "viewer":
      return "访客";
    default:
      return role ?? "";
  }
}

function handleUserSelect(key: string) {
  if (key === "password") {
    router.push({ name: "settings", query: { tab: "security" } });
  } else if (key === "logout") {
    dialog.warning({
      title: "退出登录",
      content: "确定要退出当前账号吗？",
      positiveText: "退出",
      negativeText: "取消",
      onPositiveClick: async () => {
        await auth.logout();
        router.push({ name: "login" });
      },
    });
  }
}

const pageTitle = computed(() => (route.meta.title as string) ?? "EasyTier 控制台");
</script>

<template>
  <n-layout has-sider class="app-layout">
    <n-layout-sider
      bordered
      collapse-mode="width"
      :collapsed-width="64"
      :width="220"
      :collapsed="collapsed"
      show-trigger
      @collapse="collapsed = true"
      @expand="collapsed = false"
    >
      <div class="brand" :class="{ collapsed }">
        <div class="brand-logo">ET</div>
        <transition name="fade">
          <div v-if="!collapsed" class="brand-text">
            <div class="brand-title">EasyTier 控制台</div>
            <div class="brand-sub">网络与节点管理平台</div>
          </div>
        </transition>
      </div>
      <n-menu
        :value="activeKey"
        :collapsed="collapsed"
        :collapsed-width="64"
        :collapsed-icon-size="20"
        :options="menuOptions"
        @update:value="handleMenu"
      />
    </n-layout-sider>

    <n-layout>
      <n-layout-header bordered class="app-header">
        <div class="header-left">
          <n-button quaternary circle size="small" @click="collapsed = !collapsed">
            <template #icon><n-icon :component="MenuOutline" /></template>
          </n-button>
          <h2 class="header-title">{{ pageTitle }}</h2>
        </div>
        <div class="header-right">
          <n-button quaternary circle size="small" @click="toggleTheme">
            <template #icon>
              <n-icon :component="isDark ? SunnyOutline : MoonOutline" />
            </template>
          </n-button>
          <n-dropdown :options="userOptions" trigger="click" @select="handleUserSelect">
            <n-button quaternary size="small">
              <template #icon><n-icon :component="PersonCircleOutline" /></template>
              {{ auth.user?.username }}
            </n-button>
          </n-dropdown>
        </div>
      </n-layout-header>

      <n-layout-content class="app-content">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </n-layout-content>
    </n-layout>
  </n-layout>
</template>

<style scoped>
.app-layout {
  height: 100vh;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 16px 18px;
  height: 64px;
}
.brand.collapsed {
  justify-content: center;
  padding: 16px 0;
}
.brand-logo {
  width: 34px;
  height: 34px;
  border-radius: 9px;
  background: linear-gradient(135deg, #2563eb, #0d9488);
  color: #fff;
  font-weight: 800;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.brand-title {
  font-weight: 700;
  font-size: 14.5px;
  line-height: 1.2;
}
.brand-sub {
  font-size: 11.5px;
  opacity: 0.6;
}
.app-header {
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}
.header-title {
  font-size: 16px;
  font-weight: 650;
  margin: 0;
}
.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.app-content {
  height: calc(100vh - 64px);
  overflow: auto;
}
</style>
