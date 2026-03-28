# BSDS Dashboard — Gap Analysis vs DPS Dashboard
**Date:** 2026-03-26
**Scope:** Absolute feature, data, layout, and test parity
**Source of truth:** `/home/mojo/github/cypherstream projects/dps-dashboard`
**Target:** `/home/mojo/github/cypherstream projects/bsds-dashboard`

---

## Executive Summary

The bsds-dashboard port has solid foundational infrastructure (schema, auth, core CRUD, scheduler, seeding), but has **critical functional gaps** in five areas:

1. **Sub-member write operations** — backend route only supports GET; POST/PUT/DELETE missing
2. **Approval routes** — wrong HTTP method (GET with body instead of POST); separate approve/reject sub-routes missing; individual approval GET missing
3. **Transaction reject** — no reject endpoint; update/delete routes return errors (correct) but no admin rejection path
4. **Memberships list** — returns empty array when no `member_id` provided; full paginated list not implemented
5. **Test coverage** — no service-level unit tests; missing security, schema, lifecycle, and transaction-membership-link tests

---

## Section 1: Backend API Route Gaps

### 1.1 Sub-Members — CRITICAL

| Endpoint | DPS | BSDS | Status |
|----------|-----|------|--------|
| `GET /api/members/:id/sub-members` | ✅ | ✅ | OK |
| `POST /api/members/:id/sub-members` | ✅ | ❌ | **MISSING** |
| `PUT /api/members/:id/sub-members` | ✅ | ❌ | **MISSING** |
| `DELETE /api/members/:id/sub-members` | ✅ | ❌ | **MISSING** |

**Impact:** Members dashboard UI cannot add, edit, or remove sub-members. This silently fails or 405s.
**File:** `apps/backend/src/routes/members.rs:27` — only `get(list_sub_members)` registered
**DPS source:** `src/app/api/members/[id]/sub-members/route.ts` — full CRUD with max-3 enforcement

---

### 1.2 Approvals — CRITICAL

| Endpoint | DPS | BSDS | Status |
|----------|-----|------|--------|
| `GET /api/approvals` | ✅ | ✅ | OK |
| `GET /api/approvals/:id` | ✅ | ❌ | **MISSING** (current GET/:id takes body = bug) |
| `POST /api/approvals/:id/approve` | ✅ | ❌ | **MISSING** |
| `POST /api/approvals/:id/reject` | ✅ | ❌ | **MISSING** |

**Bug:** `apps/backend/src/routes/approvals.rs:25-26` registers `review_approval` as `GET /:id`. This handler accepts a JSON body and performs approve/reject — GET requests must not have bodies (RFC 9110). Approval actions must be POST.
**Impact:** Admin cannot approve or reject pending approvals. The frontend's approval flow is completely broken.
**DPS source:** `src/app/api/approvals/[id]/approve/route.ts` and `reject/route.ts`

---

### 1.3 Transaction Reject — HIGH

| Endpoint | DPS | BSDS | Status |
|----------|-----|------|--------|
| `POST /api/transactions/:id/reject` | ✅ | ❌ | **MISSING** |

**Context:** BSDS correctly returns errors from `update_transaction` and `delete_transaction` (transactions are immutable). But there is no admin reject path. DPS uses `POST /api/transactions/[id]/reject` to mark a transaction as REJECTED and write to the audit log.
**File:** `apps/backend/src/routes/transactions.rs:29` — no `/reject` sub-route
**DPS source:** `src/app/api/transactions/[id]/reject/route.ts`

---

### 1.4 Memberships List — HIGH

| Behavior | DPS | BSDS | Status |
|----------|-----|------|--------|
| `GET /api/memberships` (no params) | Returns paginated list | Returns `[]` | **BROKEN** |
| `GET /api/memberships?member_id=X` | Returns member's list | ✅ Works | OK |

