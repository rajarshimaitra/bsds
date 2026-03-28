# BSDS Dashboard — Parity Fix Plan (Wave 7)
**Date:** 2026-03-26
**Derived from:** `docs/gap-analysis.md`
**Parent plan:** `docs/execution-strategy.md` (this is Wave 7, appended after Wave 6)

---

## Overview

Waves 1–6 produced a structurally sound port. Wave 7 closes the functional parity gaps found in the gap analysis. The work is split into four parallel lanes that can run simultaneously, plus one final lane (tests) that depends on all four.

```
Wave 7A  ──  backend-routes    (fix route gaps + role enforcement)
Wave 7B  ──  backend-services  (fix service gaps + validation)
Wave 7C  ──  frontend-copy     (verify + fix all frontend UI gaps)
             ↓
Wave 7D  ──  backend-tests     (exhaustive E2E + security + lifecycle tests)
Wave 7E  ──  frontend-tests    (exhaustive frontend test coverage)
```

7A, 7B, and 7C are fully parallel. 7D and 7E can only start after 7A + 7B + 7C are done.

---

## Wave 7A — `backend-routes`

**Agent:** `backend-routes`
**Runs parallel with:** 7B, 7C

### Context files to read first

```
# DPS sources (read before writing any Rust)
../dps-dashboard/src/app/api/members/[id]/sub-members/route.ts
../dps-dashboard/src/app/api/approvals/route.ts
../dps-dashboard/src/app/api/approvals/[id]/approve/route.ts
../dps-dashboard/src/app/api/approvals/[id]/reject/route.ts
../dps-dashboard/src/app/api/transactions/[id]/reject/route.ts
../dps-dashboard/src/app/api/memberships/route.ts
../dps-dashboard/src/app/api/notifications/whatsapp/route.ts
../dps-dashboard/src/lib/permissions.ts

# BSDS files to modify
apps/backend/src/routes/members.rs
apps/backend/src/routes/approvals.rs
apps/backend/src/routes/transactions.rs
apps/backend/src/routes/memberships.rs
apps/backend/src/auth/permissions.rs
apps/backend/src/services/member_service.rs   (read only — call its new functions)
```

### Task list

#### T7A-1 — Sub-members write endpoints
**File:** `apps/backend/src/routes/members.rs`

Add three new handlers alongside the existing `list_sub_members`:

```rust
// New route registrations to add to router():
.route("/:id/sub-members", get(list_sub_members)
                            .post(add_sub_member)
                            .put(update_sub_member)
                            .delete(remove_sub_member))
```

Handler signatures:

```rust
async fn add_sub_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError>
// Calls: member_service::add_sub_member(&pool, &id, &input, &actor)
// Role: ADMIN or OPERATOR only
// Returns: { ok: true, subMemberId, memberId }

async fn update_sub_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError>
// Calls: member_service::update_sub_member(&pool, &id, &input, &actor)
// Role: ADMIN or OPERATOR only

async fn remove_sub_member(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError>
// Calls: member_service::remove_sub_member(&pool, &id, &actor)
// Role: ADMIN or OPERATOR only
```

Input shape for `add_sub_member` body (mirrors DPS):
```json
{ "name": "...", "email": "...", "phone": "...", "relation": "..." }
```

Input shape for `update_sub_member` body:
```json
{ "name": "...", "phone": "...", "relation": "...", "canLogin": true }
```

---

#### T7A-2 — Fix approvals route (split GET/POST, add individual GET)
**File:** `apps/backend/src/routes/approvals.rs`

Replace the current buggy route registration:
```rust
// REMOVE THIS:
.route("/:id", get(review_approval))

// REPLACE WITH:
.route("/:id", get(get_approval))
.route("/:id/approve", post(approve_entry))
.route("/:id/reject", post(reject_entry))
```

New handlers:

```rust
// GET /:id — view single approval (read only, no body)
async fn get_approval(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError>
// Role: ADMIN only
// Calls: approval_service::get_approval(&pool, &id)
// Returns: full approval record with previousData and newData

// POST /:id/approve — apply the proposed change
async fn approve_entry(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError>
// Role: ADMIN only
// Body: { "notes": "..." }   (optional)
// Calls: approval_service::approve_entry(&pool, &id, &reviewer, notes)
// Returns: { ok: true }

// POST /:id/reject — discard the proposed change
async fn reject_entry(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError>
// Role: ADMIN only
// Body: { "notes": "..." }   (optional)
// Calls: approval_service::reject_entry(&pool, &id, &reviewer, notes)
// Returns: { ok: true }
```

Also add `get_approval` to `approval_service` (see T7B-4).

---

#### T7A-3 — Transaction reject endpoint
**File:** `apps/backend/src/routes/transactions.rs`

Add a sub-route for admin rejection:

```rust
// Add to router():
.route("/:id/reject", post(reject_transaction))
```

Handler:

```rust
async fn reject_transaction(
    AuthSession(claims): AuthSession,
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError>
// Role: ADMIN only
// Body: { "notes": "..." }   (optional)
// Calls: transaction_service::reject_transaction(&pool, &id, &actor)
// Returns: { ok: true }
```

Also remove `update_transaction` and `delete_transaction` handlers entirely (or return 405 Method Not Allowed). Transactions are immutable; PATCH and DELETE on `/:id` must not exist. **Read DPS's transaction route and confirm it has no PATCH/DELETE on /:id either.**

---

#### T7A-4 — Role enforcement in all handlers
**File:** All files in `apps/backend/src/routes/`

Every handler currently does `let _ = claims;`. Replace with role checks.

Pattern to apply:

```rust
// At top of handler, after extracting claims:
use crate::auth::permissions::{Role, can_access_route};

let role = Role::from_str(&claims.role).ok_or(AppError::Unauthorized)?;
if !can_access_route(role, "/api/approvals") {
    return Err(AppError::Forbidden);
}
```

Per-route role requirements (from `permissions.rs`):

| Route module | Required role (minimum) |
|---|---|
| `approvals.rs` — all handlers | ADMIN |
| `members.rs` — `create_member`, `update_member`, `delete_member` | ADMIN or OPERATOR |
| `members.rs` — `list_members`, `get_member`, `list_sub_members` | ADMIN, OPERATOR, or ORGANISER |
| `members.rs` — `add_sub_member`, `update_sub_member`, `remove_sub_member` | ADMIN or OPERATOR |
| `transactions.rs` — `create_transaction`, `reject_transaction` | ADMIN or OPERATOR |
| `transactions.rs` — `list_transactions`, `get_transaction`, `transaction_summary` | ADMIN, OPERATOR, or ORGANISER |
| `memberships.rs` — all | ADMIN, OPERATOR, ORGANISER, or MEMBER |
| `audit_log.rs`, `activity_log.rs` | ADMIN, OPERATOR, or ORGANISER |
| `sponsors.rs`, `sponsor_links.rs` | ADMIN or OPERATOR |
| `cron.rs` | ADMIN |
| `dashboard.rs` | ADMIN, OPERATOR, ORGANISER, or MEMBER |

