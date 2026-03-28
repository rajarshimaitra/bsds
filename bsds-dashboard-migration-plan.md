# BSDS Dashboard Restructure Plan

## Summary

This project is `bsds-dashboard` which is restructuring of the parent project [`dps-dashboard`](../dps-dashboard/) monolith into:

- a frontend-only Next.js + TypeScript app
- a separate Rust backend
- a SQLite database

Guiding rule:

- Copy first
- Modify only where necessary
- Do not refactor for style or architecture unless the split requires it

Required parity:

- exact UI
- same feature set
- same business logic behavior
- same seed strategy
- same testing intent
- same logical schema and data semantics
- clean frontend/backend separation over HTTP
- retire the `dps-dashboard` name from the new project and use `bsds-dashboard` for the repo, app, docs, and package naming

Chosen backend stack:

- Axum for HTTP
- SQLx for SQLite access
- tokio-cron-scheduler for in-process scheduled jobs

Chosen backend dependency set:

### Runtime dependencies

- `axum`
- `tokio`
- `tower-http`
- `serde`
- `serde_json`
- `chrono`
- `uuid`
- `sqlx`
- `bcrypt`
- `rand`
- `aes-gcm`
- `base64`
- `hmac`
- `sha2`
- `hex`
- `reqwest`
- `tokio-cron-scheduler`
- `tracing`
- `tracing-subscriber`
- `thiserror`
- `dotenvy`

### Dev/test dependencies

- `tempfile`
- `axum-test`
- `tokio-test`
- `pretty_assertions`

## Current Stack Analysis

The current project is a Next.js 14 monolith with these main pieces:

- frontend pages and components in `src/app/**` and `src/components/**`
- API route handlers in `src/app/api/**`
- business logic in `src/lib/services/**`
- auth via `next-auth`
- persistence via Prisma + PostgreSQL
- shared TS types and validators in `src/types/**` and `src/lib/validators.ts`
- rich seed data in `prisma/seed.ts`
- Vitest-based tests across component, route, and business logic layers

Important migration observations:

- most dashboard pages are already client-side and fetch-driven
- much of the UI can be copied with little or no change
- most required change is concentrated at:
  - auth/session handling
  - API routing layer
  - database access layer
  - server-only helpers

## Implementation Changes

### 1. Repository Structure

Create the new `bsds-dashboard` project with this target structure:

- `apps/frontend`
- `apps/backend`

Frontend owns:

- pages
- components
- styling
- UI-only client state such as dialog visibility, form inputs before submit, loading flags, selected rows, pagination controls, and local view filters
- auth UI
- HTTP calls to backend

Backend owns:

- authentication
- cookies/sessions
- request validation
- business logic
- database access
- scheduled jobs
- external integrations
- receipts and reporting APIs
- all business/domain state including members, memberships, transactions, approvals, audit logs, activity logs, dashboard stats, and payment outcomes

Planned new `bsds-dashboard` layout:

- `apps/frontend/app/**` for routes/pages copied from current `src/app/**`
- `apps/frontend/components/**` for copied UI/component modules from current `src/components/**`
- `apps/frontend/lib/**` for frontend-safe utilities, hooks, auth client, and API client
- `apps/frontend/public/**` for static assets copied from current `public/**`
- `apps/backend/src/main.rs` for backend bootstrap
- `apps/backend/src/routes/**` for HTTP handlers matching current `/api/*` groups
- `apps/backend/src/services/**` for ported business rules
- `apps/backend/src/repositories/**` for SQLx query modules
- `apps/backend/src/auth/**` for login/session/password-change logic
- `apps/backend/src/support/**` for member IDs, encryption, rate limits, receipts, shared helpers
- `apps/backend/src/integrations/**` for Razorpay and WhatsApp clients
- `apps/backend/src/scheduler/**` for cron job registration and task runners
- `apps/backend/migrations/**` for SQLite schema
- `apps/backend/src/bin/seed.rs` or equivalent seed entrypoint
- `sqlite/` for the local SQLite database file, kept inside the repo directory but ignored by git

