import { defineStore } from "pinia";
import { ref } from "vue";
import { authApi } from "@/api";
import type { Role } from "@/types";

export interface SessionUser {
  id: string;
  username: string;
  role: Role;
}

export const useAuthStore = defineStore("auth", () => {
  const user = ref<SessionUser | null>(null);
  const initialized = ref<boolean | null>(null);
  const ready = ref(false);

  async function bootstrap(): Promise<void> {
    try {
      const status = await authApi.status();
      initialized.value = status.initialized;
      if (status.initialized) {
        try {
          const { user: me } = await authApi.me();
          user.value = me as SessionUser;
        } catch {
          user.value = null;
        }
      }
    } catch {
      initialized.value = true;
    } finally {
      ready.value = true;
    }
  }

  async function login(username: string, password: string): Promise<SessionUser> {
    const { user: me } = await authApi.login({ username, password });
    user.value = { id: me.id, username: me.username, role: me.role };
    initialized.value = true;
    return user.value;
  }

  async function setup(payload: {
    username: string;
    password: string;
    displayName?: string;
    email?: string;
  }): Promise<SessionUser> {
    const { user: me } = await authApi.setup(payload);
    user.value = { id: me.id, username: me.username, role: me.role };
    initialized.value = true;
    return user.value;
  }

  async function logout(): Promise<void> {
    try {
      await authApi.logout();
    } finally {
      user.value = null;
    }
  }

  function clear(): void {
    user.value = null;
  }

  return { user, initialized, ready, bootstrap, login, setup, logout, clear };
});