**File:** `apps/backend/src/routes/memberships.rs:32-38` — `list_memberships` returns empty array if no `member_id`.
**Impact:** Memberships dashboard page shows no data unless filtering by member.

---

### 1.5 Missing Routes

| Route | DPS | BSDS | Notes |
|-------|-----|------|-------|
| `POST /api/notifications/whatsapp` | ✅ | ❌ | Admin-only WhatsApp send; low priority |
| `GET /api/approvals/:id` (individual) | ✅ | ❌ | Needed for approval-detail modal |
| Role enforcement per-handler | ✅ | ⚠️ | See Section 2 |

---

## Section 2: Security — Role Enforcement Gap

**Status: HIGH**

DPS enforces roles inline in every route handler using `requireRole(session, ...)`. BSDS defines `permissions.rs` with the full role matrix but **never calls `can_access_route()` inside any route handler**. The permissions module is defined but never enforced at the handler level.

**Effect:** Any authenticated user (including MEMBER role) can call admin-only endpoints like `POST /api/approvals/:id` or `POST /api/transactions`.

**Required:** Each handler must extract the role from `AuthSession` claims and call `can_access_route()` or inline role checks, mirroring DPS's `requireRole()` pattern.

**Files to fix:** All files in `apps/backend/src/routes/` — currently all handlers do `let _ = claims;` (ignoring the role).

---

## Section 3: Business Logic Gaps

### 3.1 `applyMembershipStatusFromTransaction` — CRITICAL

DPS membership service calls `applyMembershipStatusFromTransaction()` after a membership transaction is approved. This updates:
- `User.membershipStatus` → ACTIVE
- `User.membershipType`
- `User.membershipStart` / `User.membershipExpiry`
- `User.totalPaid` (incremented)
- `User.applicationFeePaid` (if applicable)
- `User.annualFeePaid`, `User.annualFeeStart`, `User.annualFeeExpiry`

**Verify:** `apps/backend/src/services/membership_service.rs` — confirm `approve_membership()` updates ALL these User fields. If it only updates the `memberships.status` field but not the `users` table, the member's dashboard will show wrong data.

---

### 3.2 Audit Log — TRANSACTION_CREATED Event

DPS `audit.ts` logs `TRANSACTION_CREATED` to `audit_logs` when any transaction is created (not just approved). BSDS `support/audit.rs` has this event type. **Verify** it is actually called in `transaction_service.rs::create_transaction()`.

---

### 3.3 Activity Log Action Completeness

DPS logs 47+ distinct action strings to `activity_logs`. Verify BSDS logs the same actions in the same places:

| Action | Logged In DPS | Verify In BSDS |
|--------|--------------|----------------|
| `login_success` | auth service | auth route |
| `login_failed` | auth service | auth route |
| `member_created` | member service | member_service.rs |
| `member_updated` | member service | member_service.rs |
| `member_deleted` | member service | member_service.rs |
| `sub_member_added` | member service | member_service.rs |
| `sub_member_updated` | member service | member_service.rs |
| `sub_member_removed` | member service | member_service.rs |
| `membership_created` | membership service | membership_service.rs |
| `membership_approved` | approval service | approval_service.rs |
| `membership_rejected` | approval service | approval_service.rs |
| `transaction_created` | transaction service | transaction_service.rs |
| `transaction_approved` | approval service | approval_service.rs |
| `transaction_rejected` | approval service | approval_service.rs |
| `approval_submitted` | all services | all services |
| `approval_reviewed` | approval service | approval_service.rs |
| `password_changed` | auth | auth route |
| `sponsor_created` | sponsor service | sponsor_service.rs |
| `sponsor_link_created` | sponsor service | sponsor_service.rs |
| `cron_expiry_check` | cron | scheduler/cron_jobs.rs |
| `notification_sent` | notification service | notification_service.rs |
| `notification_failed` | notification service | notification_service.rs |

---

### 3.4 Input Validation