---

#### T7A-5 — Fix memberships list
**File:** `apps/backend/src/routes/memberships.rs`

Replace the current `list_memberships` handler body:

```rust
// CURRENT (broken — returns [] if no member_id):
if let Some(member_id) = &q.member_id {
    ...
}
Ok(Json(serde_json::json!([])))

// REPLACE WITH (match DPS behavior):
// If member_id provided → get_memberships_by_member
// Otherwise → list all with pagination
async fn list_memberships(...) {
    if let Some(member_id) = &q.member_id {
        let items = membership_service::get_memberships_by_member(&pool, member_id).await?;
        return Ok(Json(serde_json::json!(items)));
    }
    // Full paginated list
    let items = membership_service::list_memberships(&pool, q.page, q.limit).await?;
    Ok(Json(serde_json::json!(items)))
}
```

Also add `page` and `limit` to `ListQuery` struct.

---

#### T7A-6 — Add notifications route
**File:** `apps/backend/src/routes/` — create `notifications.rs`

```rust
// POST /api/notifications/whatsapp
// Role: ADMIN only
// Body: { "phone": "...", "templateName": "...", "params": [...] }
// Calls: notification_service::send_manual_whatsapp(...)
// Returns: { ok: true }
```

Wire into `main.rs`: `.nest("/api/notifications", routes::notifications::router())`

Read DPS source: `../dps-dashboard/src/app/api/notifications/whatsapp/route.ts`

---

### Wave 7A Done When

- [ ] `GET/POST/PUT/DELETE /api/members/:id/sub-members` all return correct responses
- [ ] `GET /api/approvals/:id` returns single approval record
- [ ] `POST /api/approvals/:id/approve` applies change and returns `{ ok: true }`
- [ ] `POST /api/approvals/:id/reject` discards change and returns `{ ok: true }`
- [ ] `POST /api/transactions/:id/reject` rejects transaction
- [ ] MEMBER calling `POST /api/approvals/:id/approve` gets 403
- [ ] ORGANISER calling `POST /api/transactions` gets 403
- [ ] `GET /api/memberships` (no params) returns paginated list
- [ ] `POST /api/notifications/whatsapp` sends WhatsApp and returns `{ ok: true }`
- [ ] `cargo build` succeeds

---

## Wave 7B — `backend-services`

**Agent:** `backend-services`
**Runs parallel with:** 7A, 7C

### Context files to read first

```
# DPS sources
../dps-dashboard/src/lib/services/member-service.ts      (addSubMember, updateSubMember, removeSubMember)
../dps-dashboard/src/lib/services/membership-service.ts  (applyMembershipStatusFromTransaction)
../dps-dashboard/src/lib/services/approval-service.ts    (approveEntry MEMBERSHIP branch)
../dps-dashboard/src/lib/validators.ts                   (all Zod schemas for validation rules)
../dps-dashboard/src/lib/audit.ts                        (logAudit, logActivity)

# BSDS files to modify
apps/backend/src/services/member_service.rs
apps/backend/src/services/membership_service.rs
apps/backend/src/services/approval_service.rs
apps/backend/src/support/audit.rs
apps/backend/src/support/validation.rs
apps/backend/src/repositories/members.rs
apps/backend/src/repositories/users.rs
```

### Task list

#### T7B-1 — Add sub-member write operations to member_service
**File:** `apps/backend/src/services/member_service.rs`

Add three functions (porting from DPS `addSubMember`, `updateSubMember`, `removeSubMember`):

```rust
pub async fn add_sub_member(
    pool: &SqlitePool,
    parent_user_id: &str,
    input: &AddSubMemberInput,
    actor: &RequestedBy,
) -> Result<AddSubMemberResult, MemberServiceError>
```

Business rules to enforce:
- Count existing sub-members for `parent_user_id` — if ≥ 3, return `Err(MemberServiceError::MaxSubMembersReached)`
- Generate sub-member `memberId` using `support::member_id::generate_sub_member_id(parent_member_id, index)`
- Generate temp password using `auth::temp_password::generate_temp_password()`
- Hash password with bcrypt (`auth::mod::hash_password()`)
- Insert into `sub_members` table
- Log to `activity_logs`: action = `"sub_member_added"`, description includes parent member name

```rust
pub async fn update_sub_member(
    pool: &SqlitePool,
    sub_member_id: &str,
    input: &UpdateSubMemberInput,
    actor: &RequestedBy,
) -> Result<(), MemberServiceError>
```

Business rules:
- Verify sub-member exists
- Update `name`, `phone`, `relation`, `can_login` fields
- Log to `activity_logs`: action = `"sub_member_updated"`

```rust
pub async fn remove_sub_member(
    pool: &SqlitePool,
    sub_member_id: &str,
    actor: &RequestedBy,
) -> Result<(), MemberServiceError>
```

Business rules:
- Verify sub-member exists
- Hard delete from `sub_members` (no status field — same as DPS)
- Log to `activity_logs`: action = `"sub_member_removed"`

---

#### T7B-2 — Fix approve_membership to update all User fields
**File:** `apps/backend/src/services/membership_service.rs`

Read DPS `applyMembershipStatusFromTransaction()` carefully. When a membership is approved, the following `users` fields must be updated atomically **in a single SQLx transaction**:

```rust
// When membership.fee_type == "SUBSCRIPTION":
users.membership_status = 'ACTIVE'
users.membership_type   = membership.type       // MONTHLY / HALF_YEARLY / ANNUAL
users.membership_start  = membership.start_date
users.membership_expiry = membership.end_date
users.total_paid        += membership.amount    // increment, do not overwrite

// When membership.is_application_fee == true:
users.application_fee_paid = 1
users.total_paid           += membership.amount

// When membership.fee_type == "ANNUAL_FEE":
users.annual_fee_paid   = 1
users.annual_fee_start  = today
users.annual_fee_expiry = today + 365 days
users.total_paid        += membership.amount
```

All updates must happen inside the same `BEGIN TRANSACTION … COMMIT` block as the `memberships.status = 'APPROVED'` update.

