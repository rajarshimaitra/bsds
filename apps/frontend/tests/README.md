# Frontend Test Suite

Tests for the BSDS Dashboard Next.js frontend (`apps/frontend`).

## Running the Tests

```bash
cd apps/frontend
npm test
```

> **Prerequisite:** `node_modules` must be installed. If `npm install` fails due to
> no network access, symlink the `dps-dashboard` node_modules directory:
> ```bash
> ln -sf ../../dps-dashboard/node_modules apps/frontend/node_modules
> ```

To run with verbose output:

```bash
npm test -- --reporter=verbose
```

To run a single file:

```bash
npm test -- tests/unit/utils.test.ts
```

---

## Test File Inventory

### `tests/unit/auth-client.test.ts`

**Purpose:** Unit tests for the `lib/auth-client.ts` HTTP auth module. All four
exported async functions are tested against a stubbed `fetch`.

**What it covers:**
- `login` — calls `POST /api/auth/login` with `username`/`password` in JSON
  body and `credentials: "include"`; returns `{ ok: true, mustChangePassword }`
  on 200; returns `{ ok: false, error }` on 401/403; falls back to
  `"Login failed"` when error body is missing
- `logout` — calls `POST /api/auth/logout` with `credentials: "include"`;
  resolves without throwing even on non-ok response
- `getMe` — calls `GET /api/auth/me` with `credentials: "include"`; returns
  the user object on 200; returns `null` on 401 or any non-ok response
- `changePassword` — calls `POST /api/auth/change-password` with
  `currentPassword`/`newPassword` in JSON body; returns `{ ok: true }` on 200;
  returns `{ ok: false, error }` on 400; falls back to
  `"Failed to change password"` when error body is missing

**What it does NOT cover:**
- Token refresh or session expiry behaviour
- Cookie management internals
- Any React component or hook that wraps auth-client

---

### `tests/unit/cash-fee-logic.test.ts`

**Purpose:** Unit tests for membership fee business constants and the pure
arithmetic used to calculate membership payment totals.

**What it covers:**
- `MEMBERSHIP_FEES` — pins MONTHLY (₹250), HALF_YEARLY (₹1500), ANNUAL (₹3000)
- `APPLICATION_FEE` — pins ₹10000
- `ANNUAL_MEMBERSHIP_FEE` — pins ₹5000
- Fee total combinations: subscription-only, subscription + application fee,
  subscription + annual membership fee, all three together
- `formatCurrency` applied to each business fee amount (display format check)

**What it does NOT cover:**
- Razorpay integration, payment processing, or API endpoints
- React component rendering of the cash page
- Any conditional fee-waivers or discount logic

---

### `tests/unit/approval-constants.test.ts`

**Purpose:** Unit tests for all constants and pure functions exported from
`app/dashboard/approvals/constants.ts`.

**What it covers:**
- `ENTITY_TYPE_LABELS` — all 5 backend entity types have labels; exact count
  check prevents silent addition of unmapped types
- `STATUS_CLASSES` — all 3 approval statuses (PENDING, APPROVED, REJECTED) have
  CSS class strings
- `ENTITY_TYPE_COLORS` — all 5 entity types have color classes
- `ACTION_LABELS` — 7 key action labels verified
- `FIELD_LABELS` — 5 key field labels verified
- `ENUM_LABELS` — approvalStatus, type (CASH_IN/CASH_OUT), paymentMode,
  category enum label resolution
- `entityApiUrl` — all routing branches: TRANSACTION, MEMBER_EDIT (normal +
  sub-member), MEMBER_DELETE (normal + sub-member), MEMBERSHIP
  (approve_membership + other), unknown type, placeholder ID
- `formatFieldValue` — null/undefined/empty → "—"; enum lookup; amount
  formatting with CASH_IN/CASH_OUT color classes; date fields; memberId
  truncation; plain string passthrough
- `formatDate` (local to constants) — formats ISO date string with en-IN locale

**What it does NOT cover:**
- React component rendering of the approvals page
- Network calls or SWR data fetching
- The `ApprovalDetail`, `DetailRow`, `DiffRow`, `LiveEntityView`,
  `MemberLiveSection` components