DPS uses Zod schemas (`src/lib/validators.ts`, 729 lines) to validate all API inputs before service calls. BSDS routes do raw JSON parsing with `.as_str().unwrap_or("")` — no validation. Missing:
- Required field checks (empty string accepted as valid)
- Phone format validation (+91XXXXXXXXXX)
- Email format validation
- Amount decimal validation
- Enum value validation (category, type, payment_mode)

---

## Section 4: Frontend Layout & UI Gaps

### 4.1 Sub-Member Management UI

**Status: BLOCKED by backend**

DPS `src/app/dashboard/members/page.tsx` has:
- Inline sub-member panel per member showing all sub-members
- "Add Sub-member" dialog with name, email, phone, relation fields
- Edit sub-member button per row
- Remove sub-member button with confirmation
- Max-3 indicator badge

BSDS frontend may have copied this UI. But since the backend routes don't support POST/PUT/DELETE on sub-members, these operations silently fail.

---

### 4.2 Approvals Page — Action Endpoints

DPS frontend `src/app/dashboard/approvals/page.tsx` calls:
- `POST /api/approvals/:id/approve` with `{ notes }`
- `POST /api/approvals/:id/reject` with `{ notes }`

BSDS backend only registers `GET /:id`. The frontend calls will 405 Method Not Allowed.

**Verify:** `apps/frontend/app/dashboard/approvals/page.tsx` — what URL pattern does it use for approve/reject?

---

### 4.3 Individual Approval Detail

DPS fetches a single approval via `GET /api/approvals/:id` for the approval detail modal. This endpoint is missing in BSDS. The modal will be empty or error.

---

### 4.4 Loading Skeleton Files

DPS has `loading.tsx` for every dashboard page. Check which are missing in BSDS:

| Page | DPS | BSDS | Status |
|------|-----|------|--------|
| `dashboard/members/loading.tsx` | ✅ | ❓ | Verify |
| `dashboard/my-membership/loading.tsx` | ✅ | ❓ | Verify |
| `dashboard/cash/loading.tsx` | ✅ | ❓ | Verify |
| `dashboard/approvals/loading.tsx` | ✅ | ❓ | Verify |
| `dashboard/audit-log/loading.tsx` | ✅ | ❓ | Verify |
| `dashboard/activity-log/loading.tsx` | ✅ | ❓ | Verify |
| `dashboard/sponsorship/loading.tsx` | ✅ | ❓ | Verify |
| `dashboard/memberships/page.tsx` | ❓ | ❓ | Verify (DPS may not have memberships page) |

---

### 4.5 Approvals Constants File

DPS has `src/app/dashboard/approvals/constants.ts` (approval entity type labels, action labels, status colors). BSDS may be missing this or have it inlined in the page.

---

### 4.6 Cash Page — Membership Fee Checkbox Logic

DPS `cash/page.tsx` (1472 lines) has:
- Checkbox group for fee types: `includesSubscription`, `includesAnnualFee`, `includesApplicationFee`
- Auto-calculates total amount based on checked fees
- Membership type selector (MONTHLY/HALF_YEARLY/ANNUAL) with auto-filled amount
- Member picker with live search
- Receipt preview modal
- Different button label for OPERATOR ("Submit for Approval") vs ADMIN ("Add Transaction")

**Verify** all these exist in `apps/frontend/app/dashboard/cash/page.tsx`.

---

## Section 5: Schema Parity

The SQLite schema (`apps/backend/migrations/0001_initial.sql`) is functionally equivalent to the Prisma schema. All fields are present. No schema gaps found.

**Note:** DPS PostgreSQL uses native enums; BSDS SQLite uses `CHECK` constraints. This is architecturally correct — no change needed.

**One difference:** `transactions` table missing `expense_purpose` column. In DPS, `ExpensePurpose` values (DECORATION_PANDAL, LIGHTING_SOUND, etc.) are stored in the `purpose` TEXT field for EXPENSE-category transactions. This is by design — the `purpose` field serves double duty. No schema change needed, but the frontend must enforce the enum values for EXPENSE transactions.