Current source of truth to migrate from:

- frontend routes/pages: `../dps-dashboard/src/app/**`
- frontend components: `../dps-dashboard/src/components/**`
- backend handlers: `../dps-dashboard/src/app/api/**`
- backend services/helpers: `../dps-dashboard/src/lib/**`
- schema/seed: `../dps-dashboard/prisma/schema.prisma`, `../dps-dashboard/prisma/seed.ts`
- tests: `../dps-dashboard/tests/**`

Naming rule for the new project:

- the new repository directory is `./bsds-dashboard`
- the git repository name is `bsds-dashboard`
- new package, app, and documentation references should use `bsds-dashboard`
- `dps-dashboard` remains only as the legacy source reference during migration

### 2. Frontend Migration

Copy into `apps/frontend` with no modification unless required:

- `src/app/**` page files except `src/app/api/**`
- `src/components/**`
- `public/**`
- Tailwind/shadcn config and styles
- frontend-friendly utility code

Modify only these areas:

- replace `next-auth` usage with a small frontend auth client/provider
- replace middleware/session-provider coupling
- add configurable backend API base URL
- update any file importing server-only code
- preserve route URLs, visual structure, and user flows
- keep backend as the single source of truth for business data; frontend caches may exist for UX, but mutations must re-fetch or invalidate server data rather than treating client cache as authoritative

Frontend auth contract:

- login page calls backend `POST /api/auth/login`
- logout calls backend `POST /api/auth/logout`
- session load uses backend `GET /api/auth/me`
- change password uses backend `POST /api/auth/change-password`

Frontend route protection:

- unauthenticated user -> `/login`
- temp-password user -> `/change-password`

Current files to copy with minimal or no change:

- `../dps-dashboard/src/app/page.tsx` -> `apps/frontend/app/page.tsx`
- `../dps-dashboard/src/app/login/page.tsx` -> `apps/frontend/app/login/page.tsx`
- `../dps-dashboard/src/app/change-password/page.tsx` -> `apps/frontend/app/change-password/page.tsx`
- `../dps-dashboard/src/app/membership-form/page.tsx` -> `apps/frontend/app/membership-form/page.tsx`
- `../dps-dashboard/src/app/print/receipt/page.tsx` -> `apps/frontend/app/print/receipt/page.tsx`
- `../dps-dashboard/src/app/sponsor/[token]/page.tsx` -> `apps/frontend/app/sponsor/[token]/page.tsx`
- `../dps-dashboard/src/app/sponsor/[token]/receipt/page.tsx` -> `apps/frontend/app/sponsor/[token]/receipt/page.tsx`
- `../dps-dashboard/src/app/dashboard/page.tsx` -> `apps/frontend/app/dashboard/page.tsx`
- `../dps-dashboard/src/app/dashboard/members/page.tsx` -> `apps/frontend/app/dashboard/members/page.tsx`
- `../dps-dashboard/src/app/dashboard/cash/page.tsx` -> `apps/frontend/app/dashboard/cash/page.tsx`
- `../dps-dashboard/src/app/dashboard/sponsorship/page.tsx` -> `apps/frontend/app/dashboard/sponsorship/page.tsx`
- `../dps-dashboard/src/app/dashboard/approvals/page.tsx` -> `apps/frontend/app/dashboard/approvals/page.tsx`
- `../dps-dashboard/src/app/dashboard/approvals/approval-detail.tsx` -> `apps/frontend/app/dashboard/approvals/approval-detail.tsx`
- `../dps-dashboard/src/app/dashboard/approvals/components.tsx` -> `apps/frontend/app/dashboard/approvals/components.tsx`
- `../dps-dashboard/src/app/dashboard/activity-log/page.tsx` -> `apps/frontend/app/dashboard/activity-log/page.tsx`
- `../dps-dashboard/src/app/dashboard/audit-log/page.tsx` -> `apps/frontend/app/dashboard/audit-log/page.tsx`
- `../dps-dashboard/src/app/dashboard/my-membership/page.tsx` -> `apps/frontend/app/dashboard/my-membership/page.tsx`
- `../dps-dashboard/src/app/layout.tsx` -> `apps/frontend/app/layout.tsx`
- `../dps-dashboard/src/app/globals.css` -> `apps/frontend/app/globals.css`
- `../dps-dashboard/src/app/error.tsx`, `global-error.tsx`, loading files -> same relative locations in `apps/frontend/app/**`
- `../dps-dashboard/src/components/ui/**` -> `apps/frontend/components/ui/**`
- `../dps-dashboard/src/components/layout/**` -> `apps/frontend/components/layout/**`
- `../dps-dashboard/src/components/landing/**` -> `apps/frontend/components/landing/**`
- `../dps-dashboard/src/components/members/member-page-helpers.ts` -> `apps/frontend/components/members/member-page-helpers.ts`
- `../dps-dashboard/src/components/receipts/ReceiptView.tsx` -> `apps/frontend/components/receipts/ReceiptView.tsx`
- `../dps-dashboard/public/**` -> `apps/frontend/public/**`

