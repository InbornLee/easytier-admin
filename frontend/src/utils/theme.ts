import { computed, ref } from "vue";
import { darkTheme } from "naive-ui";

const stored = localStorage.getItem("et-theme");
export const isDark = ref(
  stored
    ? stored === "dark"
    : window.matchMedia("(prefers-color-scheme: dark)").matches,
);

export const naiveTheme = computed(() => (isDark.value ? darkTheme : null));

export const themeOverrides = {
  common: {
    primaryColor: "#2563eb",
    primaryColorHover: "#3b82f6",
    primaryColorPressed: "#1d4ed8",
    primaryColorSuppl: "#3b82f6",
    borderRadius: "8px",
    fontFamily:
      "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif",
  },
  Card: { borderRadius: "12px" },
  DataTable: { borderRadius: "10px" },
};

export function toggleTheme(): void {
  isDark.value = !isDark.value;
  localStorage.setItem("et-theme", isDark.value ? "dark" : "light");
}
