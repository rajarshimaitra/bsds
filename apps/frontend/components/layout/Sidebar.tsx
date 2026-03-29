"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useAuth } from "@/hooks/use-auth";
import {
  Activity,
  CheckSquare,
  FileText,
  Handshake,
  IndianRupee,
  LayoutDashboard,
  LogOut,
  Users,
  UserCircle,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { UI_DASHBOARD_NAME } from "@/lib/branding";
import type { Role } from "@/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface NavItem {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  href: string;
  /** Badge suffix shown next to label (e.g. "read-only") */
  badge?: string;
}

interface SidebarProps {
  /** Current signed-in user data passed from the server layout */
  user: {
    name: string;
    role: Role;
    memberId: string;
  };
  /** Optionally close the mobile sheet after navigation */
  onNavigate?: () => void;
}

// ---------------------------------------------------------------------------
// Nav item definitions per role
// ---------------------------------------------------------------------------

const ADMIN_NAV: NavItem[] = [
  { label: "Dashboard Home", icon: LayoutDashboard, href: "/dashboard" },
  { label: "My Membership", icon: UserCircle, href: "/dashboard/my-membership" },
  { label: "Member Management", icon: Users, href: "/dashboard/members" },
  { label: "Cash Management", icon: IndianRupee, href: "/dashboard/cash" },
  { label: "Sponsorship Management", icon: Handshake, href: "/dashboard/sponsorship" },
  { label: "Approval Queue", icon: CheckSquare, href: "/dashboard/approvals" },
  { label: "Financial Audit Log", icon: FileText, href: "/dashboard/audit-log" },
  { label: "Activity Log", icon: Activity, href: "/dashboard/activity-log" },
];

const OPERATOR_NAV: NavItem[] = [
  { label: "Dashboard Home", icon: LayoutDashboard, href: "/dashboard" },
  { label: "My Membership", icon: UserCircle, href: "/dashboard/my-membership" },
  { label: "Member Management", icon: Users, href: "/dashboard/members" },
  { label: "Cash Management", icon: IndianRupee, href: "/dashboard/cash" },
  { label: "Financial Audit Log", icon: FileText, href: "/dashboard/audit-log" },
  { label: "Activity Log", icon: Activity, href: "/dashboard/activity-log" },
];

const ORGANISER_NAV: NavItem[] = [
  { label: "Dashboard Home", icon: LayoutDashboard, href: "/dashboard" },
  { label: "My Membership", icon: UserCircle, href: "/dashboard/my-membership" },
  { label: "Member Management", icon: Users, href: "/dashboard/members" },
  { label: "Cash Management", icon: IndianRupee, href: "/dashboard/cash" },
  { label: "Financial Audit Log", icon: FileText, href: "/dashboard/audit-log" },
  { label: "Activity Log", icon: Activity, href: "/dashboard/activity-log" },
];

const MEMBER_NAV: NavItem[] = [
  { label: "Dashboard Home", icon: LayoutDashboard, href: "/dashboard" },
  { label: "My Membership", icon: UserCircle, href: "/dashboard/my-membership" },
];

function getNavItems(role: Role): NavItem[] {
  switch (role) {
    case "ADMIN":
      return ADMIN_NAV;
    case "OPERATOR":
      return OPERATOR_NAV;
    case "ORGANISER":
      return ORGANISER_NAV;
    case "MEMBER":
      return MEMBER_NAV;
    default:
      return MEMBER_NAV;
  }
}

/** Human-readable role label for the badge */
function roleBadgeLabel(role: Role): string {
  switch (role) {
    case "ADMIN":
      return "Admin";
    case "OPERATOR":
      return "Operator";
    case "ORGANISER":
      return "Organiser";
    case "MEMBER":
      return "Member";
  }
}

/** Badge variant per role */
function roleBadgeVariant(role: Role): "default" | "secondary" | "outline" {
  switch (role) {
    case "ADMIN":
      return "default";
    case "OPERATOR":
      return "secondary";
    case "ORGANISER":
      return "outline";
    case "MEMBER":
      return "outline";
  }
}

// ---------------------------------------------------------------------------
// Internal sub-components
// ---------------------------------------------------------------------------

/**
 * Club logo / name block displayed at the top of the sidebar.
 */