Also emit:
- `audit_logs` entry: `event_type = 'TRANSACTION_APPROVED'` with full transaction snapshot
- `activity_logs` entry: action = `"membership_approved"`

---

#### T7B-3 — Verify TRANSACTION_CREATED audit log on creation
**File:** `apps/backend/src/services/transaction_service.rs`

Read `create_transaction()`. After inserting the transaction row, confirm it calls `audit::log_audit()` with `event_type = TRANSACTION_CREATED`. If not, add:

```rust
audit::log_audit(pool, AuditLogParams {
    transaction_id: &new_tx_id,
    event_type: "TRANSACTION_CREATED",
    transaction_snapshot: &build_transaction_snapshot(&tx),
    performed_by_id: &actor.id,
}).await?;

activity::log_activity(pool, ActivityLogParams {
    user_id: &actor.id,
    action: "transaction_created",
    description: &format!("Transaction {} created for ₹{}", new_tx_id, input.amount),
    metadata: None,
}).await?;
```

Also confirm that when a ADMIN creates a transaction directly (bypassing approval), it also emits `TRANSACTION_APPROVED` immediately after `TRANSACTION_CREATED`.

---

#### T7B-4 — Add `get_approval` to approval_service
**File:** `apps/backend/src/services/approval_service.rs`

Add:

```rust
pub async fn get_approval(
    pool: &SqlitePool,
    approval_id: &str,
) -> Result<ApprovalDetail, ApprovalServiceError>
```

Returns: full approval record including `previousData` and `newData` as parsed JSON, plus `requestedBy` user info. Same shape as DPS `getApprovalById()`.

---

#### T7B-5 — Consolidate all activity log action strings
**Files:** `member_service.rs`, `approval_service.rs`, `transaction_service.rs`, `membership_service.rs`

Replace all ad-hoc action strings with the consolidated scheme below. The `action` column in `activity_logs` must use exactly these strings — the frontend activity-log filter dropdown depends on them.

**Key implementation notes:**
- `approval_service::get_approval_activity_action()` currently only receives `entity_type`. It must also receive `approval.action` to distinguish sub-member vs primary-member flows within the same `MEMBER_*` entity type.
- The generic `"approval_submitted"` string is retired — each call site in `member_service.rs` and `transaction_service.rs` must emit its own specific string.

##### Member operations — `member_service.rs`

| Context | Action string |
|---|---|
| `create_member` — OPERATOR submits (MEMBER_ADD) | `"member_add_requested"` |
| `create_member` — ADMIN creates directly | `"member_add_approved"` |
| `update_member` — OPERATOR submits (MEMBER_EDIT) | `"member_edit_requested"` |
| `update_member` — ADMIN edits directly | `"member_edit_approved"` |
| `delete_member` — OPERATOR submits (MEMBER_DELETE) | `"member_delete_requested"` |
| `delete_member` — ADMIN deletes directly | `"member_delete_approved"` |

##### Sub-member operations — `member_service.rs`

| Context | Action string |
|---|---|
| `add_sub_member` — OPERATOR submits | `"submember_add_requested"` |
| `add_sub_member` — ADMIN creates directly | `"submember_add_approved"` |
| `update_sub_member` — OPERATOR submits | `"submember_edit_requested"` |
| `update_sub_member` — ADMIN edits directly | `"submember_edit_approved"` |
| `remove_sub_member` — OPERATOR submits | `"submember_delete_requested"` |
| `remove_sub_member` — ADMIN removes directly | `"submember_delete_approved"` |

##### Approval outcomes — `approval_service.rs` (`approve_entry` / `reject_entry`)

The function must branch on both `entity_type` and `approval.action`:

| entity_type | approval.action | Approved | Rejected |
|---|---|---|---|
| `MEMBER_ADD` | `add_member` | `"member_add_approved"` | `"member_add_rejected"` |
| `MEMBER_ADD` | `add_sub_member` | `"submember_add_approved"` | `"submember_add_rejected"` |
| `MEMBER_EDIT` | `edit_member` | `"member_edit_approved"` | `"member_edit_rejected"` |
| `MEMBER_EDIT` | `edit_sub_member` | `"submember_edit_approved"` | `"submember_edit_rejected"` |
| `MEMBER_DELETE` | `delete_member` | `"member_delete_approved"` | `"member_delete_rejected"` |
| `MEMBER_DELETE` | `remove_sub_member` | `"submember_delete_approved"` | `"submember_delete_rejected"` |
| `MEMBERSHIP` | any | `"membership_approved"` | `"membership_rejected"` |
| `TRANSACTION` | any | `"transaction_approved"` | `"transaction_rejected"` |

##### Transaction operations — `transaction_service.rs` and `membership_service.rs`

| Context | Action string |
|---|---|
| `create_transaction` — OPERATOR submits (pending approval) | `"transaction_requested"` |
| `create_transaction` — ADMIN creates directly | `"transaction_approved"` |
| `reject_transaction` — ADMIN rejects directly | `"transaction_rejected"` |
| `create_membership` transaction — OPERATOR submits | `"transaction_requested"` |
| `create_membership` transaction — ADMIN creates directly | `"transaction_approved"` |

##### Membership operations — `membership_service.rs`

| Context | Action string |
|---|---|
| `approve_membership` | `"membership_approved"` |
| `reject_membership` | `"membership_rejected"` |

##### Other (unchanged)

| Context | Action string |
|---|---|
| `send_manual_whatsapp` | `"whatsapp_notification_sent"` |
| cron expiry check | `"cron_expiry_check"` |
| cron reminder sent | `"whatsapp_notification_sent"` |
| login success | `"login_success"` |
| login failure | `"login_failed"` |
| logout | `"logout"` |
| change password | `"password_changed"` |
| `sponsor_created` | `"sponsor_created"` |
| `sponsor_updated` | `"sponsor_updated"` |
| `sponsor_deleted` | `"sponsor_deleted"` |
| `sponsor_link_created` | `"sponsor_link_created"` |
| `sponsor_link_deactivated` | `"sponsor_link_deactivated"` |
| webhook sponsor payment | `"sponsor_payment_received"` |

---

#### T7B-6 — Input validation in service layer
**File:** `apps/backend/src/support/validation.rs`

Add validation helpers that mirror DPS's Zod schemas. Call these at the top of each service function before DB operations:

```rust
// Phone: must match +91XXXXXXXXXX (10 digits after +91)
pub fn validate_phone(phone: &str) -> Result<(), ValidationError>

// Email: basic format check
pub fn validate_email(email: &str) -> Result<(), ValidationError>

// Amount: positive, max 2 decimal places
pub fn validate_amount(amount: f64) -> Result<(), ValidationError>

// Enum membership type
pub fn validate_membership_type(t: &str) -> Result<(), ValidationError>

// Enum transaction type
pub fn validate_transaction_type(t: &str) -> Result<(), ValidationError>

// Enum payment mode
pub fn validate_payment_mode(mode: &str) -> Result<(), ValidationError>

// Non-empty string
pub fn require_non_empty(field: &str, value: &str) -> Result<(), ValidationError>
```

Apply in:
- `member_service::create_member` — validate name, phone, email, address
- `member_service::add_sub_member` — validate name, phone, email, relation
- `transaction_service::create_transaction` — validate type, category, amount, payment_mode, purpose
- `membership_service::create_membership` — validate member_id, type, amount

---

### Wave 7B Done When

- [ ] `add_sub_member`, `update_sub_member`, `remove_sub_member` exist in `member_service.rs`
- [ ] Max-3 sub-members enforced — adding 4th returns error
- [ ] `approve_membership` updates users.membership_status, users.membership_expiry, users.total_paid atomically
- [ ] `TRANSACTION_CREATED` audit log entry emitted on every `create_transaction` call
- [ ] All consolidated activity action strings emitted correctly per T7B-5 (member, submember, transaction, membership, approval outcomes, sponsor, notification)
- [ ] `get_approval` returns full approval detail with parsed JSON data
- [ ] Validation helpers exist and are called in member, transaction, membership services
- [ ] `cargo build` succeeds

---

## Wave 7C — `frontend-copy`

**Agent:** `frontend-copy`
**Runs parallel with:** 7A, 7B

### Context files to read first

```
# DPS sources (compare against BSDS frontend)
../dps-dashboard/src/app/dashboard/approvals/page.tsx
../dps-dashboard/src/app/dashboard/approvals/approval-detail.tsx
../dps-dashboard/src/app/dashboard/approvals/constants.ts
../dps-dashboard/src/app/dashboard/cash/page.tsx
../dps-dashboard/src/app/dashboard/members/page.tsx
../dps-dashboard/src/app/dashboard/members/loading.tsx
../dps-dashboard/src/app/dashboard/my-membership/loading.tsx
../dps-dashboard/src/app/dashboard/cash/loading.tsx
../dps-dashboard/src/app/dashboard/approvals/loading.tsx
../dps-dashboard/src/app/dashboard/audit-log/loading.tsx
../dps-dashboard/src/app/dashboard/activity-log/loading.tsx
../dps-dashboard/src/app/dashboard/sponsorship/loading.tsx

# BSDS files to read (compare, then fix)
apps/frontend/app/dashboard/approvals/page.tsx
apps/frontend/app/dashboard/approvals/approval-detail.tsx
apps/frontend/app/dashboard/cash/page.tsx
apps/frontend/app/dashboard/members/page.tsx
apps/frontend/app/dashboard/audit-log/page.tsx
apps/frontend/app/dashboard/activity-log/page.tsx
```

### Task list

#### T7C-1 — Fix approval page endpoint URLs
**File:** `apps/frontend/app/dashboard/approvals/page.tsx`

Find every place the page calls the backend to approve or reject. Ensure the URLs are:

```typescript
// Approve:
await apiPost(`/api/approvals/${id}/approve`, { notes })

// Reject:
await apiPost(`/api/approvals/${id}/reject`, { notes })

// Fetch individual approval for detail modal:
await apiGet(`/api/approvals/${id}`)
```

If the current page uses the old single-endpoint pattern (`PATCH /api/approvals/:id` with `action` in body), update it to match the new split endpoints above.

---

#### T7C-2 — Add approvals/constants.ts
**File:** `apps/frontend/app/dashboard/approvals/constants.ts`

Copy from DPS `src/app/dashboard/approvals/constants.ts`. This file contains:
- Entity type labels: `{ MEMBER_ADD: "New Member", MEMBER_EDIT: "Edit Member", ... }`
- Action labels and status badge colors
- Display helpers used by both the list page and the detail modal

If already present and correct, mark done. If missing, copy from DPS.

---

#### T7C-3 — Verify approval-detail diff view
**File:** `apps/frontend/app/dashboard/approvals/approval-detail.tsx`

Compare with DPS `src/app/dashboard/approvals/approval-detail.tsx`. Verify:

- For `MEMBER_EDIT`: shows a side-by-side (or before/after) diff of changed fields vs unchanged fields
- For `MEMBER_ADD`: shows the new member data fields
- For `MEMBER_DELETE`: shows the member being suspended with a warning
- For `TRANSACTION`: shows the transaction amount, category, payment mode, and receipt if available
- For `MEMBERSHIP`: shows the membership period, type, and fee amount

If any case is missing or shows raw JSON instead of formatted fields, copy the relevant section from DPS.

---

#### T7C-4 — Verify cash page fee logic
**File:** `apps/frontend/app/dashboard/cash/page.tsx`

Compare with DPS `src/app/dashboard/cash/page.tsx` (1472 lines). Verify these features exist:

1. **Fee checkboxes** — for MEMBERSHIP category transactions, three checkboxes:
   - "Subscription fee" (`includesSubscription`)
   - "Annual membership fee" (`includesAnnualFee`)
   - "Application fee" (`includesApplicationFee`)
   - Total amount auto-calculated from checked boxes

2. **Membership type selector** — shows MONTHLY (₹250), HALF_YEARLY (₹1,500), ANNUAL (₹3,000) with auto-filled amount

3. **Member picker** — search box that queries `/api/members?search=X` and shows dropdown

4. **Operator vs Admin labels** — submit button says "Submit for Approval" for OPERATOR role, "Add Transaction" for ADMIN

5. **Receipt modal** — after successful submission, offer to view/print the receipt

6. **Summary cards** — Total Income, Total Expenses, Pending Approvals, Net Balance at top of page

If any feature is missing, copy the relevant section from DPS (adapting `useSession` → `useAuth` and fetch patterns to match the BSDS API client).

---

#### T7C-5 — Verify members page sub-member UI
**File:** `apps/frontend/app/dashboard/members/page.tsx`

Compare with DPS `src/app/dashboard/members/page.tsx`. Verify these features exist:

1. **Sub-member panel** — when a member row is expanded or a detail panel opens, sub-members are listed
2. **Add sub-member dialog** — form with name, email, phone, relation fields
3. **Edit sub-member** — inline edit for each sub-member
4. **Remove sub-member** — confirmation before deletion
5. **Max-3 indicator** — badge showing "2/3 sub-members" and "Add" button disabled when at limit