---

## Section 6: Test Coverage Gaps

### 6.1 Backend Tests — Missing Coverage

| Test Category | DPS | BSDS | Gap |
|---------------|-----|------|-----|
| Service unit tests (all 7 services) | ✅ 7 files | ❌ | All missing |
| Security tests (auth bypass, RBAC) | ✅ `security.test.ts` | ❌ | Missing |
| Schema validation tests | ✅ `schema.test.ts` | ❌ | Missing |
| Transaction-membership link test | ✅ | ❌ | Missing |
| Member lifecycle state machine | ✅ | ❌ | Missing |
| Route handler unit tests | ✅ 2 files | ❌ | Missing |
| Public route handler tests | ✅ | ❌ | Missing |
| Webhook sponsor handler test | ✅ | ⚠️ | BSDS has `webhooks_integration.rs` — verify coverage |
| Sub-members integration test | DPS has sub-member tests | ❌ | BSDS members_integration.rs may be missing |

**Current BSDS backend tests** (15 files) cover integration paths but miss:
- Role enforcement tests (critical given Section 2 gap)
- Business rule unit tests (membership fee calculations, application fee one-time rule)
- Audit log completeness tests
- Activity log completeness tests

---

### 6.2 Frontend Tests — Missing Coverage

| Test | DPS | BSDS | Gap |
|------|-----|------|-----|
| Auth flow (login, logout, session) | ✅ | ❌ | Missing |
| Approval service logic | ✅ | ❌ | Missing |
| Member service logic | ✅ | ❌ | Missing |
| Membership service logic | ✅ | ❌ | Missing |
| Transaction service logic | ✅ | ❌ | Missing |
| Sponsor service logic | ✅ | ❌ | Missing |
| Notification service | ✅ | ❌ | Missing |
| Route handler core | ✅ | ❌ | Missing |
| Route handler extended | ✅ | ❌ | Missing |
| Public route handlers | ✅ | ❌ | Missing |
| Webhook razorpay | ✅ | ❌ | Missing |
| Layout components | ✅ | ✅ | OK |
| Utility functions | ✅ | ✅ | OK |
| API hook | ✅ | ✅ | OK |

---

## Section 7: Prioritized Task List

### Priority 1 — CRITICAL (blocks normal operation)

| # | Task | Impact |
|---|------|--------|
| P1-1 | Add `POST`, `PUT`, `DELETE` to `/api/members/:id/sub-members` route and member_service | Members page sub-member CRUD broken |
| P1-2 | Fix approvals route: split GET /:id (view) + POST /:id/approve + POST /:id/reject | Approval workflow completely broken |
| P1-3 | Add `POST /api/transactions/:id/reject` route and wire to transaction_service | Admin cannot reject transactions |
| P1-4 | Enforce roles in all route handlers using `can_access_route()` or inline checks | Security: any user can call admin routes |
| P1-5 | Fix `list_memberships` to return paginated list when no `member_id` provided | Memberships page shows no data |

### Priority 2 — HIGH (feature gaps)

| # | Task | Impact |
|---|------|--------|
| P2-1 | Verify `approve_membership` updates ALL User fields (status, expiry, totalPaid, etc.) | Member dashboard shows stale data |
| P2-2 | Add `GET /api/approvals/:id` for individual approval detail | Approval detail modal broken |
| P2-3 | Add input validation in all route handlers (required fields, enum values, format checks) | Bad data enters DB silently |
| P2-4 | Verify `TRANSACTION_CREATED` audit log event is emitted on transaction creation | Audit log incomplete |
| P2-5 | Verify all 22+ activity log action strings are emitted in the correct service methods | Activity log incomplete |

### Priority 3 — MEDIUM (completeness)

