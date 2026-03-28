/**
 * Vitest global test setup.
 *
 * Runs before every test file. Provides:
 *   - A stub for next/navigation (useRouter, usePathname, useSearchParams)
 *     so that page and layout components can be imported without throwing.
 *   - A stub for next/image so <Image> renders without the Next.js pipeline.
 *   - A no-op for next/font/local (root layout uses localFont which is a
 *     build-time API only available inside the Next.js compiler).
 *
 * Auth mocks are intentionally NOT placed here — each test file that needs
 * them declares its own vi.mock() for @/hooks/use-auth so that tests remain
 * self-contained and the mock shape is explicit at the call site.
 */

import { vi } from "vitest";

// ---------------------------------------------------------------------------
// next/navigation stub
// ---------------------------------------------------------------------------
vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
    prefetch: vi.fn(),
  }),
  usePathname: () => "/",
  useSearchParams: () => new URLSearchParams(),
  redirect: vi.fn(),
}));

// ---------------------------------------------------------------------------
// next/image stub — renders a plain <img> to avoid the Next.js Image pipeline
// ---------------------------------------------------------------------------
vi.mock("next/image", () => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  default: (props: any) => {
    // Return null-ish — structural tests don't render, so this only matters
    // for future rendering tests.
    return props;
  },
}));

// ---------------------------------------------------------------------------
// next/font/local stub — root layout uses localFont (build-time only)
// ---------------------------------------------------------------------------
vi.mock("next/font/local", () => ({
  default: () => ({ className: "mock-font", variable: "--mock-font" }),
}));

// ---------------------------------------------------------------------------
// next/font/google stub
// ---------------------------------------------------------------------------
vi.mock("next/font/google", () => ({
  Inter: () => ({ className: "mock-inter", variable: "--mock-inter" }),
  Geist: () => ({ className: "mock-geist", variable: "--mock-geist" }),
  Geist_Mono: () => ({
    className: "mock-geist-mono",
    variable: "--mock-geist-mono",
  }),
}));

// ---------------------------------------------------------------------------
// next/link stub — renders children directly in jsdom environment
// ---------------------------------------------------------------------------
vi.mock("next/link", () => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  default: ({ children, href, ...rest }: any) => {
    // Return a minimal proxy — structural tests only check exports, not render
    return { children, href, ...rest };
  },
}));

// ---------------------------------------------------------------------------
// next/headers stub — used by server components for cookies()/headers()
// ---------------------------------------------------------------------------
vi.mock("next/headers", () => ({
  cookies: () => ({
    get: vi.fn(() => undefined),
    getAll: vi.fn(() => []),
  }),
  headers: () => new Headers(),
}));
