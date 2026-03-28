# BSDS Dashboard: Agent Schema

## Context

The `bsds-dashboard-migration-plan.md` defines a full migration from a Next.js 14 monolith (`dps-dashboard`) into:
- `apps/frontend` — Next.js + TypeScript, UI only
- `apps/backend` — Rust, Axum + SQLx + SQLite

The plan is explicit, decision-complete, and prohibits architectural invention. This document defines the minimal set of specialized agents that can execute the plan in parallel, each scoped to a well-bounded slice of the work.

---

## Agent Schema

### Agent 1: `repo-scaffold`
**Role:** One-time setup of the monorepo skeleton.
**Plan reference:** §1 Repository Structure
**Tasks:**
- Create `apps/frontend/` and `apps/backend/` directories
- Initialize `apps/frontend` as a Next.js 14 + TypeScript project (copy `package.json` base from `dps-dashboard`, strip server deps)
- Initialize `apps/backend` as a Rust workspace with `Cargo.toml` and full dependency set from plan §Final Dependency Recommendations
- Create `sqlite/` directory, add SQLite files to `.gitignore`
- Create top-level workspace config if needed

**Must complete before:** all other agents start

---

### Agent 2: `frontend-copy`
**Role:** Copy frontend files with minimal modification.
**Plan reference:** §2 Frontend Migration — "Copy into apps/frontend with no modification unless required"
**Tasks:**
- Copy all pages listed in plan §2 from `../dps-dashboard/src/app/**` → `apps/frontend/app/**`
- Copy all components: `components/ui/**`, `components/layout/**`, `components/landing/**`, `components/members/**`, `components/receipts/**`
- Copy `public/**`, `globals.css`, Tailwind/shadcn config
- Copy `src/types/index.ts` (strip NextAuth augmentation)
- Copy `src/lib/hooks/use-api.ts` (preserve SWR, add backend base URL)

**Depends on:** `repo-scaffold`
**Runs parallel with:** `frontend-auth`, `backend-schema`

---

### Agent 3: `frontend-auth`
**Role:** Build the frontend auth layer that replaces `next-auth`.
**Plan reference:** §2 Frontend Migration — "Frontend-only additions to create" and "Current frontend files that require modification"
**Tasks:**
- Create `apps/frontend/lib/auth-client.ts` — login/logout/me/change-password fetch wrappers
- Create `apps/frontend/lib/auth-types.ts` — session/user types without NextAuth
- Create `apps/frontend/lib/api-client.ts` — base URL config + credentials include
- Create `apps/frontend/components/providers/AuthProvider.tsx` — replaces SessionProvider, loads session via `GET /api/auth/me`
- Create `apps/frontend/hooks/use-auth.ts` — replaces `useSession`
- Modify `app/layout.tsx` — swap SessionProvider → AuthProvider
- Modify `app/login/page.tsx` — replace `signIn("credentials")` with backend login fetch
- Modify `app/change-password/page.tsx` — replace NextAuth signOut/refresh with backend calls
- Modify `app/dashboard/layout.tsx` — replace `getServerSession` with frontend auth guard
- Modify `components/layout/Sidebar.tsx` — replace `signOut` with backend logout call
- Modify all dashboard pages to replace `useSession` with `useAuth`
- Replace `middleware.ts` with frontend route guard (unauthenticated → `/login`, temp-password → `/change-password`)

**Depends on:** `repo-scaffold`
**Runs parallel with:** `frontend-copy`, `backend-schema`

---

### Agent 4: `backend-schema`
**Role:** Port the Prisma schema to SQLite migrations.
**Plan reference:** §4 Database and Schema
**Source:** `../dps-dashboard/prisma/schema.prisma`, `../dps-dashboard/prisma/migrations/**`
**Tasks:**
- Read Prisma schema and understand all entities, relations, enums
- Write `apps/backend/migrations/0001_initial.sql` — full SQLite schema
  - enums → TEXT + CHECK constraints
  - UUIDs → TEXT
  - JSON snapshots → TEXT
  - timestamps → ISO strings
  - decimal → NUMERIC
- Write `apps/backend/src/db/mod.rs` — SQLx pool init, connection from env
- Write `apps/backend/src/db/models.rs` — Rust structs matching all DB rows

**Depends on:** `repo-scaffold`
**Runs parallel with:** `frontend-copy`, `frontend-auth`

---

### Agent 5: `backend-auth`
**Role:** Implement auth routes and session logic.
**Plan reference:** §3 Backend Migration — auth section, §2 Frontend auth contract
**Source:** `../dps-dashboard/src/app/api/auth/**`, `../dps-dashboard/src/lib/auth.ts`, `../dps-dashboard/src/lib/temp-password.ts`
**Tasks:**
- `apps/backend/src/auth/mod.rs` — session creation, cookie handling, password hashing (bcrypt)
- `apps/backend/src/auth/permissions.rs` — role/permission checks from `lib/permissions.ts`
- `apps/backend/src/auth/temp_password.rs` — temp password generation/detection
- `apps/backend/src/routes/auth.rs` — POST /api/auth/login, POST /api/auth/logout, GET /api/auth/me, POST /api/auth/change-password

**Depends on:** `backend-schema`
**Runs parallel with:** `backend-support`

---