| # | Task | Impact |
|---|------|--------|
| P3-1 | Add `POST /api/notifications/whatsapp` route | Admin cannot send manual WhatsApp notifications |
| P3-2 | Verify/add all loading.tsx skeleton files for dashboard pages | UX degraded on slow connections |
| P3-3 | Verify approvals/constants.ts exists and has correct labels | UI labels may be wrong |
| P3-4 | Verify cash page has checkbox fee logic, membership type selector, and correct button labels | Feature incomplete |
| P3-5 | Verify approval-detail diff view for MEMBER_EDIT shows before/after comparison | Admin review quality degraded |

### Priority 4 — Tests (coverage)

| # | Task | Impact |
|---|------|--------|
| P4-1 | Add role enforcement integration tests to `approvals_integration.rs`, `members_integration.rs`, etc. | Security regressions undetected |
| P4-2 | Add `security_integration.rs` — test that MEMBER/ORGANISER cannot call admin routes | RBAC gaps undetected |
| P4-3 | Add `member_lifecycle_integration.rs` — test PENDING_APPROVAL → ACTIVE state machine | Lifecycle regressions undetected |
| P4-4 | Add `transaction_membership_link_integration.rs` — test fee inclusion flags flow end-to-end | Business rule regressions |
| P4-5 | Add service-level unit tests for membership_rules, audit helpers, receipt generation | Pure logic regressions |
| P4-6 | Add frontend tests: auth flow, approval actions, member CRUD with sub-members | Frontend regressions undetected |

---

## Section 8: Agent Task Assignments

### Agent: `backend-routes` (fix backend route gaps)

**Task:** Fix all P1 and P2 backend route gaps
**Knowledge sources:**
- DPS source: `src/app/api/members/[id]/sub-members/route.ts`
- DPS source: `src/app/api/approvals/[id]/approve/route.ts`
- DPS source: `src/app/api/approvals/[id]/reject/route.ts`
- DPS source: `src/app/api/transactions/[id]/reject/route.ts`
- BSDS: `apps/backend/src/routes/members.rs`
- BSDS: `apps/backend/src/routes/approvals.rs`
- BSDS: `apps/backend/src/routes/transactions.rs`
- BSDS: `apps/backend/src/routes/memberships.rs`
- BSDS: `apps/backend/src/auth/permissions.rs`

**Specific tasks:**
1. Add POST/PUT/DELETE handlers to `members.rs` for `/:id/sub-members`
2. Fix `approvals.rs`: replace `GET /:id` with `GET /:id` (view only) + `POST /:id/approve` + `POST /:id/reject`
3. Add `POST /:id/reject` to `transactions.rs`
4. Add role enforcement calls (`can_access_route` or per-handler role checks) to all handlers
5. Fix `list_memberships` to support full paginated listing

---

### Agent: `backend-services` (fix business logic gaps)

**Task:** Fill service-level gaps for sub-members, membership status updates, validation
**Knowledge sources:**
- DPS source: `src/lib/services/member-service.ts` — `addSubMember()`, `updateSubMember()`, `removeSubMember()`
- DPS source: `src/lib/services/membership-service.ts` — `applyMembershipStatusFromTransaction()`
- DPS source: `src/lib/services/approval-service.ts` — `approveEntry()` dispatch for MEMBERSHIP type
- DPS source: `src/lib/validators.ts` — all Zod schemas
- BSDS: `apps/backend/src/services/member_service.rs`
- BSDS: `apps/backend/src/services/membership_service.rs`
- BSDS: `apps/backend/src/services/approval_service.rs`

**Specific tasks:**
1. Add `add_sub_member()`, `update_sub_member()`, `remove_sub_member()` to `member_service.rs`
2. Verify and fix `approve_membership()` to update ALL User fields atomically
3. Add input validation (enum checks, required fields, phone/email format) to service layer
4. Verify audit log emitted on TRANSACTION_CREATED
5. Audit all 22+ activity log action strings across services

---

### Agent: `backend-tests` (close test coverage gaps)

