/**
 * Unit tests for lib/hooks/use-api.ts — matchesApiPrefix utility.
 *
 * Covers: matchesApiPrefix — exact matches, query-string variants, nested
 *         subpaths, sibling-prefix false positives, and non-string key guard.
 * Does NOT cover: useApi SWR hook (requires a running SWR context + fetch
 *                 mock), apiFetcher network behaviour, or revalidateApiPrefixes.
 * Protects: apps/frontend/lib/hooks/use-api.ts
 *
 * Ported from: dps-dashboard/tests/unit/use-api.test.ts
 * Changes: Import path updated from @/lib/hooks/use-api (same in both repos).
 *          NEXT_PUBLIC_API_URL behaviour is unchanged — apiFetcher prepends
 *          the env var to every URL; matchesApiPrefix is pure logic with no
 *          network dependency.
 */

import { describe, expect, it } from "vitest";

import { matchesApiPrefix } from "@/lib/hooks/use-api";

describe("matchesApiPrefix", () => {
  it("matches exact API keys", () => {
    expect(matchesApiPrefix("/api/approvals", ["/api/approvals"])).toBe(true);
  });

  it("matches query-string variants", () => {
    expect(matchesApiPrefix("/api/approvals?page=2&status=PENDING", ["/api/approvals"])).toBe(true);
  });

  it("matches nested subpaths", () => {
    expect(matchesApiPrefix("/api/transactions/summary", ["/api/transactions"])).toBe(true);
  });

  it("does not overmatch sibling prefixes", () => {
    expect(matchesApiPrefix("/api/memberships", ["/api/members"])).toBe(false);
  });

  it("ignores non-string keys", () => {
    expect(matchesApiPrefix(["/api/approvals"], ["/api/approvals"])).toBe(false);
  });
});