Current frontend files that require modification:

- `../dps-dashboard/src/components/providers/SessionProvider.tsx`
  - replace with `apps/frontend/components/providers/AuthProvider.tsx`
  - remove `next-auth/react`
  - load session via backend `/api/auth/me`
- `../dps-dashboard/src/app/layout.tsx`
  - update provider wiring from SessionProvider to AuthProvider
- `../dps-dashboard/src/app/login/page.tsx`
  - replace `signIn("credentials")` with backend login fetch
- `../dps-dashboard/src/app/change-password/page.tsx`
  - replace `signOut` and NextAuth refresh behavior with backend logout/session refresh
- `../dps-dashboard/src/app/dashboard/layout.tsx`
  - replace `getServerSession` usage with frontend auth guard pattern
- `../dps-dashboard/src/components/layout/Sidebar.tsx`
  - replace `signOut` with backend logout call
- `../dps-dashboard/src/lib/hooks/use-api.ts`
  - preserve SWR behavior
  - add centralized backend base URL handling
  - keep `credentials: "include"`
- page files using `useSession` from `next-auth/react`
  - current sites:
    - `../dps-dashboard/src/app/dashboard/page.tsx`
    - `../dps-dashboard/src/app/dashboard/members/page.tsx`
    - `../dps-dashboard/src/app/dashboard/cash/page.tsx`
    - `../dps-dashboard/src/app/dashboard/sponsorship/page.tsx`
    - `../dps-dashboard/src/app/dashboard/approvals/page.tsx`
    - `../dps-dashboard/src/app/dashboard/activity-log/page.tsx`
    - `../dps-dashboard/src/app/dashboard/audit-log/page.tsx`
    - `../dps-dashboard/src/app/dashboard/my-membership/page.tsx`
  - replace with `useAuth`
- `../dps-dashboard/src/middleware.ts`
  - do not copy as-is
  - replace with frontend-side route protection strategy in `apps/frontend`
- `../dps-dashboard/src/types/index.ts` and `src/types/next-auth.d.ts`
  - keep domain types
  - remove NextAuth augmentation from the new frontend types

Frontend-only additions to create:

- `apps/frontend/lib/auth-client.ts`
- `apps/frontend/lib/api-client.ts`
- `apps/frontend/lib/auth-types.ts`
- `apps/frontend/components/providers/AuthProvider.tsx`
- `apps/frontend/hooks/use-auth.ts` or `apps/frontend/lib/hooks/use-auth.ts`
- optional route-guard wrapper/component in `apps/frontend/components/auth/**`

### 3. Backend Migration

Build a Rust backend in `apps/backend` with these modules:

- `auth`
- `routes`
- `services`
- `repositories`
- `db`
- `scheduler`
- `payments`
- `notifications`
- `receipts`
- `support`

Preserve the current API surface wherever feasible:

- `/api/members`
- `/api/memberships`
- `/api/transactions`
- `/api/approvals`
- `/api/sponsors`
- `/api/sponsor-links`
- `/api/dashboard/stats`
- `/api/audit-log`
- `/api/activity-log`
- `/api/receipts`
- `/api/payments/*`
- `/api/webhooks/razorpay`
- `/api/cron` only if retained as a manual/internal trigger surface

Auth-specific change:

- replace NextAuth endpoints with:
  - `POST /api/auth/login`
  - `POST /api/auth/logout`
  - `GET /api/auth/me`
  - `POST /api/auth/change-password`

Current route-handler sources to port:

- `../dps-dashboard/src/app/api/auth/[...nextauth]/route.ts`
- `../dps-dashboard/src/app/api/auth/change-password/route.ts`
- `../dps-dashboard/src/app/api/members/route.ts`
- `../dps-dashboard/src/app/api/members/[id]/route.ts`
- `../dps-dashboard/src/app/api/memberships/route.ts`
- `../dps-dashboard/src/app/api/memberships/[id]/route.ts`
- `../dps-dashboard/src/app/api/my-membership/route.ts`
- `../dps-dashboard/src/app/api/transactions/route.ts`
- `../dps-dashboard/src/app/api/transactions/[id]/route.ts`
- `../dps-dashboard/src/app/api/transactions/summary/route.ts`
- `../dps-dashboard/src/app/api/approvals/route.ts`
- `../dps-dashboard/src/app/api/audit-log/route.ts`
- `../dps-dashboard/src/app/api/activity-log/route.ts`
- `../dps-dashboard/src/app/api/dashboard/stats/route.ts`
- `../dps-dashboard/src/app/api/sponsors/route.ts`
- `../dps-dashboard/src/app/api/sponsors/[id]/route.ts`
- `../dps-dashboard/src/app/api/sponsor-links/route.ts`
- `../dps-dashboard/src/app/api/sponsor-links/[token]/route.ts`
- `../dps-dashboard/src/app/api/receipts/[id]/route.ts`
- `../dps-dashboard/src/app/api/payments/create-order/route.ts`
- `../dps-dashboard/src/app/api/payments/verify/route.ts`
- `../dps-dashboard/src/app/api/payments/sponsor-order/route.ts`
- `../dps-dashboard/src/app/api/payments/sponsor-verify/route.ts`
- `../dps-dashboard/src/app/api/notifications/whatsapp/route.ts`
- `../dps-dashboard/src/app/api/webhooks/razorpay/route.ts`
- `../dps-dashboard/src/app/api/cron/route.ts`

Current service/helper sources to port:

- `../dps-dashboard/src/lib/services/member-service.ts` -> `apps/backend/src/services/member_service.rs`
- `../dps-dashboard/src/lib/services/membership-service.ts` -> `apps/backend/src/services/membership_service.rs`
- `../dps-dashboard/src/lib/services/transaction-service.ts` -> `apps/backend/src/services/transaction_service.rs`
- `../dps-dashboard/src/lib/services/approval-service.ts` -> `apps/backend/src/services/approval_service.rs`
- `../dps-dashboard/src/lib/services/sponsor-service.ts` -> `apps/backend/src/services/sponsor_service.rs`
- `../dps-dashboard/src/lib/services/notification-service.ts` -> `apps/backend/src/services/notification_service.rs`
- `../dps-dashboard/src/lib/services/webhook-sponsor-handler.ts` -> `apps/backend/src/services/webhook_sponsor_handler.rs`
- `../dps-dashboard/src/lib/auth.ts` -> `apps/backend/src/auth/mod.rs`
- `../dps-dashboard/src/lib/permissions.ts` -> `apps/backend/src/auth/permissions.rs`
- `../dps-dashboard/src/lib/audit.ts` -> `apps/backend/src/support/audit.rs`
- `../dps-dashboard/src/lib/cron.ts` -> `apps/backend/src/scheduler/cron_jobs.rs`
- `../dps-dashboard/src/lib/member-id.ts` -> `apps/backend/src/support/member_id.rs`
- `../dps-dashboard/src/lib/membership-rules.ts` -> `apps/backend/src/support/membership_rules.rs`
- `../dps-dashboard/src/lib/encrypt.ts` -> `apps/backend/src/support/encrypt.rs`
- `../dps-dashboard/src/lib/rate-limit.ts` -> `apps/backend/src/support/rate_limit.rs`
- `../dps-dashboard/src/lib/razorpay.ts` -> `apps/backend/src/integrations/razorpay.rs`
- `../dps-dashboard/src/lib/whatsapp.ts` -> `apps/backend/src/integrations/whatsapp.rs`
- `../dps-dashboard/src/lib/receipt.ts` and `src/lib/receipt-utils.ts` -> `apps/backend/src/support/receipt.rs`
- `../dps-dashboard/src/lib/validators.ts` -> split across request DTOs in `apps/backend/src/routes/**` and validation helpers in `apps/backend/src/support/validation.rs`
- `../dps-dashboard/src/lib/temp-password.ts` -> `apps/backend/src/auth/temp_password.rs`
- `../dps-dashboard/src/lib/prisma.ts`
  - do not port directly
  - replace with SQLx pool/bootstrap in `apps/backend/src/db/mod.rs`
  - move encryption-at-rest behavior into repository/support layer

Planned backend route layout:

- `apps/backend/src/routes/auth.rs`
- `apps/backend/src/routes/members.rs`
- `apps/backend/src/routes/memberships.rs`
- `apps/backend/src/routes/my_membership.rs`
- `apps/backend/src/routes/transactions.rs`
- `apps/backend/src/routes/approvals.rs`
- `apps/backend/src/routes/audit_log.rs`
- `apps/backend/src/routes/activity_log.rs`
- `apps/backend/src/routes/dashboard.rs`
- `apps/backend/src/routes/sponsors.rs`
- `apps/backend/src/routes/sponsor_links.rs`
- `apps/backend/src/routes/receipts.rs`
- `apps/backend/src/routes/payments.rs`
- `apps/backend/src/routes/webhooks.rs`
- `apps/backend/src/routes/cron.rs`

### 4. Database and Schema

Port the existing logical schema to SQLite with minimal adaptation:

- preserve table/entity set
- preserve field meaning
- preserve relationships
- preserve approval/audit/activity semantics
- preserve member ID format and generation behavior
- preserve encrypted PII behavior
- preserve seed data semantics

SQLite adaptations only where necessary:

- enums -> text + Rust validation/check constraints
- UUIDs -> text
- JSON snapshots -> text JSON
- dates/timestamps -> ISO strings or SQLite-compatible datetime representation
- decimal values -> SQLite numeric representation

No redesign unless SQLite forces a narrow compatibility change.

SQLite file location rule:

- keep the live SQLite database inside the local project repository for simple local development
- store it under `./sqlite/bsds-dashboard.sqlite3` (path is hardcoded in the backend binary)
- do not place the working database in a global temp directory or outside the project tree
- do not commit the SQLite database, WAL file, or SHM file to git

Current schema and data sources:

- `dps-dashboard/prisma/schema.prisma`
- `dps-dashboard/prisma/migrations/**`
- `dps-dashboard/prisma/seed.ts`

Schema review targets in the current codebase:

- Type definitions mirrored in `dps-dashboard/src/types/index.ts`
- validation rules in `dps-dashboard/src/lib/validators.ts`
- business invariants in:
  - `dps-dashboard/src/lib/member-id.ts`
  - `dps-dashboard/src/lib/membership-rules.ts`
  - `dps-dashboard/src/lib/services/member-service.ts`
  - `dps-dashboard/src/lib/services/membership-service.ts`
  - `dps-dashboard/src/lib/services/transaction-service.ts`
  - `dps-dashboard/src/lib/services/approval-service.ts`

Planned backend DB locations:

- `apps/backend/migrations/0001_initial.sql`
- subsequent migrations in `apps/backend/migrations/**`
- `apps/backend/src/db/mod.rs`
- `apps/backend/src/db/models.rs`
- `apps/backend/src/repositories/users.rs`
- `apps/backend/src/repositories/members.rs`
- `apps/backend/src/repositories/memberships.rs`
- `apps/backend/src/repositories/transactions.rs`
- `apps/backend/src/repositories/approvals.rs`
- `apps/backend/src/repositories/sponsors.rs`
- `apps/backend/src/repositories/sponsor_links.rs`
- `apps/backend/src/repositories/audit_logs.rs`
- `apps/backend/src/repositories/activity_logs.rs`
- `apps/backend/src/repositories/receipts.rs`

### 5. Scheduler / Cron

Keep cron because feature parity requires time-based behavior.

Use:

- `tokio-cron-scheduler`

Scheduled responsibilities:

- membership expiry reminders
- automatic expiry transitions
- any equivalent current timed housekeeping

Rule:

- scheduled jobs must call normal service-layer logic
- do not bury business rules inside scheduler-specific code

Current cron logic source:

- `dps-dashboard/src/lib/cron.ts`
- HTTP trigger wrapper: `dps-dashboard/src/app/api/cron/route.ts`

Planned backend locations:

- `apps/backend/src/scheduler/mod.rs`
- `apps/backend/src/scheduler/cron_jobs.rs`
- `apps/backend/src/routes/cron.rs` only if a manual trigger/debug endpoint is retained

Expected implementation split:

- scheduler registration and schedule strings in `apps/backend/src/scheduler/**`
- business actions delegated into service modules such as:
  - `apps/backend/src/services/membership_service.rs`
  - `apps/backend/src/services/notification_service.rs`

### 6. Seed Strategy

Port `prisma/seed.ts` into a Rust seed command.

Preserve:

- same account set
- same passwords
- same roles
- same representative members/sub-members
- same approvals coverage
- same transaction coverage
- same sponsor/sponsor-link coverage
- same audit/activity coverage

The seed remains rerunnable.

Current seed source:

- `dps-dashboard/prisma/seed.ts`

Related current code that informs seed correctness:

- `dps-dashboard/docs/testing-guide.md`
- `dps-dashboard/src/lib/encrypt.ts`
- `dps-dashboard/src/lib/member-id.ts`
- `dps-dashboard/src/lib/services/**`

Planned new locations:

- `apps/backend/src/bin/seed.rs`
- optional helper modules:
  - `apps/backend/src/seed/mod.rs`
  - `apps/backend/src/seed/builders.rs`
  - `apps/backend/src/seed/fixtures.rs`

### 7. Testing Strategy

#### Frontend

Keep and adapt current frontend tests:

- component render tests
- page render tests
- frontend utility tests

Only modify test plumbing where needed for:

- new auth provider mocks
- API base URL handling
- removal of `next-auth` mocks

Frontend test documentation requirements:

- every test file must start with a short module header explaining:
  - what feature area it covers
  - what is intentionally not covered there
  - which source module or page it protects
- each `describe` block must read like a feature checklist, not a vague grouping
- each test case name must describe the behavior and expected result in plain language
- test helpers must be centralized and named by business intent rather than UI mechanics
- if a page or component has notable edge cases, the file header must list them explicitly
- the frontend test suite must have a short companion index document at `apps/frontend/tests/README.md` summarizing:
  - test file inventory
  - purpose of each file
  - major covered flows
  - known gaps, if any

Current frontend-oriented tests to carry over:

- `dps-dashboard/tests/components/layout.test.ts`
- `dps-dashboard/tests/components/pages.test.ts`
- `dps-dashboard/tests/unit/use-api.test.ts`
- `dps-dashboard/tests/unit/utils.test.ts`
- `dps-dashboard/tests/unit/utils-extended.test.ts`

Current tests that need targeted frontend auth updates:

- `dps-dashboard/tests/components/pages.test.ts`
- `dps-dashboard/tests/components/layout.test.ts`
- any test referencing `next-auth` mocks from:
  - `dps-dashboard/tests/unit/auth.test.ts`
  - `dps-dashboard/tests/unit/route-handlers-core.test.ts`
  - `dps-dashboard/tests/unit/route-handlers-extended.test.ts`

#### Backend

Create strong Rust backend tests for:

- auth flows
- member CRUD
- sub-member constraints
- membership rules
- transactions
- approvals
- sponsors and sponsor links
- audit logging
- activity logging
- receipt endpoints
- payment verification flows
- cron-driven expiry/reminder behavior

Testing approach:

- use `axum-test` for HTTP-level integration coverage
- use `tempfile` SQLite DBs for isolated test databases
- keep tests close to current behavior expectations, not new abstractions

Backend test documentation requirements:

- every backend test file must begin with a short doc comment or header explaining:
  - the API surface or service behavior under test
  - the scenarios covered
  - the invariants the file is meant to protect
- test files must be organized by business capability, not by helper or transport detail
- `describe`-equivalent grouping in Rust should mirror the user-visible feature or backend contract
- each test must have a precise name that makes pass/fail meaning obvious without opening the implementation
- shared test fixtures must be documented in a dedicated test support module so a reviewer can quickly understand:
  - what seed-like data exists
  - how auth/session setup works
  - how SQLite test databases are created
  - how external integrations are stubbed
- the backend test suite must include a high-level index document at `apps/backend/tests/README.md` with:
  - a table of all test files
  - the feature/contract covered by each file
  - major happy paths
  - major edge/error paths
  - explicit note of any unimplemented coverage
- the backend test suite must also include a concise coverage map doc at `apps/backend/tests/COVERAGE.md` listing:
  - each API area
  - its corresponding integration test files
  - each major business rule
  - where that rule is verified

Reviewer usability requirement:

- a reviewer should be able to read `apps/frontend/tests/README.md`, `apps/backend/tests/README.md`, and `apps/backend/tests/COVERAGE.md`, then skim test file headers, and quickly judge completeness without reading every test body
- if a capability is intentionally untested or partially tested, that gap must be called out in the docs rather than left implicit

Current test sources to port behavior from:

- auth:
  - `dps-dashboard/tests/unit/auth.test.ts`
  - `dps-dashboard/tests/integration/auth.test.ts`
- routing/API surface:
  - `dps-dashboard/tests/integration/api-routes.test.ts`
  - `dps-dashboard/tests/unit/route-handlers-core.test.ts`
  - `dps-dashboard/tests/unit/route-handlers-extended.test.ts`
  - `dps-dashboard/tests/unit/public-route-handlers.test.ts`
  - `dps-dashboard/tests/unit/sub-members-route.test.ts`
- services/business rules:
  - `dps-dashboard/tests/unit/member-service.test.ts`
  - `dps-dashboard/tests/unit/membership-service.test.ts`
  - `dps-dashboard/tests/unit/transaction-service.test.ts`
  - `dps-dashboard/tests/unit/approval-service.test.ts`
  - `dps-dashboard/tests/unit/sponsor-service.test.ts`
  - `dps-dashboard/tests/unit/member-lifecycle.test.ts`
  - `dps-dashboard/tests/unit/transaction-membership-link.test.ts`
  - `dps-dashboard/tests/integration/membership-transaction-flow.test.ts`
