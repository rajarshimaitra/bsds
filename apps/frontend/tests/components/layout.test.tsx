/**
 * Layout and UI component module-structure tests.
 *
 * Covers: Sidebar, Header, DashboardShell, AuthProvider, landing components
 *         (NavBar, PrintButton), ReceiptView, and all shadcn/ui primitives.
 * Does NOT cover: rendered output, interactions, responsive behaviour, or
 *                 accessibility — those require a full Next.js render environment.
 * Protects: apps/frontend/components/layout/
 *           apps/frontend/components/providers/
 *           apps/frontend/components/landing/
 *           apps/frontend/components/receipts/
 *           apps/frontend/components/ui/
 *
 * Ported from: dps-dashboard/tests/components/layout.test.ts
 * Auth changes: SessionProvider reference replaced with AuthProvider.
 */

import { describe, it, expect } from "vitest";

// ---------------------------------------------------------------------------
// Layout components
// ---------------------------------------------------------------------------

describe("Sidebar component — module structure", () => {
  it("is importable and exports a default component", async () => {
    const module = await import("@/components/layout/Sidebar");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

describe("Header component — module structure", () => {
  it("is importable and exports a default component", async () => {
    const module = await import("@/components/layout/Header");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

describe("DashboardShell component — module structure", () => {
  it("is importable and exports a default component", async () => {
    const module = await import("@/components/layout/DashboardShell");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// Provider components
// ---------------------------------------------------------------------------

describe("AuthProvider component — module structure", () => {
  it("is importable and exports AuthProvider as a named export", async () => {
    const module = await import("@/components/providers/AuthProvider");
    expect(module.AuthProvider).toBeDefined();
    expect(typeof module.AuthProvider).toBe("function");
  });

  it("exports useAuthContext as a named export", async () => {
    const module = await import("@/components/providers/AuthProvider");
    expect(module.useAuthContext).toBeDefined();
    expect(typeof module.useAuthContext).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// Landing components
// ---------------------------------------------------------------------------

describe("NavBar component — module structure", () => {
  it("is importable and exports a default component", async () => {
    const module = await import("@/components/landing/NavBar");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

describe("PrintButton component — module structure", () => {
  it("is importable and exports a default component", async () => {
    const module = await import("@/components/landing/PrintButton");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// Receipt components
// ---------------------------------------------------------------------------

describe("ReceiptView component — module structure", () => {
  it("is importable and exports a default component", async () => {
    const module = await import("@/components/receipts/ReceiptView");
    expect(module.default).toBeDefined();
    expect(typeof module.default).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// UI primitive components (shadcn/ui)
// Note: shadcn/ui components use React.forwardRef which returns an object
// (typeof === "object"), not a plain function. We check they are defined
// and non-null (truthy) rather than checking typeof === "function".
// ---------------------------------------------------------------------------

describe("UI primitives — module structure", () => {
  it("Button is importable and exports Button", async () => {
    const module = await import("@/components/ui/button");
    expect(module.Button).toBeDefined();
    expect(module.Button).toBeTruthy();
  });

  it("Card exports Card, CardHeader, CardContent, CardTitle, CardDescription, CardFooter", async () => {
    const module = await import("@/components/ui/card");
    expect(module.Card).toBeDefined();
    expect(module.CardHeader).toBeDefined();
    expect(module.CardContent).toBeDefined();
    expect(module.CardTitle).toBeDefined();
    expect(module.CardDescription).toBeDefined();
    expect(module.CardFooter).toBeDefined();
  });

  it("Input is importable and exports Input", async () => {
    const module = await import("@/components/ui/input");
    expect(module.Input).toBeDefined();
    expect(module.Input).toBeTruthy();
  });

  it("Label is importable and exports Label", async () => {
    const module = await import("@/components/ui/label");
    expect(module.Label).toBeDefined();
    expect(module.Label).toBeTruthy();
  });

  it("Badge is importable and exports Badge", async () => {
    const module = await import("@/components/ui/badge");
    expect(module.Badge).toBeDefined();
    expect(typeof module.Badge).toBe("function"); // Badge uses a plain function, not forwardRef
  });

  it("Separator is importable", async () => {
    const module = await import("@/components/ui/separator");
    expect(module.Separator).toBeDefined();
    expect(module.Separator).toBeTruthy();
  });

  it("Dialog exports Dialog, DialogContent, DialogHeader, DialogTitle", async () => {
    const module = await import("@/components/ui/dialog");
    expect(module.Dialog).toBeDefined();
    expect(module.DialogContent).toBeDefined();
    expect(module.DialogHeader).toBeDefined();
    expect(module.DialogTitle).toBeDefined();
  });

  it("Select exports Select, SelectContent, SelectItem, SelectTrigger, SelectValue", async () => {
    const module = await import("@/components/ui/select");
    expect(module.Select).toBeDefined();
    expect(module.SelectContent).toBeDefined();
    expect(module.SelectItem).toBeDefined();
    expect(module.SelectTrigger).toBeDefined();
    expect(module.SelectValue).toBeDefined();
  });

  it("Tabs exports Tabs, TabsContent, TabsList, TabsTrigger", async () => {
    const module = await import("@/components/ui/tabs");
    expect(module.Tabs).toBeDefined();
    expect(module.TabsContent).toBeDefined();
    expect(module.TabsList).toBeDefined();
    expect(module.TabsTrigger).toBeDefined();
  });

  it("Table exports Table, TableBody, TableCell, TableHead, TableHeader, TableRow", async () => {
    const module = await import("@/components/ui/table");
    expect(module.Table).toBeDefined();
    expect(module.TableBody).toBeDefined();
    expect(module.TableCell).toBeDefined();
    expect(module.TableHead).toBeDefined();
    expect(module.TableHeader).toBeDefined();
    expect(module.TableRow).toBeDefined();
  });

  it("Toaster is importable", async () => {
    const module = await import("@/components/ui/toaster");
    expect(module.Toaster).toBeDefined();
    expect(module.Toaster).toBeTruthy();
  });
});