If missing, copy the sub-member UI section from DPS.

---

#### T7C-6 — Add all missing loading.tsx skeleton files
**Files:** `apps/frontend/app/dashboard/*/loading.tsx`

For each missing file, copy from DPS:

| Target | DPS source |
|--------|------------|
| `apps/frontend/app/dashboard/members/loading.tsx` | `src/app/dashboard/members/loading.tsx` |
| `apps/frontend/app/dashboard/my-membership/loading.tsx` | `src/app/dashboard/my-membership/loading.tsx` |
| `apps/frontend/app/dashboard/cash/loading.tsx` | `src/app/dashboard/cash/loading.tsx` |
| `apps/frontend/app/dashboard/approvals/loading.tsx` | `src/app/dashboard/approvals/loading.tsx` |
| `apps/frontend/app/dashboard/audit-log/loading.tsx` | `src/app/dashboard/audit-log/loading.tsx` |
| `apps/frontend/app/dashboard/activity-log/loading.tsx` | `src/app/dashboard/activity-log/loading.tsx` |
| `apps/frontend/app/dashboard/sponsorship/loading.tsx` | `src/app/dashboard/sponsorship/loading.tsx` |

Before copying, read the current BSDS `app/dashboard/` directory to see which loading files already exist. Only create the missing ones.

---

#### T7C-7 — Verify audit-log and activity-log pages
**Files:** `apps/frontend/app/dashboard/audit-log/page.tsx` and `activity-log/page.tsx`

Compare each against its DPS counterpart. Verify:

**Audit log:**
- Filter controls: category dropdown, date range pickers
- Table columns: Date/Time, Category, Sender/Receiver, Payment Mode, Amount, Performed By
- Row click → detail modal with: transaction snapshot, linked transaction link, fee breakdown if applicable
- No create/edit/delete UI (read-only)

**Activity log:**
- Filter controls: user search, action dropdown (all 24+ action types), date range pickers
- Table columns: Date/Time, User, Action, Description
- Row click → detail modal with full metadata
- No create/edit/delete UI (read-only)

---

### Wave 7C Done When

- [ ] Approval page calls `POST /api/approvals/:id/approve` and `POST /api/approvals/:id/reject`
- [ ] Approval page calls `GET /api/approvals/:id` for detail modal
- [ ] `approvals/constants.ts` exists with all entity type and action labels
- [ ] Approval detail shows diff view for MEMBER_EDIT, new fields for MEMBER_ADD, warning for MEMBER_DELETE
- [ ] Cash page has fee checkboxes, membership type selector, member picker, summary cards, receipt modal
- [ ] Cash submit button label correct per role
- [ ] Members page has sub-member add/edit/remove UI with max-3 enforcement
- [ ] All 7 `loading.tsx` files exist for dashboard pages
- [ ] Audit log page has category + date filters, correct columns, detail modal
- [ ] Activity log page has user search + action + date filters, correct columns, detail modal
- [ ] `npm run build` succeeds in `apps/frontend/`

---

## Wave 7D — `backend-tests`

**Agent:** `backend-tests`
**Starts after:** 7A + 7B complete
**Runs parallel with:** 7E

### Context files to read first

```
# DPS test sources (read before writing each test file)
../dps-dashboard/tests/unit/security.test.ts
../dps-dashboard/tests/unit/member-lifecycle.test.ts
../dps-dashboard/tests/unit/transaction-membership-link.test.ts
../dps-dashboard/tests/unit/sub-members-route.test.ts
../dps-dashboard/tests/unit/approval-service.test.ts
../dps-dashboard/tests/unit/member-service.test.ts
../dps-dashboard/tests/unit/membership-service.test.ts
../dps-dashboard/tests/unit/transaction-service.test.ts
../dps-dashboard/tests/integration/membership-transaction-flow.test.ts
../dps-dashboard/docs/testing-guide.md

# BSDS test files to augment
apps/backend/tests/common/mod.rs
apps/backend/tests/auth_integration.rs
apps/backend/tests/members_integration.rs
apps/backend/tests/memberships_integration.rs
apps/backend/tests/transactions_integration.rs
apps/backend/tests/approvals_integration.rs
```

### Task list

#### T7D-1 — `security_integration.rs` — RBAC enforcement
**File:** `apps/backend/tests/security_integration.rs` (new file)

This file verifies that role enforcement is correctly applied. Each test calls a protected endpoint with a user of insufficient role and asserts 403.

```rust
//! # Security Integration Tests
//!
//! Covers: RBAC enforcement — correct roles accepted, insufficient roles rejected
//! Protects: apps/backend/src/auth/permissions.rs, all route handlers

// Tests to write:

#[tokio::test]
async fn member_role_cannot_approve_approvals() { ... }
// POST /api/approvals/:id/approve with MEMBER cookie → 403

#[tokio::test]
async fn operator_role_cannot_approve_approvals() { ... }
// POST /api/approvals/:id/approve with OPERATOR cookie → 403

#[tokio::test]
async fn organiser_cannot_create_transactions() { ... }
// POST /api/transactions with ORGANISER cookie → 403

#[tokio::test]
async fn member_cannot_create_transactions() { ... }
// POST /api/transactions with MEMBER cookie → 403

#[tokio::test]
async fn member_cannot_list_members() { ... }
// GET /api/members with MEMBER cookie → 403

#[tokio::test]
async fn unauthenticated_cannot_access_any_api() { ... }
// GET /api/members with no cookie → 401

#[tokio::test]
async fn unauthenticated_cannot_access_approvals() { ... }
// GET /api/approvals with no cookie → 401

#[tokio::test]
async fn operator_cannot_add_sub_member_as_admin_only_action() { ... }
// OPERATOR can add sub-members (allowed); assert 200

#[tokio::test]
async fn organiser_cannot_add_sub_member() { ... }
// POST /api/members/:id/sub-members with ORGANISER → 403

#[tokio::test]
async fn member_cannot_view_audit_log() { ... }
// GET /api/audit-log with MEMBER cookie → 403

#[tokio::test]
async fn admin_can_trigger_cron() { ... }
// POST /api/cron with ADMIN cookie → 200

#[tokio::test]
async fn operator_cannot_trigger_cron() { ... }
// POST /api/cron with OPERATOR cookie → 403
```

---

#### T7D-2 — `member_lifecycle_integration.rs` — full state machine
**File:** `apps/backend/tests/member_lifecycle_integration.rs` (new file)