### Agent 6: `backend-support`
**Role:** Port all shared support/utility modules.
**Plan reference:** §3 Backend Migration — "Current service/helper sources to port" (support section)
**Source:** `../dps-dashboard/src/lib/`
**Tasks:**
- `support/member_id.rs` — from `member-id.ts`
- `support/encrypt.rs` — from `encrypt.ts` (AES-GCM + base64)
- `support/rate_limit.rs` — from `rate-limit.ts`
- `support/audit.rs` — from `audit.ts`
- `support/membership_rules.rs` — from `membership-rules.ts`
- `support/receipt.rs` — from `receipt.ts` + `receipt-utils.ts`
- `support/validation.rs` — shared validation helpers from `validators.ts`
- `integrations/razorpay.rs` — from `razorpay.ts`
- `integrations/whatsapp.rs` — from `whatsapp.ts`

**Depends on:** `backend-schema`
**Runs parallel with:** `backend-auth`

---

### Agent 7: `backend-services`
**Role:** Port all service modules (business logic layer).
**Plan reference:** §3 Backend Migration — service sources table
**Source:** `../dps-dashboard/src/lib/services/**`
**Tasks:**
- `services/member_service.rs`
- `services/membership_service.rs`
- `services/transaction_service.rs`
- `services/approval_service.rs`
- `services/sponsor_service.rs`
- `services/notification_service.rs`
- `services/webhook_sponsor_handler.rs`
- `repositories/` — matching SQLx query modules for all entities

**Depends on:** `backend-schema`, `backend-support`, `backend-auth`

---

### Agent 8: `backend-routes`
**Role:** Port all HTTP route handlers.
**Plan reference:** §3 Backend Migration — planned backend route layout
**Source:** `../dps-dashboard/src/app/api/**`
**Tasks:**
- One file per route group: members, memberships, my_membership, transactions, approvals, audit_log, activity_log, dashboard, sponsors, sponsor_links, receipts, payments, webhooks, cron
- Wire all routes into `main.rs` via Axum router
- Preserve all API path shapes from the plan's preserved routes list

**Depends on:** `backend-services`

---

### Agent 9: `backend-scheduler`
**Role:** Implement cron/scheduler.
**Plan reference:** §5 Scheduler / Cron
**Source:** `../dps-dashboard/src/lib/cron.ts`, `../dps-dashboard/src/app/api/cron/route.ts`
**Tasks:**
- `scheduler/mod.rs` — register tokio-cron-scheduler, bind schedule strings
- `scheduler/cron_jobs.rs` — job runners that delegate to service layer
- Optional: `routes/cron.rs` — manual trigger endpoint

**Depends on:** `backend-services`
**Runs parallel with:** `backend-routes`

---

### Agent 10: `backend-seed`
**Role:** Port seed data to Rust.
**Plan reference:** §6 Seed Strategy
**Source:** `../dps-dashboard/prisma/seed.ts`
**Tasks:**
- `src/bin/seed.rs` — main seed entrypoint
- Optional: `src/seed/mod.rs`, `builders.rs`, `fixtures.rs`
- Must reproduce same accounts, passwords, roles, members, transactions, approvals, sponsors, audit/activity entries
- Must be rerunnable (idempotent)

**Depends on:** `backend-services`
**Runs parallel with:** `backend-routes`, `backend-scheduler`

---

### Agent 11: `backend-tests`
**Role:** Write backend integration test suite.
**Plan reference:** §7 Testing Strategy — Backend
**Tasks:**
- One integration test file per domain area (see plan §7 Planned backend test locations)
- Use `axum-test` + `tempfile` SQLite DBs
- Write `apps/backend/tests/README.md` — inventory + flows + gaps
- Write `apps/backend/tests/COVERAGE.md` — API area × test file mapping, rule × verification mapping

**Depends on:** `backend-routes`

---

### Agent 12: `frontend-tests`
**Role:** Adapt and port frontend test suite.
**Plan reference:** §7 Testing Strategy — Frontend
**Source:** `../dps-dashboard/tests/components/**`, `../dps-dashboard/tests/unit/use-api.test.ts`, `../dps-dashboard/tests/unit/utils*.test.ts`
**Tasks:**
- Port component and page render tests
- Replace `next-auth` mock patterns with new `AuthProvider`/`useAuth` mocks
- Write `apps/frontend/tests/README.md` — inventory + covered flows + known gaps

**Depends on:** `frontend-copy`, `frontend-auth`

---

## Execution Waves

| Wave | Agents | Condition |
|------|--------|-----------|
| 1 | `repo-scaffold` | First — must finish before anything else |
| 2 | `frontend-copy`, `frontend-auth`, `backend-schema` | After Wave 1 — fully parallel |
| 3 | `backend-auth`, `backend-support` | After `backend-schema` — parallel with each other |
| 4 | `backend-services` | After Wave 3 |
| 5 | `backend-routes`, `backend-scheduler`, `backend-seed` | After `backend-services` — fully parallel |
| 6 | `backend-tests`, `frontend-tests` | After Wave 5 |

---

## Rules for All Agents

- Read only the plan sections and source files needed for your scope — no scope creep
- Copy first, modify only where the frontend/backend split requires it
- Do not refactor for style or architecture
- Backend agents: keep as the single source of truth for all business/domain state
- Frontend agents: mutations must re-fetch or invalidate server data, never treat client cache as authoritative