function SidebarBrand() {
  return (
    <div className="px-5 pb-4 pt-7">
      <div className="flex items-center gap-3">
        <Image
          src="/images/logo.jpg"
          alt="BALLYGUNGE SARBOJANIN DURGOTSAB SAMITY (DESHAPRIYA PARK)"
          width={44}
          height={44}
          className="h-11 w-11 shrink-0 rounded-2xl border border-white/20 object-cover shadow-[0_18px_40px_-20px_rgba(234,88,12,0.85)]"
        />
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold leading-tight text-white">
            {UI_DASHBOARD_NAME}
          </p>
          <p className="truncate text-[11px] leading-tight text-slate-400">
            Deshapriya Park Operations
          </p>
        </div>
      </div>
    </div>
  );
}

function NavSectionLabel() {
  return (
    <div className="mb-3 px-3">
      <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-slate-500">
        Navigation
      </p>
    </div>
  );
}

/**
 * A single navigation item row.
 */
function NavItemRow({
  item,
  isActive,
  onClick,
}: {
  item: NavItem;
  isActive: boolean;
  onClick?: () => void;
}) {
  const Icon = item.icon;

  return (
    <Link
      href={item.href}
      onClick={onClick}
      className={cn(
        "group flex items-center gap-3 rounded-2xl px-3.5 py-3 text-sm font-medium transition-all duration-200",
        isActive
          ? "bg-gradient-to-r from-orange-500 to-amber-500 text-white shadow-[0_18px_34px_-20px_rgba(234,88,12,0.85)]"
          : "text-slate-300 hover:bg-white/10 hover:text-white"
      )}
      aria-current={isActive ? "page" : undefined}
    >
      <span
        className={cn(
          "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border transition-colors",
          isActive
            ? "border-white/20 bg-white/15 text-white"
            : "border-white/10 bg-white/5 text-slate-400 group-hover:text-white"
        )}
      >
        <Icon className="h-4 w-4 shrink-0" />
      </span>
      <span className="flex-1 truncate">{item.label}</span>
      {item.badge && (
        <Badge
          variant="outline"
          className={cn(
            "shrink-0 border-white/10 bg-white/5 px-2 py-1 text-[9px] text-slate-300",
            isActive && "border-white/20 bg-white/10 text-white"
          )}
        >
          {item.badge}
        </Badge>
      )}
    </Link>
  );
}

// ---------------------------------------------------------------------------
// Main exported component
// ---------------------------------------------------------------------------

/**
 * Role-based navigation sidebar.
 *
 * Renders different nav items for ADMIN, OPERATOR, and MEMBER roles.
 * Can be used both as a fixed desktop sidebar and inside a mobile Sheet overlay.
 *
 * @param user  - Signed-in user's name, role, and memberId
 * @param onNavigate - Optional callback invoked after a nav link is clicked
 *                     (used to close the mobile Sheet)
 */
export default function Sidebar({ user, onNavigate }: SidebarProps) {
  const pathname = usePathname();
  const router = useRouter();
  const { logout } = useAuth();
  const navItems = getNavItems(user.role);

  async function handleLogout() {
    onNavigate?.();
    await logout();
    router.push("/login");
  }

  /**
   * Determine if a nav item is "active".
   * Dashboard Home only matches exact /dashboard; all others match the prefix.
   */
  function isActive(href: string): boolean {
    if (href === "/dashboard") {
      return pathname === "/dashboard";
    }
    return pathname === href || pathname.startsWith(href + "/");
  }

  return (
    <div className="flex h-full w-full max-w-72 flex-col bg-[radial-gradient(circle_at_top,_rgba(251,146,60,0.18),_transparent_18rem),linear-gradient(180deg,#1a0e05_0%,#1e1308_52%,#1a0e05_100%)] text-white">
      <SidebarBrand />

      <nav className="flex-1 overflow-y-auto px-4 pb-6" aria-label="Main navigation">
        <NavSectionLabel />
        <ul className="space-y-2" role="list">
          {navItems.map((item) => (
            <li key={item.href}>
              <NavItemRow
                item={item}
                isActive={isActive(item.href)}
                onClick={onNavigate}
              />
            </li>
          ))}
        </ul>
      </nav>

      <div className="border-t border-white/10 px-4 py-4">
        <button
          type="button"
          onClick={handleLogout}
          className="flex w-full items-center gap-3 rounded-2xl border border-white/10 bg-white/5 px-3.5 py-3 text-left text-sm font-medium text-slate-300 transition-all duration-200 hover:bg-white/10 hover:text-white"
        >
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-slate-400">
            <LogOut className="h-4 w-4 shrink-0" />
          </span>
          <span className="flex-1 truncate">Logout</span>
        </button>
      </div>
    </div>
  );
}