```rust
//! # Member Lifecycle Integration Tests
//!
//! Covers: full state machine PENDING_APPROVAL → PENDING_PAYMENT → ACTIVE → EXPIRED
//! Protects: member_service, membership_service, approval_service, scheduler/cron_jobs

#[tokio::test]
async fn new_member_starts_as_pending_approval() { ... }
// Create member → GET /api/members/:id → membershipStatus == "PENDING_APPROVAL"

#[tokio::test]
async fn admin_approving_member_moves_to_pending_payment() { ... }
// Create MEMBER_ADD approval → admin approves → user membershipStatus == "PENDING_PAYMENT"

#[tokio::test]
async fn operator_creating_member_queues_for_approval() { ... }
// OPERATOR POST /api/members → returns approvalId, not memberId directly

#[tokio::test]
async fn membership_payment_approval_activates_member() { ... }
// Create membership payment → admin approves → membershipStatus == "ACTIVE"
// Verify: membership_expiry is set, total_paid incremented, membership_type set

#[tokio::test]
async fn application_fee_marks_application_fee_paid() { ... }
// Submit application fee (is_application_fee=true) → approve → application_fee_paid == 1

#[tokio::test]
async fn annual_fee_sets_annual_fee_fields() { ... }
// Submit ANNUAL_FEE type → approve → annual_fee_paid, annual_fee_start, annual_fee_expiry set

#[tokio::test]
async fn total_paid_increments_across_multiple_payments() { ... }
// Submit 3 separate membership payments, approve all → total_paid = sum of all amounts

#[tokio::test]
async fn expired_member_marked_by_cron() { ... }
// Create ACTIVE member with expiry = yesterday → POST /api/cron → membershipStatus == "EXPIRED"

#[tokio::test]
async fn reminder_logged_15_days_before_expiry() { ... }
// Create ACTIVE member with expiry = today+14 → POST /api/cron → activity log has cron_expiry_check

#[tokio::test]
async fn deleting_member_suspends_not_hard_deletes() { ... }
// Admin DELETE /api/members/:id → membershipStatus == "SUSPENDED", record still in DB
```

---

#### T7D-3 — `transaction_membership_link_integration.rs` — fee inclusion flags
**File:** `apps/backend/tests/transaction_membership_link_integration.rs` (new file)

```rust
//! # Transaction–Membership Link Tests
//!
//! Covers: fee inclusion flags, combined payment handling, User field updates on approval
//! Protects: transaction_service, membership_service, approval_service

#[tokio::test]
async fn includes_subscription_flag_creates_membership_record() { ... }
// POST /api/transactions with includesSubscription=true, membershipType="MONTHLY"
// → memberships table has new row linked to member

#[tokio::test]
async fn includes_application_fee_flag_creates_membership_record() { ... }
// POST /api/transactions with includesApplicationFee=true
// → memberships table has is_application_fee=1 row

#[tokio::test]
async fn combined_payment_includes_both_subscription_and_application_fee() { ... }
// POST /api/transactions with includesSubscription=true AND includesApplicationFee=true
// → two membership rows created (one subscription, one application fee)
// → amount must equal subscription fee + application fee exactly

#[tokio::test]
async fn approving_transaction_sets_membership_status_to_approved() { ... }
// Create PENDING transaction with includesSubscription → approve → memberships.status == "APPROVED"

#[tokio::test]
async fn approving_membership_transaction_updates_user_membership_fields() { ... }
// Create and approve subscription transaction → users.membership_status == "ACTIVE"
// users.membership_expiry is set, users.total_paid incremented

#[tokio::test]
async fn rejecting_transaction_sets_membership_status_to_rejected() { ... }
// Create PENDING transaction with includesSubscription → reject → memberships.status == "REJECTED"
// users.membership_status NOT changed

#[tokio::test]
async fn admin_transaction_approved_immediately_without_approval_queue() { ... }
// ADMIN POST /api/transactions → approval_status == "APPROVED" immediately
// audit_logs has TRANSACTION_CREATED and TRANSACTION_APPROVED events

#[tokio::test]
async fn operator_transaction_queued_pending_admin_approval() { ... }
// OPERATOR POST /api/transactions → approval_status == "PENDING", approvals table has entry
```

---

#### T7D-4 — Augment `members_integration.rs` with sub-member tests
**File:** `apps/backend/tests/members_integration.rs` (augment existing)

Add to the existing file:

```rust
#[tokio::test]
async fn add_sub_member_creates_with_unique_member_id() { ... }
// POST /api/members/:id/sub-members → sub_members table has new row
// memberId format: BSDS-YYYY-NNNN-01

#[tokio::test]
async fn cannot_add_more_than_3_sub_members() { ... }
// Add 3 sub-members → 200 each time
// Add 4th sub-member → 400 with error message

#[tokio::test]
async fn update_sub_member_changes_fields() { ... }
// PUT /api/members/:id/sub-members with { name: "New Name" } → row updated in DB

#[tokio::test]
async fn remove_sub_member_hard_deletes_row() { ... }
// DELETE /api/members/:id/sub-members → row no longer in sub_members table

#[tokio::test]
async fn organiser_cannot_add_sub_member() { ... }
// ORGANISER POST /api/members/:id/sub-members → 403

#[tokio::test]
async fn activity_log_records_sub_member_operations() { ... }
// Add, update, remove → activity_logs has entries with correct action strings
```

---

#### T7D-5 — Augment `approvals_integration.rs` with complete flow tests
**File:** `apps/backend/tests/approvals_integration.rs` (augment existing)

Add:

```rust
#[tokio::test]
async fn get_single_approval_returns_full_detail() { ... }
// GET /api/approvals/:id → response has entityType, previousData, newData, requestedBy

#[tokio::test]
async fn approve_member_add_creates_user_and_member() { ... }
// OPERATOR creates member → approval queued
// ADMIN POST /api/approvals/:id/approve
// → users table has new row, members table has new row, memberId assigned

#[tokio::test]
async fn approve_member_edit_updates_member_fields() { ... }
// OPERATOR updates member → approval queued with previousData and newData
// ADMIN approves → member.name == newData.name

#[tokio::test]
async fn approve_member_delete_suspends_user() { ... }
// OPERATOR deletes member → approval queued
// ADMIN approves → users.membership_status == "SUSPENDED"

#[tokio::test]
async fn reject_member_add_leaves_no_user_created() { ... }
// OPERATOR creates member → approval queued
// ADMIN POST /api/approvals/:id/reject
// → users table has NO new row, approval.status == "REJECTED"

#[tokio::test]
async fn approve_transaction_moves_status_to_approved() { ... }
// OPERATOR creates transaction → approval queued
// ADMIN approves → transactions.approval_status == "APPROVED"
// audit_logs has TRANSACTION_APPROVED entry

#[tokio::test]
async fn reject_transaction_moves_status_to_rejected() { ... }
// OPERATOR creates transaction → approval queued
// ADMIN rejects → transactions.approval_status == "REJECTED"
// audit_logs has TRANSACTION_REJECTED entry

#[tokio::test]
async fn approval_notes_saved_on_review() { ... }
// ADMIN approves with notes → approvals.notes == provided notes

#[tokio::test]
async fn operator_cannot_approve_their_own_request() { ... }
// OPERATOR submits → tries to call approve as OPERATOR → 403
```