---

### `tests/components/layout.test.tsx`

**Purpose:** Structural smoke tests for all layout, provider, landing, receipt,
and shadcn/ui primitive components.

**What it covers:**
- `components/layout/Sidebar`, `Header`, `DashboardShell` — each exports a
  default function component
- `components/providers/AuthProvider` — exports both `AuthProvider` and
  `useAuthContext` as named exports
- `components/landing/NavBar`, `PrintButton` — each exports a default function
- `components/receipts/ReceiptView` — exports a default function
- `components/ui/*` — Button, Card (all named exports), Input, Label, Badge,
  Separator, Dialog, Select, Tabs, Table, Toaster

**What it does NOT cover:**
- Rendered HTML output or visual appearance
- Click interactions, navigation, or form submission
- Mobile/responsive layout behaviour
- Accessibility tree structure

**Ported from:** `dps-dashboard/tests/components/layout.test.ts`
**Auth changes:** `SessionProvider` reference replaced with `AuthProvider`.

---

### `tests/components/pages.test.tsx`

**Purpose:** Structural smoke tests ensuring every Next.js App Router page
component can be imported without a runtime error and exports a default
function.

**What it covers:**
- Root pages: `/`, `/login`, `/change-password`, `/membership-form`
- Sponsor pages: `/sponsor/[token]`, `/sponsor/[token]/receipt`
- Dashboard pages: `/dashboard`, `/dashboard/members`, `/dashboard/cash`,
  `/dashboard/approvals`, `/dashboard/sponsorship`, `/dashboard/my-membership`,
  `/dashboard/audit-log`, `/dashboard/activity-log`
- App layouts: `/app/dashboard/layout`

**What it does NOT cover:**
- Rendered output, data fetching, or API calls
- Auth guard redirect behaviour (requires full Next.js runtime)
- Root layout (`app/layout.tsx`) — skipped because it uses `next/font/local`,
  a build-time-only API not available in the Vitest jsdom environment

**Ported from:** `dps-dashboard/tests/components/pages.test.ts`
**Auth changes:** `useSession`/`SessionProvider` mocks replaced with `useAuth`/
`AuthProvider` mocks so that client components calling `useAuth()` can be
imported without throwing "must be used within AuthProvider".

---

### `tests/unit/use-api.test.ts`

**Purpose:** Unit tests for the `matchesApiPrefix` pure utility exported from
`lib/hooks/use-api.ts`.

**What it covers:**
- Exact key match
- Query-string variants (`/api/approvals?page=2`)
- Nested subpath match (`/api/transactions/summary`)
- Sibling-prefix false positive prevention (`/api/memberships` must not match
  `/api/members`)
- Non-string key guard (arrays are rejected)

**What it does NOT cover:**
- `useApi` SWR hook (requires a running SWR context and fetch mock)
- `apiFetcher` network behaviour (requires a mocked `fetch`)
- `revalidateApiPrefixes` (requires a live SWR cache)

**Ported from:** `dps-dashboard/tests/unit/use-api.test.ts`
**Changes:** Import path is identical (`@/lib/hooks/use-api`). No auth
dependency. `NEXT_PUBLIC_API_URL` prepending behaviour is not tested because it
is a runtime fetch concern, not pure logic.

---

### `tests/unit/utils.test.ts`

**Purpose:** Unit tests for all formatting utilities in `lib/utils.ts`.

**What it covers:**
- `formatCurrency` — whole numbers, decimals, strings, NaN, zero
- `formatDate` — ISO strings, null, undefined, invalid strings, Date objects
- `formatDateTime` — ISO strings with time, null, invalid
- `formatPhone` — +91 passthrough, 12-digit, 10-digit, unknown, empty
- `formatMemberId` — passthrough identity
- `formatSponsorPurpose` — TITLE_SPONSOR, FOOD_PARTNER, MARKETING_PARTNER
- `formatMembershipType` — MONTHLY, HALF_YEARLY, ANNUAL, null, undefined
- `formatMembershipStatus` — PENDING_APPROVAL, ACTIVE, PENDING_PAYMENT, EXPIRED

