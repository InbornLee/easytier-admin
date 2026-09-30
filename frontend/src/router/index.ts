import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import { useAuthStore } from "@/stores/auth";

const routes: RouteRecordRaw[] = [
  {
    path: "/setup",
    name: "setup",
    component: () => import("@/views/SetupView.vue"),
    meta: { public: true, layout: "blank" },
  },
  {
    path: "/login",
    name: "login",
    component: () => import("@/views/LoginView.vue"),
    meta: { public: true, layout: "blank" },
  },
  {
    path: "/",
    component: () => import("@/layouts/MainLayout.vue"),
    children: [
      {
        path: "",
        name: "dashboard",
        component: () => import("@/views/DashboardView.vue"),
        meta: { title: "概览", icon: "dashboard" },
      },
      {
        path: "networks",
        name: "networks",
        component: () => import("@/views/NetworksView.vue"),
        meta: { title: "网络管理", icon: "networks" },
      },
      {
        path: "networks/:id",
        name: "network-detail",
        component: () => import("@/views/NetworkDetailView.vue"),
        meta: { title: "网络详情", hidden: true },
      },
      {
        path: "nodes",
        name: "nodes",
        component: () => import("@/views/NodesView.vue"),
        meta: { title: "节点管理", icon: "nodes" },
      },
      {
        path: "credentials",
        name: "credentials",
        component: () => import("@/views/CredentialsView.vue"),
        meta: { title: "凭据管理", icon: "credentials" },
      },
      {
        path: "logs",
        name: "logs",
        component: () => import("@/views/LogsView.vue"),
        meta: { title: "日志中心", icon: "logs" },
      },
      {
        path: "settings",
        name: "settings",
        component: () => import("@/views/SettingsView.vue"),
        meta: { title: "系统设置", icon: "settings" },
      },
    ],
  },
  {
    path: "/:pathMatch(.*)*",
    name: "not-found",
    component: () => import("@/views/NotFoundView.vue"),
    meta: { public: true, layout: "blank" },
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach(async (to) => {
  const auth = useAuthStore();
  if (!auth.ready) {
    await auth.bootstrap();
  }
  if (auth.initialized === false) {
    return to.name === "setup" ? true : { name: "setup" };
  }
  if (to.meta.public) {
    if (to.name === "setup" && auth.initialized) return { name: "dashboard" };
    if (to.name === "login" && auth.user) return { name: "dashboard" };
    return true;
  }
  if (!auth.user) {
    return { name: "login", query: { redirect: to.fullPath } };
  }
  return true;
});

export default router;