**Task:** Write exhaustive integration and unit tests for all critical workflows
**Knowledge sources:**
- DPS tests: `tests/unit/security.test.ts`
- DPS tests: `tests/unit/member-lifecycle.test.ts`
- DPS tests: `tests/unit/transaction-membership-link.test.ts`
- DPS tests: `tests/unit/approval-service.test.ts`
- DPS tests: `tests/integration/membership-transaction-flow.test.ts`
- BSDS: `apps/backend/tests/` — all existing test files
- BSDS: `apps/backend/tests/common/mod.rs`

**Specific tasks:**
1. Add `security_integration.rs` — RBAC enforcement for all admin-only routes
2. Add `member_lifecycle_integration.rs` — full state machine from creation to expiry
3. Add `transaction_membership_link_integration.rs` — fee inclusion flags → User field updates
4. Add sub-member CRUD tests to `members_integration.rs`
5. Add approve/reject flow tests to `approvals_integration.rs`
6. Add role enforcement assertions to every existing integration test

---

### Agent: `frontend-copy` (verify/fix frontend parity)

**Task:** Verify frontend pages match DPS pixel-for-pixel; add missing loading skeletons; fix approval endpoint URLs
**Knowledge sources:**
- DPS: `src/app/dashboard/approvals/page.tsx` — approve/reject endpoint URLs
- DPS: `src/app/dashboard/approvals/approval-detail.tsx` — diff view for MEMBER_EDIT
- DPS: `src/app/dashboard/approvals/constants.ts`
- DPS: `src/app/dashboard/cash/page.tsx` — full 1472-line version
- DPS: `src/app/dashboard/members/page.tsx` — sub-member UI
- DPS: All `loading.tsx` files
- BSDS: `apps/frontend/app/dashboard/` — all pages

**Specific tasks:**
1. Verify approval page calls `POST /api/approvals/:id/approve` and `POST /api/approvals/:id/reject` (not GET)
2. Verify `GET /api/approvals/:id` is called for approval detail modal
3. Verify cash page has all: fee checkboxes, membership type selector, member picker, receipt modal, operator vs admin button labels
4. Verify members page has sub-member add/edit/remove UI
5. Add missing `loading.tsx` files for all dashboard pages
6. Verify `approvals/constants.ts` exists with correct labels

---

### Agent: `frontend-tests` (frontend test coverage)

**Task:** Add frontend service-level unit tests and critical workflow tests
**Knowledge sources:**
- DPS tests: `tests/unit/approval-service.test.ts`
- DPS tests: `tests/unit/member-service.test.ts`
- DPS tests: `tests/unit/membership-service.test.ts`
- DPS tests: `tests/unit/transaction-service.test.ts`
- DPS tests: `tests/unit/auth.test.ts`
- DPS tests: `tests/unit/route-handlers-core.test.ts`
- DPS tests: `tests/unit/public-route-handlers.test.ts`
- BSDS: `apps/frontend/tests/` — all existing test files
- BSDS: `apps/frontend/tests/setup.ts`

**Specific tasks:**
1. Add `tests/unit/auth-flow.test.ts` — login, logout, temp password redirect, session
2. Add `tests/unit/approval-workflow.test.ts` — approve/reject UI flow
3. Add `tests/unit/member-submember.test.ts` — member CRUD + sub-member add/remove
4. Add `tests/unit/cash-fees.test.ts` — fee checkbox logic, amount calculation
5. Add `tests/integration/approval-flow.test.ts` — end-to-end approval from operator submit to admin action

---

## Section 9: Verification Checklist

Before marking any gap as closed, verify against these acceptance criteria:

### Approval Workflow
- [ ] Operator creates a member → appears in approval queue as MEMBER_ADD
- [ ] Admin views queue → sees pending items with entity type and summary
- [ ] Admin clicks item → detail modal shows newData fields
- [ ] Admin approves → member is created, User + Member records in DB, temp password sent via WhatsApp
- [ ] Admin rejects → approval marked REJECTED, activity log entry created
- [ ] MEMBER role user cannot call `/api/approvals` → 403

