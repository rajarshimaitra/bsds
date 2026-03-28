/**
 * DashboardShell — server-renderable wrapper that composes the sidebar layout.
 *
 * Desktop (>= lg / 1024px):
 *   - Fixed 256px sidebar on the left
 *   - Content area fills remaining width
 *
 * Mobile/tablet (< lg):
 *   - Sidebar hidden; accessible via the Header hamburger button (Sheet overlay)
 *   - Content area is full width
 *
 * This component itself is a server component (no "use client").
 * The Sidebar and Header children are client components that handle
 * interactivity (active link highlighting, mobile sheet, logout).
 */

import Header from "@/components/layout/Header";
import Sidebar from "@/components/layout/Sidebar";
import type { Role } from "@/types";

interface DashboardShellProps {
  children: React.ReactNode;
  user: {
    name: string;
    role: Role;
    memberId: string;
  };
}

/**
 * Root layout shell for all /dashboard/* pages.
 *
 * @param children  - Page content rendered in the main area
 * @param user      - Authenticated user's name, role, and memberId
 */
export default function DashboardShell({ children, user }: DashboardShellProps) {
  return (
    <div className="flex min-h-screen bg-transparent">
      <aside className="hidden lg:fixed lg:inset-y-0 lg:left-0 lg:z-40 lg:flex lg:w-72 lg:shrink-0 lg:flex-col">
        <Sidebar user={user} />
      </aside>

      <div className="relative flex flex-1 flex-col lg:pl-72">
        <Header user={user} />

        <main className="flex-1 px-3 pb-6 pt-20 sm:px-4 sm:pb-8 lg:px-8 lg:pb-10 lg:pt-10">
          <div className="mx-auto w-full max-w-[1600px]">{children}</div>
        </main>
      </div>
    </div>
  );
}
