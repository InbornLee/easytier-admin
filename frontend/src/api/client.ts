import axios, { type AxiosError } from "axios";

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
      return data.issues.map((i) => i.message).join("；");
    }
    if (data?.message) return data.message;
    if (error.code === "ECONNABORTED") return "请求超时，请稍后重试";
    if (!error.response) return "无法连接到服务器";
    return `请求失败 (${error.response.status})`;
  }
  if (error instanceof Error) return error.message;
  return String(error);
}

export default client;