### Cash/Transaction Approval
- [ ] Operator submits cash-in → transaction created as PENDING, approval queued
- [ ] Admin approves transaction → `approval_status = APPROVED`, audit log entry `TRANSACTION_APPROVED`
- [ ] Admin rejects transaction → `approval_status = REJECTED`, audit log entry `TRANSACTION_REJECTED`
- [ ] Admin directly creates cash-in → `approval_status = APPROVED` immediately, audit log entry `TRANSACTION_CREATED` + `TRANSACTION_APPROVED`
- [ ] ORGANISER cannot create transactions → 403

### Membership Lifecycle
- [ ] Member created → `membershipStatus = PENDING_APPROVAL`
- [ ] Admin approves membership application → `membershipStatus = PENDING_PAYMENT`
- [ ] Membership payment transaction approved → `membershipStatus = ACTIVE`, `membershipExpiry` set
- [ ] Cron runs after expiry → `membershipStatus = EXPIRED`
- [ ] 15 days before expiry → WhatsApp reminder sent, activity log entry `cron_expiry_check`

### Sub-Members
- [ ] Admin adds sub-member to member → BSDS-YYYY-NNNN-01 memberId generated
- [ ] Cannot add 4th sub-member → 400 error
- [ ] Admin edits sub-member → fields updated
- [ ] Admin removes sub-member → record deleted

### Audit & Activity Logs
- [ ] Audit log shows `TRANSACTION_CREATED` on every new transaction
- [ ] Audit log shows `TRANSACTION_APPROVED` when approved
- [ ] Audit log shows `TRANSACTION_REJECTED` when rejected
- [ ] Audit log is append-only (no delete, no edit endpoints)
- [ ] Activity log shows login, logout, member CRUD, approval actions, notifications
- [ ] Activity log is append-only

---

## File Reference Map

| Area | DPS Path | BSDS Path |
|------|----------|-----------|
| Sub-members API | `src/app/api/members/[id]/sub-members/route.ts` | `apps/backend/src/routes/members.rs` |
| Approval approve | `src/app/api/approvals/[id]/approve/route.ts` | `apps/backend/src/routes/approvals.rs` |
| Approval reject | `src/app/api/approvals/[id]/reject/route.ts` | `apps/backend/src/routes/approvals.rs` |
| Transaction reject | `src/app/api/transactions/[id]/reject/route.ts` | `apps/backend/src/routes/transactions.rs` |
| Membership service | `src/lib/services/membership-service.ts` | `apps/backend/src/services/membership_service.rs` |
| Member service | `src/lib/services/member-service.ts` | `apps/backend/src/services/member_service.rs` |
| Approval service | `src/lib/services/approval-service.ts` | `apps/backend/src/services/approval_service.rs` |
| Audit helper | `src/lib/audit.ts` | `apps/backend/src/support/audit.rs` |
| Permissions | `src/lib/permissions.ts` | `apps/backend/src/auth/permissions.rs` |
| Approvals UI | `src/app/dashboard/approvals/page.tsx` | `apps/frontend/app/dashboard/approvals/page.tsx` |
| Approval detail | `src/app/dashboard/approvals/approval-detail.tsx` | `apps/frontend/app/dashboard/approvals/approval-detail.tsx` |
| Cash UI | `src/app/dashboard/cash/page.tsx` | `apps/frontend/app/dashboard/cash/page.tsx` |
| Members UI | `src/app/dashboard/members/page.tsx` | `apps/frontend/app/dashboard/members/page.tsx` |
| DB Schema | `prisma/schema.prisma` | `apps/backend/migrations/0001_initial.sql` |
| Backend tests | `tests/unit/*.test.ts` + `tests/integration/` | `apps/backend/tests/` |
| Frontend tests | `tests/unit/` + `tests/components/` | `apps/frontend/tests/` |
