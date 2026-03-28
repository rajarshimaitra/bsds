# BSDS Dashboard: Execution Strategy

Each wave lists agents that can be spawned in parallel. Do not start a wave until all agents in the previous wave report complete. Agent prompts are available in both `.claude/agents/<name>.md` and `.codex/agents/<name>.md`; use the set that matches the tool you are running.

## Current Resume Point

This repository appears to be at the Wave 4/5 boundary, but do not treat that as sufficient proof. Before starting **Wave 5**, explicitly verify that Waves 1 through 4 satisfy their full "Done when" checklists. If any earlier-wave output is missing or materially incomplete, finish that earlier wave first and only then continue to Wave 5.

## Required Preflight

Before starting any wave N work:

1. Re-check the "Done when" list for every prior wave.
2. Confirm the required files exist and are not still placeholders.
3. If a prior wave is incomplete, stop and complete that wave instead of proceeding.

For this repo, that means Wave 5 agents must verify all Wave 1-4 outputs before they write anything.

---

## Wave 1 — Scaffold (sequential, must finish first)

**Spawn:** `repo-scaffold`

Creates the full directory skeleton, `package.json`, `Cargo.toml` with all dependencies, module stubs, `.gitignore`, and `sqlite/`.

**Done when:**
- `apps/frontend/package.json` exists with name `bsds-dashboard-frontend`
- `apps/backend/Cargo.toml` exists with all plan dependencies
- All `src/*/mod.rs` stubs exist under `apps/backend/src/`
- `sqlite/.gitkeep` exists, SQLite files are in `.gitignore`

---

## Wave 2 — Copy + Schema (fully parallel)

Spawn all three simultaneously in separate chats.

| Agent | What it does |
|---|---|
| `frontend-copy` | Copies all pages, components, assets, types, hooks from `../dps-dashboard` into `apps/frontend` |
| `frontend-auth` | Builds `AuthProvider`, `useAuth`, `auth-client`, `api-client`; replaces all `next-auth` usage |
| `backend-schema` | Ports Prisma schema → `0001_initial.sql`, writes `db/mod.rs` and `db/models.rs` |

**Done when:**
- All `apps/frontend/app/**` pages exist
- `apps/frontend/components/providers/AuthProvider.tsx` exists
- `apps/frontend/hooks/use-auth.ts` exists
- `apps/backend/migrations/0001_initial.sql` exists
- `apps/backend/src/db/models.rs` has a struct for every table

**Note:** `frontend-copy` and `frontend-auth` may work on overlapping files. If running in parallel, `frontend-copy` should finish first so `frontend-auth` has files to modify. Alternatively, run `frontend-copy` then `frontend-auth` sequentially within Wave 2.

---

## Wave 3 — Auth + Support (parallel, both need schema)

Spawn both simultaneously.

| Agent | What it does |
|---|---|
| `backend-auth` | `auth/mod.rs`, `auth/permissions.rs`, `auth/temp_password.rs`, `routes/auth.rs` |
| `backend-support` | All `support/*.rs` modules + `integrations/razorpay.rs` + `integrations/whatsapp.rs` |

**Done when:**
- `apps/backend/src/auth/mod.rs` has session creation/verification and bcrypt helpers
- `apps/backend/src/routes/auth.rs` has all four auth endpoints
- All 7 `support/*.rs` files exist
- Both integration client files exist

---

## Wave 4 — Services (sequential, needs auth + support)

**Spawn:** `backend-services`

Ports all service modules and creates all repository modules.

**Done when:**
- All 7 `services/*.rs` files exist
- All 10 `repositories/*.rs` files exist
- `services/mod.rs` and `repositories/mod.rs` declare all modules

---

## Wave 5 — Routes + Scheduler + Seed (fully parallel)

Spawn all three simultaneously.

| Agent | What it does |
|---|---|
| `backend-routes` | All 15+ route handler files, full Axum router in `main.rs`, CORS, `AppError` type |
| `backend-scheduler` | `scheduler/mod.rs`, `scheduler/cron_jobs.rs`, wired into `main.rs` |
| `backend-seed` | `src/bin/seed.rs`, `src/seed/` with fixtures and builders |

**Done when:**
- All `routes/*.rs` files exist
- `main.rs` has full Axum router with all route nests
- Scheduler starts in `main.rs` via `tokio::spawn`
- `cargo run --bin seed` runs without errors against a fresh DB

---

## Wave 6 — Tests (parallel)

Spawn both simultaneously.

| Agent | What it does |
|---|---|
| `backend-tests` | 14 integration test files + `tests/README.md` + `tests/COVERAGE.md` |
| `frontend-tests` | Ported frontend test files + `tests/README.md` |

**Done when:**
- `cargo test` passes in `apps/backend/`
- `npm test` passes in `apps/frontend/`
- Both README files exist
- `apps/backend/tests/COVERAGE.md` exists

---

## Acceptance Checklist

From `bsds-dashboard-migration-plan.md` §Acceptance Criteria:

- [ ] Frontend and backend are cleanly separated
- [ ] Frontend communicates with backend only over HTTP (`credentials: "include"`, `NEXT_PUBLIC_API_URL`)
- [ ] UI matches current app screen-for-screen
- [ ] Core route paths and response contracts remain compatible
- [ ] Seed data reproduces the current working dataset (`cargo run --bin seed`)
- [ ] Backend integration tests cover all core business flows (`cargo test`)
- [ ] Frontend and backend test docs make coverage review fast and explicit
- [ ] Scheduled expiry/reminder behavior is preserved
- [ ] Prisma, PostgreSQL, and NextAuth are no longer required

---

## Quick Reference: Agent → File

| Agent | Key output files |
|---|---|
| `repo-scaffold` | `apps/frontend/package.json`, `apps/backend/Cargo.toml`, all `mod.rs` stubs |
| `frontend-copy` | `apps/frontend/app/**`, `apps/frontend/components/**`, `apps/frontend/public/**` |
| `frontend-auth` | `lib/auth-client.ts`, `lib/auth-types.ts`, `lib/api-client.ts`, `AuthProvider.tsx`, `use-auth.ts` |
| `backend-schema` | `migrations/0001_initial.sql`, `src/db/mod.rs`, `src/db/models.rs` |
| `backend-auth` | `src/auth/**`, `src/routes/auth.rs` |
| `backend-support` | `src/support/**`, `src/integrations/**` |
| `backend-services` | `src/services/**`, `src/repositories/**` |
| `backend-routes` | `src/routes/**`, updated `src/main.rs` |
| `backend-scheduler` | `src/scheduler/**`, updated `src/main.rs` |
| `backend-seed` | `src/bin/seed.rs`, `src/seed/**` |
| `backend-tests` | `tests/**`, `tests/README.md`, `tests/COVERAGE.md` |
| `frontend-tests` | `apps/frontend/tests/**`, `apps/frontend/tests/README.md` |
