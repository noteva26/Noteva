import { create } from "zustand";
import { authApi, User } from "@/lib/api";

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
  
  login: (usernameOrEmail: string, password: string) => Promise<void>;
  register: (username: string, email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  isAuthenticated: false,
  isLoading: false,
  error: null,

  login: async (usernameOrEmail: string, password: string) => {
    set({ isLoading: true, error: null });
    try {
      const { data } = await authApi.login(usernameOrEmail, password);
      // Cookie is set automatically by the server (httpOnly)
      set({ user: data.user, isAuthenticated: true, isLoading: false });
    } catch (error: any) {
      const message = error.response?.data?.error?.message || "Login failed";
      set({ error: message, isLoading: false });
      throw error;
    }
  },

  register: async (username: string, email: string, password: string) => {
    set({ isLoading: true, error: null });
    try {
      await authApi.register(username, email, password);
      set({ isLoading: false });
    } catch (error: any) {
      const message = error.response?.data?.error?.message || "Registration failed";
      set({ error: message, isLoading: false });
      throw error;
    }
  },

  logout: async () => {
    try {
      await authApi.logout();
    } catch {
      // Ignore logout errors
    } finally {
      // Cookie is cleared by the server
      set({ user: null, isAuthenticated: false });
    }
  },

  checkAuth: async () => {
    // If already authenticated with user data, skip API call
    const currentState = useAuthStore.getState();
    if (currentState.isAuthenticated && currentState.user) {
      return;
    }

    try {
      // Cookie is sent automatically with withCredentials: true
      const { data } = await authApi.me();
      set({ user: data, isAuthenticated: true });
    } catch {
      set({ user: null, isAuthenticated: false });
    }
  },

  clearError: () => set({ error: null }),
}));
