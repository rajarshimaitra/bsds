"use client";

/**
 * Dashboard Layout — client component.
 *
 * Guards all /dashboard/* routes:
 * 1. Loading → show nothing (or spinner)
 * 2. No user → redirect to /login
 * 3. mustChangePassword === true → redirect to /change-password
 * 4. Valid user → render DashboardShell with user data
 */

import { useEffect } from "react";
import { useRouter } from "next/navigation";

import { useAuth } from "@/hooks/use-auth";
import DashboardShell from "@/components/layout/DashboardShell";
import type { Role } from "@/types";

interface DashboardLayoutProps {
  children: React.ReactNode;
}

export default function DashboardLayout({ children }: DashboardLayoutProps) {
  const { user, loading } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (loading) return;
    if (!user) {
      router.replace("/login");
    } else if (user.mustChangePassword) {
      router.replace("/change-password");
    }
  }, [user, loading, router]);

  // While loading or redirecting, render nothing
  if (loading || !user || user.mustChangePassword) {
    return null;
  }

  // Normalise user data for DashboardShell
  const shellUser = {
    name: user.username ?? "User",
    role: (user.role ?? "MEMBER") as Role,
    memberId: user.memberId ?? "",
  };

  return <DashboardShell user={shellUser}>{children}</DashboardShell>;
}