- support/integrations:
  - `dps-dashboard/tests/unit/member-id.test.ts`
  - `dps-dashboard/tests/unit/member-id-extended.test.ts`
  - `dps-dashboard/tests/unit/encrypt.test.ts`
  - `dps-dashboard/tests/unit/encrypt-extended.test.ts`
  - `dps-dashboard/tests/unit/validators-extended.test.ts`
  - `dps-dashboard/tests/unit/schema.test.ts`
  - `dps-dashboard/tests/unit/receipt.test.ts`
  - `dps-dashboard/tests/unit/razorpay.test.ts`
  - `dps-dashboard/tests/unit/webhook-razorpay.test.ts`
  - `dps-dashboard/tests/unit/whatsapp.test.ts`
  - `dps-dashboard/tests/unit/webhook-sponsor-handler.test.ts`
  - `dps-dashboard/tests/unit/cron.test.ts`

Planned backend test locations:

- `apps/backend/tests/auth_integration.rs`
- `apps/backend/tests/members_integration.rs`
- `apps/backend/tests/memberships_integration.rs`
- `apps/backend/tests/transactions_integration.rs`
- `apps/backend/tests/approvals_integration.rs`
- `apps/backend/tests/sponsors_integration.rs`
- `apps/backend/tests/sponsor_links_integration.rs`
- `apps/backend/tests/dashboard_integration.rs`
- `apps/backend/tests/receipts_integration.rs`
- `apps/backend/tests/webhooks_integration.rs`
- `apps/backend/tests/cron_integration.rs`
- `apps/backend/tests/support_member_id.rs`
- `apps/backend/tests/support_encrypt.rs`
- `apps/backend/tests/support_receipt.rs`

## Final Dependency Recommendations

### Backend runtime

- `axum`
- `tokio`
- `tower-http`
- `serde`
- `serde_json`
- `chrono`
- `uuid`
- `sqlx`
- `bcrypt`
- `rand`
- `aes-gcm`
- `base64`
- `hmac`
- `sha2`
- `hex`
- `reqwest`
- `tokio-cron-scheduler`
- `tracing`
- `tracing-subscriber`
- `thiserror`
- `dotenvy`

### Backend dev/test

- `tempfile`
- `axum-test`
- `tokio-test`
- `pretty_assertions`

Explicitly excluded to keep backend minimal:

- `wiremock`
- `anyhow`
- `config`
- `validator`
- `utoipa`
- `utoipa-swagger-ui`
- `askama`
- `tower` direct dependency
- `hyper` direct dependency
- `cookie` direct dependency
- `time` direct dependency

## Execution Order

1. Freeze current API/UI/schema behavior as migration contract
2. Create `apps/frontend` and copy frontend code over
3. Remove `src/app/api/**` and `next-auth` coupling from frontend
4. Create `apps/backend` Axum skeleton with auth/session/cookie support
5. Port schema to SQLite migrations
6. Port seed logic
7. Port auth and shared support code
8. Port services and matching route handlers module-by-module
9. Add scheduler with `tokio-cron-scheduler`
10. Repoint frontend HTTP calls to backend
11. Rebuild backend integration coverage
12. Re-run parity checks and remove obsolete monolith pieces

## Acceptance Criteria

The migration is complete when all of the following are true:

- frontend and backend are cleanly separated
- frontend communicates with backend only over HTTP
- UI matches the current app screen-for-screen
- core route paths and response contracts remain compatible
- seed data reproduces the current working dataset
- backend integration tests cover all core business flows
- frontend and backend test docs make coverage review fast and explicit
- scheduled expiry/reminder behavior is preserved
- Prisma, PostgreSQL, and NextAuth are no longer required by the new app

## Assumptions

- frontend shell remains Next.js for maximum copy/reuse and UI fidelity
- backend uses Axum + SQLx + SQLite
- exact feature parity includes cron-based expiry/reminder behavior
- deployment strategy is intentionally out of scope
- this plan is decision-complete and intended to be implemented without further architectural choices