---

#### T7D-6 — Augment `transactions_integration.rs` with reject + audit tests
**File:** `apps/backend/tests/transactions_integration.rs` (augment existing)

Add:

```rust
#[tokio::test]
async fn admin_can_reject_transaction() { ... }
// POST /api/transactions/:id/reject → approval_status == "REJECTED"
// audit_logs has TRANSACTION_REJECTED entry
// activity_logs has "transaction_rejected" entry

#[tokio::test]
async fn audit_log_has_transaction_created_on_creation() { ... }
// Create any transaction → audit_logs has TRANSACTION_CREATED entry with snapshot

#[tokio::test]
async fn admin_transaction_has_both_created_and_approved_audit_events() { ... }
// ADMIN creates transaction → audit_logs has TRANSACTION_CREATED and TRANSACTION_APPROVED
// Both events have correct performedById

#[tokio::test]
async fn operator_cannot_reject_transaction() { ... }
// OPERATOR POST /api/transactions/:id/reject → 403
```

---

#### T7D-7 — Update README.md and COVERAGE.md
**Files:** `apps/backend/tests/README.md`, `apps/backend/tests/COVERAGE.md`

Update both files to reflect the new tests added in 7D-1 through 7D-6.

**README.md** must list all new test files with one-line purpose descriptions.

**COVERAGE.md** must map:
- Every API endpoint to its test(s)
- Every business rule invariant to the test that verifies it
- Mark the following as now covered: RBAC enforcement, member lifecycle state machine, transaction-membership link, sub-member CRUD, audit log completeness

---

### Wave 7D Done When

- [ ] `security_integration.rs` exists with 12 tests — all pass
- [ ] `member_lifecycle_integration.rs` exists with 10 tests — all pass
- [ ] `transaction_membership_link_integration.rs` exists with 8 tests — all pass
- [ ] `members_integration.rs` has 6 new sub-member tests — all pass
- [ ] `approvals_integration.rs` has 9 new flow tests — all pass
- [ ] `transactions_integration.rs` has 4 new audit/reject tests — all pass
- [ ] `README.md` and `COVERAGE.md` updated to reflect new coverage
- [ ] `cargo test` passes — all tests green

---

## Wave 7E — `frontend-tests`

**Agent:** `frontend-tests`
**Starts after:** 7A + 7B + 7C complete
**Runs parallel with:** 7D

### Context files to read first

```
# DPS test sources (read before writing each test)
../dps-dashboard/tests/unit/auth.test.ts
../dps-dashboard/tests/unit/approval-service.test.ts
../dps-dashboard/tests/unit/member-service.test.ts
../dps-dashboard/tests/unit/membership-service.test.ts
../dps-dashboard/tests/unit/transaction-service.test.ts
../dps-dashboard/tests/unit/route-handlers-core.test.ts
../dps-dashboard/tests/unit/public-route-handlers.test.ts

# BSDS test files to augment
apps/frontend/tests/setup.ts
apps/frontend/tests/components/layout.test.tsx
apps/frontend/tests/components/pages.test.tsx
apps/frontend/tests/unit/utils.test.ts
```

### Task list

#### T7E-1 — `tests/unit/auth-flow.test.ts`
**File:** `apps/frontend/tests/unit/auth-flow.test.ts` (new file)

```typescript
// Mock: lib/auth-client.ts (login, logout, getMe, changePassword)
// Mock: lib/api-client.ts (all fetch calls)

// Tests:
// login_with_valid_credentials_sets_auth_state()
// login_with_invalid_credentials_shows_error()
// logout_clears_auth_state()
// temp_password_user_redirected_to_change_password()
// change_password_success_clears_temp_password_flag()
// unauthenticated_user_redirected_to_login_from_dashboard()
// session_loaded_on_mount_via_getMe()
```

---

#### T7E-2 — `tests/unit/approval-workflow.test.ts`
**File:** `apps/frontend/tests/unit/approval-workflow.test.ts` (new file)

```typescript
// Mock: apiPost, apiGet from lib/api-client.ts

// Tests:
// approval_list_fetches_from_correct_endpoint()
// clicking_approval_row_fetches_GET_approvals_id()
// approving_calls_POST_approvals_id_approve()
// rejecting_calls_POST_approvals_id_reject()
// approval_detail_shows_diff_for_member_edit()
// approval_detail_shows_warning_for_member_delete()
// approval_detail_shows_transaction_fields_for_transaction_type()
// notes_field_included_in_approve_reject_payload()
// approval_list_revalidates_after_action()
```

---

#### T7E-3 — `tests/unit/cash-fee-logic.test.ts`
**File:** `apps/frontend/tests/unit/cash-fee-logic.test.ts` (new file)

```typescript
// Pure logic tests (no mocks needed) for fee calculation

// Tests:
// selecting_monthly_subscription_sets_amount_250()
// selecting_half_yearly_subscription_sets_amount_1500()
// selecting_annual_subscription_sets_amount_3000()
// checking_application_fee_adds_10000()
// checking_annual_fee_adds_5000()
// combined_subscription_and_application_fee_sums_correctly()
// unchecking_all_fees_resets_amount_to_zero()
// operator_submit_button_label_is_submit_for_approval()
// admin_submit_button_label_is_add_transaction()
```

---

#### T7E-4 — `tests/unit/member-submember.test.ts`
**File:** `apps/frontend/tests/unit/member-submember.test.ts` (new file)

```typescript
// Mock: apiPost, apiPut, apiDelete from lib/api-client.ts

// Tests:
// add_sub_member_calls_POST_members_id_sub_members()
// update_sub_member_calls_PUT_members_id_sub_members()
// remove_sub_member_calls_DELETE_members_id_sub_members()
// add_button_disabled_when_3_sub_members_exist()
// max_sub_member_badge_shows_correct_count()
// sub_member_form_validates_required_fields()
// sub_member_list_revalidates_after_add()
```

