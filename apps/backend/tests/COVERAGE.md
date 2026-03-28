# Backend Test Coverage Map

Maps each API area and business rule to the test file(s) that verify it.

## API Surface Coverage

| API Area | Route File | Integration Test |
|---|---|---|
| `POST /api/auth/login` | `routes/auth.rs` | `auth_integration.rs` |
| `POST /api/auth/logout` | `routes/auth.rs` | `auth_integration.rs` |
| `GET /api/auth/me` | `routes/auth.rs` | `auth_integration.rs` |
| `POST /api/auth/change-password` | `routes/auth.rs` | `auth_integration.rs` |
| `GET /api/members` | `routes/members.rs` | `members_integration.rs`, `security_integration.rs` |
| `POST /api/members` | `routes/members.rs` | `members_integration.rs`, `member_lifecycle_integration.rs`, `security_integration.rs` |
| `GET /api/members/:id` | `routes/members.rs` | `members_integration.rs` |
| `PATCH /api/members/:id` | `routes/members.rs` | `members_integration.rs`, `member_lifecycle_integration.rs` |
| `DELETE /api/members/:id` | `routes/members.rs` | `member_lifecycle_integration.rs` |
| `GET /api/members/:id/sub-members` | `routes/members.rs` | `members_integration.rs` |
| `POST /api/members/:id/sub-members` | `routes/members.rs` | `members_integration.rs`, `security_integration.rs` |
| `DELETE /api/members/:id/sub-members/:sub_id` | `routes/members.rs` | `members_integration.rs` |
| `GET /api/memberships` | `routes/memberships.rs` | `memberships_integration.rs`, `security_integration.rs` |
| `POST /api/memberships` | `routes/memberships.rs` | `memberships_integration.rs` |
| `GET /api/memberships/:id` | `routes/memberships.rs` | `memberships_integration.rs` |
| `PATCH /api/memberships/:id` | `routes/memberships.rs` | `memberships_integration.rs` |
| `GET /api/transactions` | `routes/transactions.rs` | `transactions_integration.rs`, `security_integration.rs` |
| `POST /api/transactions` | `routes/transactions.rs` | `transactions_integration.rs`, `transaction_membership_link_integration.rs` |
| `GET /api/transactions/:id` | `routes/transactions.rs` | `transactions_integration.rs` |
| `POST /api/transactions/:id/reject` | `routes/transactions.rs` | `transaction_membership_link_integration.rs` |
| `GET /api/transactions/summary` | `routes/transactions.rs` | `transactions_integration.rs` |
| `GET /api/approvals` | `routes/approvals.rs` | `approvals_integration.rs`, `member_lifecycle_integration.rs`, `security_integration.rs` |
| `GET /api/approvals/:id` | `routes/approvals.rs` | `approvals_integration.rs` |
| `POST /api/approvals/:id/approve` | `routes/approvals.rs` | `approvals_integration.rs`, `member_lifecycle_integration.rs`, `transaction_membership_link_integration.rs` |
| `POST /api/approvals/:id/reject` | `routes/approvals.rs` | `approvals_integration.rs`, `member_lifecycle_integration.rs` |
| `GET /api/sponsors` | `routes/sponsors.rs` | `sponsors_integration.rs` |
| `POST /api/sponsors` | `routes/sponsors.rs` | `sponsors_integration.rs` |
| `GET /api/sponsors/:id` | `routes/sponsors.rs` | `sponsors_integration.rs` |
| `PATCH /api/sponsors/:id` | `routes/sponsors.rs` | `sponsors_integration.rs` |
| `DELETE /api/sponsors/:id` | `routes/sponsors.rs` | `sponsors_integration.rs` |
| `GET /api/sponsor-links` | `routes/sponsor_links.rs` | `sponsor_links_integration.rs` |
| `POST /api/sponsor-links` | `routes/sponsor_links.rs` | `sponsor_links_integration.rs` |
| `GET /api/sponsor-links/:id` | `routes/sponsor_links.rs` | `sponsor_links_integration.rs` |
| `PATCH /api/sponsor-links/:id` | `routes/sponsor_links.rs` | `sponsor_links_integration.rs` |
| `GET /api/dashboard/stats` | `routes/dashboard.rs` | `dashboard_integration.rs` |
| `GET /api/receipts/:id` | `routes/receipts.rs` | `receipts_integration.rs` |
| `POST /api/webhooks/razorpay` | `routes/webhooks.rs` | `webhooks_integration.rs` |
| `POST /api/cron` | `routes/cron.rs` | `cron_integration.rs`, `security_integration.rs` |
| `GET /api/audit-log` | `routes/audit_log.rs` | *(auth guard only; data written and verified via transaction/member flows)* |
| `GET /api/activity-log` | `routes/activity_log.rs` | *(auth guard only)* |
| `GET /api/my-membership` | `routes/my_membership.rs` | *(auth guard only)* |
| `POST /api/payments/create-order` | `routes/payments.rs` | **Not covered** — requires live Razorpay API |
| `POST /api/payments/verify` | `routes/payments.rs` | **Not covered** — requires live Razorpay API |
| `POST /api/payments/sponsor-order` | `routes/payments.rs` | **Not covered** — requires live Razorpay API |
| `POST /api/payments/sponsor-verify` | `routes/payments.rs` | **Not covered** — requires live Razorpay API |

