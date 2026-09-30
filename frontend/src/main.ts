import { createApp } from "vue";
import { createPinia } from "pinia";
import naive from "naive-ui";
import App from "./App.vue";
import router from "./router";
import { i18n } from "./i18n";

import "./styles/global.css";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import "@vue-flow/controls/dist/style.css";
import "@vue-flow/minimap/dist/style.css";

import { use } from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import { LineChart } from "echarts/charts";
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
  DataZoomComponent,
} from "echarts/components";

use([
  CanvasRenderer,
  LineChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  DataZoomComponent,
]);

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.use(naive);
app.use(i18n);
app.mount("#app");
