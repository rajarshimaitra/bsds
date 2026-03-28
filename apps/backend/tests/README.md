# Backend Integration Tests

All tests use isolated `tempfile` SQLite databases and `axum-test` for HTTP-level coverage. Each test binary runs against a fresh DB — no shared state between tests.

## Test File Inventory

| File | Feature / Contract | Tests |
|---|---|---|
| `auth_integration.rs` | Login, logout, /me, change-password, session cookie, temp-password guard | 17 |
| `members_integration.rs` | Member CRUD, sub-member constraints (max 3), role-differentiated list/filter, auth guards | 22 |
| `memberships_integration.rs` | List by member, get, create (admin/operator), approve, reject, wrong-amount guard | 10 |
| `transactions_integration.rs` | List, get, summary, create (admin=direct / operator=pending), immutability guard, auth | 12 |
| `approvals_integration.rs` | List pending, get single, approve, reject, audit log, member-add approval side-effects, OPERATOR blocked | 15 |
| `sponsors_integration.rs` | List/search, get, create, update, delete, auth | 13 |
| `sponsor_links_integration.rs` | List/filter, get, create, toggle active, auth | 11 |
| `dashboard_integration.rs` | Stats endpoint shape, auth | 6 |
| `receipts_integration.rs` | Receipt by transaction ID, not found, auth | 4 |
| `webhooks_integration.rs` | Signature guard, ignored events, missing memberId, sponsor routing, idempotency | 6 |
| `cron_integration.rs` | Secret-header bypass, session auth fallback, summary shape | 8 |
| `support_member_id.rs` | Member ID format generation, uniqueness, parsing | 25 |
| `support_encrypt.rs` | Encrypt/decrypt roundtrip, wrong key, empty/null handling | 16 |
| `support_receipt.rs` | Receipt number generation, formatting, uniqueness | 41 |
| `security_integration.rs` | Role-based access: unauthenticated→401, MEMBER/ORGANISER/OPERATOR/ADMIN permission boundaries | 23 |
| `member_lifecycle_integration.rs` | Full state machine: operator submit, admin approve/reject, activity log, operator/admin update+delete differences | 9 |
| `transaction_membership_link_integration.rs` | Audit log entries, operator rejection forbidden, membership record auto-creation on includesSubscription | 8 |

**Total: 246 tests across 17 files**

## Common Infrastructure

`tests/common/mod.rs` provides:

- `test_app()` — spins up a `TestServer` backed by a fresh `tempfile` SQLite DB with migrations applied
- `seed_admin_user()` / `seed_operator_user()` / `seed_member_user()` / `seed_organiser_user()` — insert users with a specific role and return credentials
- `seed_user_with_role(pool, role)` — generic helper; returns `(user_id, cookie_string)` for any role string
- `seed_member()` — insert a member profile linked to a MEMBER user; returns a struct with `id`, `user_id`, `name`
- `seed_member_record(pool, user_id, name)` — insert a raw member row linked to an existing user
- `auth_cookie()` — build a valid `bsds_session=<token>` cookie header for a seeded user
- `create_pending_approval(pool, server, operator_cookie)` — submit a member via operator API and return the resulting `approvalId`

## Major Happy Paths Covered

- Login with correct credentials returns session cookie
- All CRUD endpoints return correct shape for authenticated requests
- Admin can create members, memberships, transactions, sponsors directly (no approval queue)
- Operator can create members and transactions (pending approval); cannot approve
- Admin can approve/reject memberships, approvals, and pending transactions
- Approving a MEMBER_ADD approval creates a linked `users` row
- Rejecting a MEMBER_ADD approval leaves no `users` row in the DB
- Operator update of a member queues a MEMBER_UPDATE approval rather than applying directly
- Admin update of a member applies directly without queuing an approval
- Operator delete of a member queues a MEMBER_DELETE approval; user is not suspended yet
- Admin delete of a member suspends the user immediately
- Admin-created transaction writes both TRANSACTION_CREATED and TRANSACTION_APPROVED audit log entries
- Operator-created transaction writes only TRANSACTION_CREATED; TRANSACTION_APPROVED is absent until admin approves
- Direct rejection of a transaction (`POST /api/transactions/:id/reject`) writes a `transaction_rejected` activity log entry
- Approving a transaction via the approval service writes a TRANSACTION_APPROVED audit log entry
- Transaction with `includesSubscription: true` creates a linked membership record
- Razorpay webhook with correct HMAC and `payment.captured` creates transaction
- Duplicate Razorpay payment ID returns `alreadyProcessed: true`
- Sponsor webhook with `sponsorLinkToken` routes to sponsor handler

## Major Edge / Error Paths Covered

- Unauthenticated requests return 401 on every protected endpoint
- MEMBER role cannot list, create, update, or delete members (403)
- ORGANISER can list members and memberships but cannot create or mutate (403 on write routes)
- OPERATOR cannot access `/api/approvals` (403)
- OPERATOR cannot trigger cron jobs (403)
- OPERATOR cannot reject a transaction via the direct reject route (403)
- ADMIN has full access to all routes
- Sub-member limit (max 3) enforced; fourth sub-member returns 400
- ORGANISER cannot add sub-members; OPERATOR can
- Wrong fee amount for membership type returns 400
- Membership PATCH with unknown action returns 400
- DELETE on a transaction returns 405 (no route registered) — confirms immutability
- Missing `memberId` in Razorpay notes returns 400
- Invalid HMAC signature returns 401
- Approval notes submitted during rejection are persisted to the `approvals` row

## Known Gaps

- Payment flow (create-order, verify) requires Razorpay API — not integration-tested; covered by unit-level mocks in dps-dashboard
- WhatsApp notification side-effects are fire-and-forget — not asserted
- Cron-driven expiry transitions require time manipulation — tested at the service unit level only
- Receipt PDF generation not tested (no PDF renderer in test environment)
- Concurrent session conflicts require load testing, not integration tests
- `audit-log` and `activity-log` list endpoint data shapes are auth-guarded only; full field coverage is verified indirectly via transaction/member flows