## Business Rule Coverage

| Business Rule | Source | Test File(s) |
|---|---|---|
| Passwords are bcrypt-hashed | `auth/mod.rs` | `auth_integration.rs` |
| Temp password forces change-password redirect | `auth/mod.rs` | `auth_integration.rs` |
| Session cookie created on login | `auth/mod.rs` | `auth_integration.rs` |
| Unauthenticated requests → 401 | `auth/mod.rs` | all `*_integration.rs`, `security_integration.rs` |
| MEMBER role cannot manage members/memberships | `auth/permissions.rs` | `security_integration.rs` |
| ORGANISER can read but not mutate members/memberships | `auth/permissions.rs` | `security_integration.rs` |
| OPERATOR cannot access approvals | `auth/permissions.rs` | `security_integration.rs` |
| OPERATOR cannot trigger cron | `auth/permissions.rs` | `security_integration.rs` |
| ADMIN has full access to all routes | `auth/permissions.rs` | `security_integration.rs` |
| OPERATOR submit creates a pending_approval (not direct) | `services/member_service.rs` | `member_lifecycle_integration.rs` |
| ADMIN submit applies directly without queuing approval | `services/member_service.rs` | `member_lifecycle_integration.rs` |
| Approving MEMBER_ADD creates a linked user record | `services/approval_service.rs` | `member_lifecycle_integration.rs`, `approvals_integration.rs` |
| Rejecting MEMBER_ADD leaves no user record | `services/approval_service.rs` | `member_lifecycle_integration.rs`, `approvals_integration.rs` |
| OPERATOR update queues MEMBER_UPDATE approval | `services/member_service.rs` | `member_lifecycle_integration.rs` |
| ADMIN update applies directly | `services/member_service.rs` | `member_lifecycle_integration.rs` |
| OPERATOR delete queues MEMBER_DELETE approval (user not yet suspended) | `services/member_service.rs` | `member_lifecycle_integration.rs` |
| ADMIN delete suspends user immediately | `services/member_service.rs` | `member_lifecycle_integration.rs` |
| Activity log written on operator member submission | `services/member_service.rs` | `member_lifecycle_integration.rs` |
| Approval notes are persisted to `approvals.notes` | `services/approval_service.rs` | `member_lifecycle_integration.rs`, `approvals_integration.rs` |
| Max 3 sub-members per primary member | `services/member_service.rs` | `members_integration.rs` |
| ORGANISER cannot add sub-members | `auth/permissions.rs` | `members_integration.rs` |
| OPERATOR can add sub-members | `auth/permissions.rs` | `members_integration.rs` |
| Membership types: MONTHLY/HALF_YEARLY/ANNUAL | `support/membership_rules.rs` | `memberships_integration.rs` |
| Wrong fee amount returns 400 | `services/membership_service.rs` | `memberships_integration.rs` |
| Application fee one-time only | `services/membership_service.rs` | `memberships_integration.rs` |
| OPERATOR cannot approve memberships | `auth/permissions.rs` | `memberships_integration.rs` |
| Transactions are immutable (DELETE → 405) | `services/transaction_service.rs` | `transactions_integration.rs` |
| Transaction summary aggregates income/expenses | `repositories/transactions.rs` | `transactions_integration.rs` |
| Admin transaction writes TRANSACTION_CREATED + TRANSACTION_APPROVED audit entries | `services/transaction_service.rs` | `transaction_membership_link_integration.rs` |
| Operator transaction writes only TRANSACTION_CREATED audit entry (no APPROVED yet) | `services/transaction_service.rs` | `transaction_membership_link_integration.rs` |
| Direct rejection writes `transaction_rejected` to `activity_logs` | `routes/transactions.rs` | `transaction_membership_link_integration.rs` |
| Approval-path approval writes TRANSACTION_APPROVED to `audit_logs` | `services/approval_service.rs` | `transaction_membership_link_integration.rs`, `approvals_integration.rs` |
| OPERATOR cannot use direct reject route (403) | `auth/permissions.rs` | `transaction_membership_link_integration.rs` |
| Transaction with `includesSubscription` creates membership record | `services/transaction_service.rs` | `transaction_membership_link_integration.rs` |
| Razorpay HMAC verification | `integrations/razorpay.rs` | `webhooks_integration.rs` |
| Webhook idempotency (duplicate payment ID) | `routes/webhooks.rs` | `webhooks_integration.rs` |
| Sponsor payment routing via sponsorLinkToken | `services/webhook_sponsor_handler.rs` | `webhooks_integration.rs` |
| Missing memberId in webhook → 400 | `routes/webhooks.rs` | `webhooks_integration.rs` |
| Cron runs without session (secret header) | `routes/cron.rs` | `cron_integration.rs` |
| Member ID format: BSDS-YYYY-NNNN-NN | `support/member_id.rs` | `support_member_id.rs` |
| Encryption/decryption roundtrip (AES-GCM) | `support/encrypt.rs` | `support_encrypt.rs` |
| Wrong decryption key returns error | `support/encrypt.rs` | `support_encrypt.rs` |
| Receipt number generation and formatting | `support/receipt.rs` | `support_receipt.rs` |

