/**
 * Page component module-structure tests.
 *
 * Covers: All Next.js App Router page components and layouts — verifies that
 *         each file is importable without runtime errors and exports a default
 *         React component function.
 * Does NOT cover: rendered markup, data fetching, redirects, or any
 *                 server-side auth guard logic — those require the full
 *                 Next.js runtime.
 * Protects: apps/frontend/app/ (all page.tsx and layout.tsx files)
 *
 * Ported from: dps-dashboard/tests/components/pages.test.ts
 * Auth changes: No next-auth imports existed in the original page-structure
 *               tests. Auth is mocked via useAuth in tests/setup.ts so that
 *               client components using useAuth can be imported safely.
 */

import { describe, it, expect, vi } from "vitest";

// ---------------------------------------------------------------------------
// Mock useAuth so any client component that calls useAuth() on import
// (or at module evaluation time) does not throw "must be used within AuthProvider".
// ---------------------------------------------------------------------------
vi.mock("@/hooks/use-auth", () => ({
  useAuth: () => ({
    user: { id: "1", username: "test", role: "ADMIN", mustChangePassword: false },
    loading: false,
    login: vi.fn(),
    logout: vi.fn(),
    refresh: vi.fn(),
  }),
}));

vi.mock("@/components/providers/AuthProvider", () => ({
  AuthProvider: ({ children }: { children: React.ReactNode }) => children,
  useAuthContext: () => ({
    user: { id: "1", username: "test", role: "ADMIN", mustChangePassword: false },
    loading: false,
    login: vi.fn(),
    logout: vi.fn(),
    refresh: vi.fn(),
  }),
}));

// ---------------------------------------------------------------------------
// Root-level pages
// ---------------------------------------------------------------------------

describe("Root pages — module structure", () => {
  it("/ (home) page exports a default component", async () => {
    const module = await import("@/app/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/login page exports a default component", async () => {
    const module = await import("@/app/login/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/change-password page exports a default component", async () => {
    const module = await import("@/app/change-password/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/membership-form page exports a default component", async () => {
    const module = await import("@/app/membership-form/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// Sponsor pages (public)
// ---------------------------------------------------------------------------

describe("Sponsor pages — module structure", () => {
  it("/sponsor/[token] page exports a default component", async () => {
    const module = await import("@/app/sponsor/[token]/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/sponsor/[token]/receipt page exports a default component", async () => {
    const module = await import("@/app/sponsor/[token]/receipt/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// Dashboard pages (authenticated)
// ---------------------------------------------------------------------------

describe("Dashboard pages — module structure", () => {
  it("/dashboard page exports a default component", async () => {
    const module = await import("@/app/dashboard/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/dashboard/members page exports a default component", async () => {
    const module = await import("@/app/dashboard/members/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/dashboard/cash page exports a default component", async () => {
    const module = await import("@/app/dashboard/cash/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/dashboard/approvals page exports a default component", async () => {
    const module = await import("@/app/dashboard/approvals/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/dashboard/sponsorship page exports a default component", async () => {
    const module = await import("@/app/dashboard/sponsorship/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/dashboard/my-membership page exports a default component", async () => {
    const module = await import("@/app/dashboard/my-membership/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/dashboard/audit-log page exports a default component", async () => {
    const module = await import("@/app/dashboard/audit-log/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });

  it("/dashboard/activity-log page exports a default component", async () => {
    const module = await import("@/app/dashboard/activity-log/page");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// App layouts
// ---------------------------------------------------------------------------

describe("App layouts — module structure", () => {
  // Root layout uses next/font/local (localFont) which is a Next.js build-time
  // API not available in the Vitest jsdom environment. We skip import-level
  // testing for the root layout and instead verify the dashboard layout.
  it.skip("Root layout exports a default component (skipped: uses next/font/local)", async () => {
    // next/font/local is not available outside the Next.js build pipeline.
    // The root layout is verified at build time via `tsc --noEmit`.
  });

  it("Dashboard layout exports a default component", async () => {
    const module = await import("@/app/dashboard/layout");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});