---

#### T7E-5 — Augment `tests/components/pages.test.tsx`
**File:** `apps/frontend/tests/components/pages.test.tsx` (augment existing)

Add render tests for:
- Approvals page renders approval list
- Approvals page renders loading state
- Cash page renders summary cards
- Members page renders sub-member panel
- Audit log page renders with filters
- Activity log page renders with filters

---

#### T7E-6 — Update `tests/README.md`
**File:** `apps/frontend/tests/README.md`

Update to reflect the new test files added in 7E-1 through 7E-5. Include:
- Table of all test files with purpose
- Covered flows
- Remaining known gaps

---

### Wave 7E Done When

- [ ] `tests/unit/auth-flow.test.ts` exists with 7 tests — all pass
- [ ] `tests/unit/approval-workflow.test.ts` exists with 9 tests — all pass
- [ ] `tests/unit/cash-fee-logic.test.ts` exists with 9 tests — all pass
- [ ] `tests/unit/member-submember.test.ts` exists with 7 tests — all pass
- [ ] `pages.test.tsx` has render tests for new pages — all pass
- [ ] `tests/README.md` updated
- [ ] `npm test` passes in `apps/frontend/`

---

## Acceptance Criteria (Full Parity)

All of the following must pass before Wave 7 is declared complete:

### Approval Workflow (E2E)
- [ ] OPERATOR creates member → approval appears in queue
- [ ] ADMIN views queue → sees MEMBER_ADD entry
- [ ] ADMIN opens detail → sees name, email, phone, address fields
- [ ] ADMIN approves → User + Member created, temp password auto-generated
- [ ] Approved member can log in with temp password
- [ ] ADMIN rejects → approval.status == "REJECTED", no User created
- [ ] MEMBER role cannot view or action the approvals page → 403

### Cash / Transaction Approval (E2E)
- [ ] OPERATOR submits cash-in with subscription fee checked → transaction PENDING, approval queued
- [ ] ADMIN approves → transaction APPROVED, audit log TRANSACTION_APPROVED, member ACTIVE
- [ ] ADMIN rejects → transaction REJECTED, audit log TRANSACTION_REJECTED, member NOT activated
- [ ] ADMIN submits cash-in directly → transaction APPROVED immediately, no approval queue
- [ ] ORGANISER cannot submit cash-in → frontend shows no "Add" button; backend returns 403

### Sub-Members (E2E)
- [ ] Admin adds sub-member → BSDS-YYYY-NNNN-01 memberId created
- [ ] Sub-member can log in with temp password
- [ ] Admin tries to add 4th sub-member → error shown in UI, 400 from backend
- [ ] Admin edits sub-member → name changes in DB and UI
- [ ] Admin removes sub-member → record deleted, no longer in list

### Membership Lifecycle (E2E)
- [ ] New member: membershipStatus = PENDING_APPROVAL
- [ ] After MEMBER_ADD approved: membershipStatus = PENDING_PAYMENT
- [ ] After subscription transaction approved: membershipStatus = ACTIVE, expiry set
- [ ] After expiry date passes and cron runs: membershipStatus = EXPIRED
- [ ] 15 days before expiry and cron runs: activity_log has cron reminder entry

### Audit & Activity Logs (E2E)
- [ ] Every transaction creation writes TRANSACTION_CREATED audit event
- [ ] Every approval of a transaction writes TRANSACTION_APPROVED audit event
- [ ] Every rejection of a transaction writes TRANSACTION_REJECTED audit event
- [ ] Audit log is append-only — no delete or update endpoints exposed
- [ ] Activity log records login_success, member_created, sub_member_added, transaction_created, approval_reviewed
- [ ] Activity log is append-only

### Frontend Layout (visual parity)
- [ ] Approval page shows split approve/reject buttons (not a single toggle)
- [ ] Approval detail shows before/after diff for MEMBER_EDIT
- [ ] Cash page fee checkboxes auto-calculate amount
- [ ] Members page has inline sub-member management panel
- [ ] All dashboard pages have Suspense loading skeletons
- [ ] Audit log page has category and date filters with correct column layout
- [ ] Activity log page has action dropdown showing all 24+ action types

---

## Wave 7 Dependency Graph

```
                    ┌─────────────────────┐
                    │  Wave 6 complete    │
                    │  (routes, tests,    │
                    │   seed done)        │
                    └────────┬────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        [Wave 7A]      [Wave 7B]      [Wave 7C]
     backend-routes  backend-services frontend-copy
      (route fixes)   (service fixes)  (UI fixes)
              │              │              │
              └──────────────┼──────────────┘
                             │ all three complete
                             ▼
              ┌──────────────┴──────────────┐
              ▼                             ▼
        [Wave 7D]                     [Wave 7E]
     backend-tests                  frontend-tests
    (E2E + security)               (UI + workflow)
              │                             │
              └──────────────┬──────────────┘
                             ▼
                    ┌─────────────────────┐
                    │   Wave 7 DONE       │
                    │  Full parity with   │
                    │   dps-dashboard     │
                    └─────────────────────┘
```

---

## Quick Reference — Agent → Files

| Agent | Reads | Writes |
|-------|-------|--------|
| `backend-routes` (7A) | `../dps-dashboard/src/app/api/members/[id]/sub-members/route.ts`, approvals/[id]/approve, reject, transactions/[id]/reject, notifications/whatsapp | `apps/backend/src/routes/members.rs`, `approvals.rs`, `transactions.rs`, `memberships.rs`, `notifications.rs` |
| `backend-services` (7B) | `../dps-dashboard/src/lib/services/member-service.ts`, membership-service, approval-service, validators, audit | `apps/backend/src/services/member_service.rs`, `membership_service.rs`, `approval_service.rs`, `support/validation.rs` |
| `frontend-copy` (7C) | `../dps-dashboard/src/app/dashboard/approvals/**`, cash/page.tsx, members/page.tsx, all loading.tsx | `apps/frontend/app/dashboard/approvals/**`, `cash/page.tsx`, `members/page.tsx`, all `loading.tsx` |
| `backend-tests` (7D) | All DPS unit + integration tests listed above, existing BSDS test files | `security_integration.rs`, `member_lifecycle_integration.rs`, `transaction_membership_link_integration.rs`, augmented existing files |
| `frontend-tests` (7E) | DPS unit test sources listed above, existing BSDS frontend tests | `tests/unit/auth-flow.test.ts`, `approval-workflow.test.ts`, `cash-fee-logic.test.ts`, `member-submember.test.ts` |