## Audit Log vs Activity Log Distinction

This is a non-obvious behavioral invariant that is explicitly protected by tests:

| Log Table | Written By | Event | Test |
|---|---|---|---|
| `audit_logs` | `transaction_service` | `TRANSACTION_CREATED` | `transaction_membership_link_integration.rs` |
| `audit_logs` | `transaction_service` (admin path) | `TRANSACTION_APPROVED` | `transaction_membership_link_integration.rs` |
| `audit_logs` | `approval_service` | `TRANSACTION_APPROVED` | `transaction_membership_link_integration.rs` |
| `activity_logs` | `routes/transactions` direct reject | `transaction_rejected` | `transaction_membership_link_integration.rs` |
| `activity_logs` | `member_service` | operator submission | `member_lifecycle_integration.rs` |

The direct `POST /api/transactions/:id/reject` route writes to `activity_logs` only.
The approval-service `POST /api/approvals/:id/approve` path writes to `audit_logs` only.
Both paths are separately tested to prevent regression.

## Explicitly Untested Areas

| Area | Reason |
|---|---|
| Razorpay payment create/verify | Requires live Razorpay API keys and order IDs |
| WhatsApp notifications | Fire-and-forget side effect; no observable HTTP response change |
| Cron-driven auto-expiry timing | Requires time manipulation; scheduler logic tested at service level |
| PDF receipt rendering | No PDF renderer available in test environment |
| Concurrent session conflicts | Race conditions require load testing, not integration tests |
| `audit-log` / `activity-log` list endpoint data shape | Auth guard tested; field-level coverage verified indirectly via transaction/member flows |
| PATCH `approvals/:id` (old route style) | API uses `POST /:id/approve` and `POST /:id/reject`; PATCH route does not exist |
| `DELETE /api/transactions/:id` | No DELETE route is registered; axum returns 405; immutability confirmed |
