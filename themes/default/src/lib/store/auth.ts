/**
 * Auth Store - 基于 SDK 的认证状态管理
 * 用于检测管理员登录状态（前台不再支持用户登录/注册）
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
  
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
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

  clearError: () => set({ error: null }),
}));
