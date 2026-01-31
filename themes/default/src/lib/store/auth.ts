/**
 * Auth Store - 基于 SDK 的认证状态管理
 * 使用 Noteva SDK 替代直接 API 调用
 */
import { create } from "zustand";

interface User {
  id: number;
  username: string;
  email: string;
  avatar?: string;
  display_name?: string;
  role: string;
}

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
  
  login: (usernameOrEmail: string, password: string) => Promise<void>;
  register: (username: string, email: string, password: string, verificationCode?: string) => Promise<void>;
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
  updateProfile: (data: { display_name?: string; avatar?: string }) => Promise<void>;
  changePassword: (currentPassword: string, newPassword: string) => Promise<void>;
  clearError: () => void;
}

// 获取 SDK 实例
function getNoteva() {
  if (typeof window !== "undefined" && window.Noteva) {
    return window.Noteva;
  }
  return null;
}

// 等待 SDK 就绪
async function waitForSDK(): Promise<typeof window.Noteva> {
  return new Promise((resolve) => {
    const check = () => {
      const sdk = getNoteva();
      if (sdk) {
        sdk.ready().then(() => resolve(sdk));
      } else {
        setTimeout(check, 50);
      }
    };
    check();
  });
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  isAuthenticated: false,
  isLoading: false,
  error: null,

  login: async (usernameOrEmail: string, password: string) => {
    set({ isLoading: true, error: null });
    try {
      const sdk = await waitForSDK();
      const result = await sdk.user.login({ username: usernameOrEmail, password });
      set({ user: result.user, isAuthenticated: true, isLoading: false });
    } catch (error: any) {
      const message = error.data?.error || error.message || "Login failed";
      set({ error: message, isLoading: false });
      throw error;
    }
  },

  register: async (username: string, email: string, password: string, verificationCode?: string) => {
    set({ isLoading: true, error: null });
    try {
      const sdk = await waitForSDK();
      const registerData: any = { username, email, password };
      if (verificationCode) {
        registerData.verification_code = verificationCode;
      }
      await sdk.user.register(registerData);
      set({ isLoading: false });
    } catch (error: any) {
      const message = error.data?.error || error.message || "Registration failed";
      set({ error: message, isLoading: false });
      throw error;
    }
  },

  logout: async () => {
    try {
      const sdk = getNoteva();
      if (sdk) {
        await sdk.user.logout();
      }
    } catch {
      // Ignore logout errors
    } finally {
      set({ user: null, isAuthenticated: false });
    }
  },

  checkAuth: async () => {
    const currentState = useAuthStore.getState();
    if (currentState.isAuthenticated && currentState.user) {
      return;
    }

    try {
      const sdk = await waitForSDK();
      const user = await sdk.user.check();
      if (user) {
        set({ user, isAuthenticated: true });
      } else {
        set({ user: null, isAuthenticated: false });
      }
    } catch {
      set({ user: null, isAuthenticated: false });
    }
  },

  updateProfile: async (data: { display_name?: string; avatar?: string }) => {
    set({ isLoading: true, error: null });
    try {
      const sdk = await waitForSDK();
      const updatedUser = await sdk.user.updateProfile(data);
      set({ user: updatedUser, isLoading: false });
    } catch (error: any) {
      const message = error.data?.error?.message || error.message || "Update failed";
      set({ error: message, isLoading: false });
      throw error;
    }
  },

  changePassword: async (currentPassword: string, newPassword: string) => {
    set({ isLoading: true, error: null });
    try {
      const sdk = await waitForSDK();
      await sdk.user.changePassword({ current_password: currentPassword, new_password: newPassword });
      set({ isLoading: false });
    } catch (error: any) {
      const message = error.data?.error?.message || error.message || "Password change failed";
      set({ error: message, isLoading: false });
      throw error;
    }
  },

  clearError: () => set({ error: null }),
}));
