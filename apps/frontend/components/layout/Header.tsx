"use client";

import { useState } from "react";
import { Menu } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetTitle } from "@/components/ui/sheet";
import Sidebar from "@/components/layout/Sidebar";
import type { Role } from "@/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface HeaderProps {
  user: {
    name: string;
    role: Role;
    memberId: string;
  };
}

/**
 * Dashboard top header bar.
 *
 * - Mobile: hamburger button that opens a Sheet overlay containing the Sidebar
 * - Desktop: no top-right profile menu; logout lives in the sidebar footer
 *
 * @param user - Signed-in user's name, role, and memberId
 */
export default function Header({ user }: HeaderProps) {
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <>
      <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
        <SheetContent
          side="left"
          className="w-[86vw] max-w-72 border-0 bg-transparent p-0 shadow-none"
        >
          <SheetTitle className="sr-only">Navigation Menu</SheetTitle>
          <Sidebar user={user} onNavigate={() => setMobileOpen(false)} />
        </SheetContent>
      </Sheet>

      <div className="fixed left-4 top-4 z-40 lg:hidden">
        <Button
          variant="ghost"
          size="icon"
          className="rounded-2xl border border-white/70 bg-white/80 shadow-[0_20px_50px_-30px_rgba(15,23,42,0.55)] backdrop-blur-xl"
          onClick={() => setMobileOpen(true)}
          aria-label="Open navigation menu"
        >
          <Menu className="h-5 w-5" />
        </Button>
      </div>

    </>
  );
}
