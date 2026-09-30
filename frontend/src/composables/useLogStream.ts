import { onUnmounted, ref, watch, type Ref } from "vue";
import { logApi } from "@/api";
import type { LogLine } from "@/components/LogViewer.vue";

export function useLogStream(
  networkId: Ref<string | undefined>,
  max = 800,
): { lines: Ref<LogLine[]>; connected: Ref<boolean>; clear: () => void } {
  const lines = ref<LogLine[]>([]);
  const connected = ref(false);
  let es: EventSource | null = null;

  function connect() {
    disconnect();
    es = new EventSource(logApi.streamUrl(networkId.value));
    es.addEventListener("ready", () => {
      connected.value = true;
    });
    es.addEventListener("log", (event) => {
      try {
        const line = JSON.parse((event as MessageEvent).data) as LogLine;
        lines.value.push(line);
        if (lines.value.length > max) {
          lines.value.splice(0, lines.value.length - max);
        }
      } catch {
        /* ignore malformed */
      }
    });
    es.onerror = () => {
      connected.value = false;
    };
  }

  function disconnect() {
    if (es) {
      es.close();
      es = null;
    }
    connected.value = false;
  }

  watch(networkId, () => {
    lines.value = [];
    connect();
  });

  connect();
  onUnmounted(disconnect);

  return { lines, connected, clear: () => (lines.value = []) };
}
