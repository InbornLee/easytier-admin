import axios, { type AxiosError } from "axios";
import { translate } from "@/i18n";

const client = axios.create({
  baseURL: "/",
  withCredentials: true,
  timeout: 30_000,
});

client.interceptors.response.use(
  (res) => res,
  (error: AxiosError) => {
    const status = error.response?.status;
    if (status === 401) {
      const path = window.location.pathname;
      if (!path.startsWith("/login") && !path.startsWith("/setup")) {
        window.dispatchEvent(new CustomEvent("et:unauthorized"));
      }
    }
    return Promise.reject(error);
  },
);

export function extractError(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const data = error.response?.data as
      | { message?: string; issues?: Array<{ path: string; message: string }> }
      | undefined;
    if (data?.issues?.length) {
      return data.issues.map((i) => i.message).join(translate("error.issuesJoin"));
    }
    if (data?.message) return data.message;
    if (error.code === "ECONNABORTED") return translate("error.timeout");
    if (!error.response) return translate("error.offline");
    return translate("error.requestFailed", { status: error.response.status });
  }
  if (error instanceof Error) return error.message;
  return String(error);
}

export default client;
