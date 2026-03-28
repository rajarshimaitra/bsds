"use client";

import React, {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
} from "react";
import type { AuthUser } from "@/lib/auth-types";
import {
  getMe,
  login as authLogin,
  logout as authLogout,
} from "@/lib/auth-client";

interface AuthContextValue {
  user: AuthUser | null;
  loading: boolean;
  login: (
    username: string,
    password: string
  ) => Promise<{
    ok: boolean;
    mustChangePassword?: boolean;
    error?: string;
  }>;
  logout: () => Promise<void>;
  refresh: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    const me = await getMe();
    setUser(me);
  }, []);

  useEffect(() => {
    getMe().then((me) => {
      setUser(me);
      setLoading(false);
    });
  }, []);

  const login = useCallback(
    async (username: string, password: string) => {
      const result = await authLogin(username, password);
      if (result.ok) await refresh();
      return result;
    },
    [refresh]
  );

  const logout = useCallback(async () => {
    await authLogout();
    setUser(null);
  }, []);

  return (
    <AuthContext.Provider value={{ user, loading, login, logout, refresh }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuthContext() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuthContext must be used within AuthProvider");
  return ctx;
}
