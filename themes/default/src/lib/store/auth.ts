/**
 * Auth Store - 基于 SDK 的认证状态管理
 * 用于检测管理员登录状态（前台不再支持用户登录/注册）
 */
import { create } from "zustand";
import { getNoteva, waitForNoteva, type NotevaUser } from "@/hooks/useNoteva";

type User = NotevaUser;

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
  
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
  clearError: () => void;
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
      const sdk = await waitForNoteva();
      if (!sdk) {
        set({ user: null, isAuthenticated: false });
        return;
      }

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
