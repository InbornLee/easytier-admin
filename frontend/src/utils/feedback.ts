import { computed } from "vue";
import { createDiscreteApi } from "naive-ui";
import { naiveTheme } from "./theme";

const configProviderProps = computed(() => ({ theme: naiveTheme.value }));

const discrete = createDiscreteApi(["message", "dialog", "notification", "loadingBar"], {
  configProviderProps,
});

export const message = discrete.message;
export const dialog = discrete.dialog;
export const notification = discrete.notification;
export const loadingBar = discrete.loadingBar;
