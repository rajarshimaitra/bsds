"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

/**
 * Fixed navigation bar for the landing page.
 * Transitions from transparent on hero to solid white/cool background on scroll.
 */
export default function NavBar() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 60);
    };
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  return (
    <header
      className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
        scrolled
          ? "border-b border-sky-100 bg-white/95 shadow-md backdrop-blur-sm"
          : "bg-transparent"
      }`}
    >
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Logo / Club name */}
          <Link
            href="/"
            className={`font-bold text-base sm:text-lg leading-tight transition-colors ${
              scrolled ? "text-slate-900" : "text-white"
            }`}
          >
            <span className="hidden sm:inline">Deshapriya Park Sarbojanin Durgotsav</span>
            <span className="sm:hidden">Deshapriya Park</span>
          </Link>

          {/* Desktop nav links */}
          <nav className="hidden md:flex items-center gap-6">
            {[
              { href: "#activities", label: "Activities" },
              { href: "#history", label: "History" },
              { href: "#news", label: "News" },
              { href: "#contact", label: "Contact" },
            ].map((link) => (
              <a
                key={link.href}
                href={link.href}
                className={`text-sm font-medium transition-colors hover:text-sky-500 ${
                  scrolled ? "text-slate-700" : "text-white/90"
                }`}
              >
                {link.label}
              </a>
            ))}
          </nav>

          {/* CTA buttons */}
          <div className="flex items-center gap-2 sm:gap-3">
            <Link
              href="/membership-form"
              className={`hidden sm:inline-flex items-center px-3 py-1.5 rounded-md text-sm font-medium border transition-colors ${
                scrolled
                  ? "border-sky-300 text-sky-700 hover:bg-sky-50"
                  : "border-white/70 text-white hover:bg-white/10"
              }`}
            >
              Apply for Membership
            </Link>
            <Link
              href="/login"
              className="inline-flex items-center px-4 py-1.5 rounded-md bg-slate-900 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-sky-600"
            >
              Login
            </Link>
          </div>
        </div>
      </div>
    </header>
  );
}