**What it does NOT cover:**
- `cn()` class-merging (see `utils-extended.test.ts`)
- Any React component logic

**Ported from:** `dps-dashboard/tests/unit/utils.test.ts`
**Changes:** None — import path and assertions are identical.

---

### `tests/unit/utils-extended.test.ts`

**Purpose:** Additional edge-case coverage for `lib/utils.ts`, including the
`cn()` Tailwind class-merging utility.

**What it covers:**
- `cn()` — empty args, merge, deduplication (tailwind-merge), falsy conditionals,
  object syntax, array syntax
- `formatCurrency` — business-domain amounts (₹250, ₹1500, ₹3000, ₹10000),
  negative, very large, string decimals, string "0"
- `formatDate` — January 1st, December 31st, Date objects, empty string, 0
- `formatDateTime` — midnight (00:00), 23:59, single-digit hour padding (09:05),
  undefined
- `formatPhone` — +91 with spaces, 91 with spaces (passthrough), leading-zero
  10-digit, typical 10-digit
- `formatMembershipType` — unknown fallback, empty string, all three valid types
- `formatMembershipStatus` — all five known membership statuses,
  multi-underscore status, and the three approval status strings
  (PENDING, APPROVED, REJECTED) via the same `toTitleCase` mechanism
- `formatSponsorPurpose` — all seven sponsor purpose enum values
- `formatFeeType` — ANNUAL_FEE, SUBSCRIPTION, null/undefined/empty guard,
  unknown fallback passthrough

**What it does NOT cover:**
- React rendering, hooks, network calls, or auth logic

**Ported from:** `dps-dashboard/tests/unit/utils-extended.test.ts`
**Wave 7E additions:** `formatFeeType` tests; approval status string coverage.

---

## Shared Setup (`tests/setup.ts`)

Runs before every test file. Provides vi.mock stubs for:

| Module | Stub behaviour |
|---|---|
| `next/navigation` | `useRouter`, `usePathname`, `useSearchParams`, `redirect` all no-ops |
| `next/image` | Plain passthrough (no Next.js image pipeline) |
| `next/font/local` | Returns `{ className, variable }` literals |
| `next/font/google` | Returns `{ className, variable }` literals |
| `next/link` | Passthrough |
| `next/headers` | `cookies()` / `headers()` no-ops |

Auth mocks (`useAuth`, `AuthProvider`) are declared per-file in
`pages.test.tsx` rather than in setup.ts, so each test file is
self-documenting about the auth shape it depends on.

---

## Known Gaps

| Area | Status | Reason |
|---|---|---|
| Root layout (`app/layout.tsx`) | Skipped | Uses `next/font/local` — build-time API only |
| `useApi` / `apiFetcher` | Not tested | Requires SWR context + fetch mock; deferred to integration tests |
| `revalidateApiPrefixes` | Not tested | Requires a live SWR cache |
| Rendered component output | Not tested | Requires a full Next.js render environment with mocked navigation |
| Dashboard page data fetching | Not tested | Server/API concern; covered by backend API tests |
| Auth guard redirects | Not tested | Requires Next.js middleware runtime |
| Mobile responsive layout | Not tested | Requires browser-level resize simulation |
| `ApprovalDetail` component render | Not tested | Needs full React render tree with mock props; deferred |
| `api-client.ts` (apiGet/apiPost/apiPatch/apiDelete) | Not tested | Thin wrappers over fetch; covered by auth-client pattern; deferred |

---

## Architecture Notes

- Test framework: **Vitest 3.x** with `jsdom` environment
- Config: `apps/frontend/vitest.config.ts`
- Path alias: `@/` maps to `apps/frontend/` (mirrors tsconfig paths)
- All tests are structural (import/export checks) or pure-logic unit tests —
  no rendering, no network, no database
- Wave 7E added 3 new test files: `auth-client.test.ts`,
  `cash-fee-logic.test.ts`, `approval-constants.test.ts`; and extended
  `utils-extended.test.ts` with `formatFeeType` and approval status coverage
- Total: **216 tests passing, 1 skipped** (8 test files)
