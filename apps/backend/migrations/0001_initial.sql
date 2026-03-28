-- 0001_initial.sql
-- SQLite schema translated from Prisma/PostgreSQL schema (dps-dashboard)
-- Generated: 2026-03-22

-- ============================================================
-- TABLES
-- ============================================================

CREATE TABLE IF NOT EXISTS users (
    id                   TEXT PRIMARY KEY,
    member_id            TEXT NOT NULL UNIQUE,
    name                 TEXT NOT NULL,
    email                TEXT NOT NULL UNIQUE,
    phone                TEXT NOT NULL,
    address              TEXT NOT NULL,
    password             TEXT NOT NULL,
    is_temp_password     INTEGER NOT NULL DEFAULT 1 CHECK(is_temp_password IN (0,1)),
    role                 TEXT NOT NULL DEFAULT 'MEMBER' CHECK(role IN ('ADMIN','OPERATOR','ORGANISER','MEMBER')),
    membership_status    TEXT NOT NULL DEFAULT 'PENDING_APPROVAL' CHECK(membership_status IN ('PENDING_APPROVAL','PENDING_PAYMENT','ACTIVE','EXPIRED','SUSPENDED')),
    membership_type      TEXT CHECK(membership_type IN ('MONTHLY','HALF_YEARLY','ANNUAL')),
    membership_start     TEXT,
    membership_expiry    TEXT,
    total_paid           NUMERIC NOT NULL DEFAULT 0,
    application_fee_paid INTEGER NOT NULL DEFAULT 0 CHECK(application_fee_paid IN (0,1)),
    annual_fee_start     TEXT,
    annual_fee_expiry    TEXT,
    annual_fee_paid      INTEGER NOT NULL DEFAULT 0 CHECK(annual_fee_paid IN (0,1)),
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_users_membership_status ON users(membership_status);
CREATE INDEX IF NOT EXISTS idx_users_membership_expiry ON users(membership_expiry);
CREATE INDEX IF NOT EXISTS idx_users_annual_fee_expiry ON users(annual_fee_expiry);

CREATE TABLE IF NOT EXISTS sub_members (
    id               TEXT PRIMARY KEY,
    member_id        TEXT NOT NULL UNIQUE,
    parent_user_id   TEXT NOT NULL,
    name             TEXT NOT NULL,
    email            TEXT NOT NULL UNIQUE,
    phone            TEXT NOT NULL,
    password         TEXT NOT NULL,
    is_temp_password INTEGER NOT NULL DEFAULT 1 CHECK(is_temp_password IN (0,1)),
    relation         TEXT NOT NULL,
    can_login        INTEGER NOT NULL DEFAULT 1 CHECK(can_login IN (0,1)),
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (parent_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sub_members_parent_user_id ON sub_members(parent_user_id);

CREATE TABLE IF NOT EXISTS members (
    id               TEXT PRIMARY KEY,
    user_id          TEXT UNIQUE,
    name             TEXT NOT NULL,
    phone            TEXT NOT NULL,
    email            TEXT NOT NULL,
    address          TEXT NOT NULL,
    parent_member_id TEXT,
    joined_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (parent_member_id) REFERENCES members(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_members_user_id ON members(user_id);
CREATE INDEX IF NOT EXISTS idx_members_parent_member_id ON members(parent_member_id);

CREATE TABLE IF NOT EXISTS memberships (
    id                 TEXT PRIMARY KEY,
    member_id          TEXT NOT NULL,
    type               TEXT NOT NULL CHECK(type IN ('MONTHLY','HALF_YEARLY','ANNUAL')),
    fee_type           TEXT NOT NULL DEFAULT 'SUBSCRIPTION' CHECK(fee_type IN ('ANNUAL_FEE','SUBSCRIPTION')),
    amount             NUMERIC NOT NULL,
    start_date         TEXT NOT NULL,
    end_date           TEXT NOT NULL,
    is_application_fee INTEGER NOT NULL DEFAULT 0 CHECK(is_application_fee IN (0,1)),
    status             TEXT NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING','APPROVED','REJECTED')),
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (member_id) REFERENCES members(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_memberships_member_id ON memberships(member_id);
CREATE INDEX IF NOT EXISTS idx_memberships_status ON memberships(status);
CREATE INDEX IF NOT EXISTS idx_memberships_end_date ON memberships(end_date);
CREATE INDEX IF NOT EXISTS idx_memberships_fee_type ON memberships(fee_type);

CREATE TABLE IF NOT EXISTS sponsors (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    phone         TEXT NOT NULL,
    email         TEXT NOT NULL,
    company       TEXT,
    created_by_id TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (created_by_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_sponsors_created_by_id ON sponsors(created_by_id);

CREATE TABLE IF NOT EXISTS transactions (
    id                       TEXT PRIMARY KEY,
    type                     TEXT NOT NULL CHECK(type IN ('CASH_IN','CASH_OUT')),
    category                 TEXT NOT NULL CHECK(category IN ('MEMBERSHIP','SPONSORSHIP','EXPENSE','OTHER')),
    amount                   NUMERIC NOT NULL,
    payment_mode             TEXT NOT NULL CHECK(payment_mode IN ('UPI','BANK_TRANSFER','CASH')),
    purpose                  TEXT NOT NULL,
    remark                   TEXT,
    sponsor_purpose          TEXT CHECK(sponsor_purpose IN ('TITLE_SPONSOR','GOLD_SPONSOR','SILVER_SPONSOR','FOOD_PARTNER','MEDIA_PARTNER','STALL_VENDOR','MARKETING_PARTNER','OTHER')),
    member_id                TEXT,
    sponsor_id               TEXT,
    entered_by_id            TEXT NOT NULL,
    approval_status          TEXT NOT NULL DEFAULT 'PENDING' CHECK(approval_status IN ('PENDING','APPROVED','REJECTED')),
    approval_source          TEXT NOT NULL DEFAULT 'MANUAL' CHECK(approval_source IN ('MANUAL','RAZORPAY_WEBHOOK')),
    approved_by_id           TEXT,
    approved_at              TEXT,
    razorpay_payment_id      TEXT,
    razorpay_order_id        TEXT,
    sender_name              TEXT,
    sender_phone             TEXT,
    sender_upi_id            TEXT,
    sender_bank_account      TEXT,
    sender_bank_name         TEXT,
    sponsor_sender_name      TEXT,
    sponsor_sender_contact   TEXT,
    receipt_number           TEXT,
    includes_subscription    INTEGER NOT NULL DEFAULT 0 CHECK(includes_subscription IN (0,1)),
    includes_annual_fee      INTEGER NOT NULL DEFAULT 0 CHECK(includes_annual_fee IN (0,1)),
    includes_application_fee INTEGER NOT NULL DEFAULT 0 CHECK(includes_application_fee IN (0,1)),
    created_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (member_id) REFERENCES members(id) ON DELETE SET NULL,
    FOREIGN KEY (sponsor_id) REFERENCES sponsors(id) ON DELETE SET NULL,
    FOREIGN KEY (entered_by_id) REFERENCES users(id),
    FOREIGN KEY (approved_by_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(type);
CREATE INDEX IF NOT EXISTS idx_transactions_category ON transactions(category);
CREATE INDEX IF NOT EXISTS idx_transactions_member_id ON transactions(member_id);
CREATE INDEX IF NOT EXISTS idx_transactions_sponsor_id ON transactions(sponsor_id);
CREATE INDEX IF NOT EXISTS idx_transactions_approval_status ON transactions(approval_status);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_transactions_razorpay_payment_id ON transactions(razorpay_payment_id);
CREATE INDEX IF NOT EXISTS idx_transactions_razorpay_order_id ON transactions(razorpay_order_id);

CREATE TABLE IF NOT EXISTS receipts (
    id               TEXT PRIMARY KEY,
    transaction_id   TEXT NOT NULL UNIQUE,
    receipt_number   TEXT NOT NULL UNIQUE,
    issued_by_id     TEXT NOT NULL,
    issued_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    status           TEXT NOT NULL DEFAULT 'ACTIVE' CHECK(status IN ('ACTIVE','CANCELLED')),
    type             TEXT NOT NULL CHECK(type IN ('MEMBER','SPONSOR')),
    member_name      TEXT,
    member_code      TEXT,
    membership_start TEXT,
    membership_end   TEXT,
    sponsor_name     TEXT,
    sponsor_company  TEXT,
    sponsor_purpose  TEXT,
    amount           NUMERIC NOT NULL,
    payment_mode     TEXT NOT NULL,
    category         TEXT NOT NULL,
    purpose          TEXT NOT NULL,
    breakdown        TEXT, -- NOTE: JSON stored as TEXT; parsed by application layer
    remark           TEXT,
    received_by      TEXT NOT NULL,
    club_name        TEXT NOT NULL,
    club_address     TEXT NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE,
    FOREIGN KEY (issued_by_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_receipts_status ON receipts(status);
CREATE INDEX IF NOT EXISTS idx_receipts_issued_at ON receipts(issued_at);
CREATE INDEX IF NOT EXISTS idx_receipts_issued_by_id ON receipts(issued_by_id);

CREATE TABLE IF NOT EXISTS sponsor_links (
    id            TEXT PRIMARY KEY,
    sponsor_id    TEXT,
    token         TEXT NOT NULL UNIQUE,
    amount        NUMERIC,
    upi_id        TEXT NOT NULL,
    bank_details  TEXT, -- NOTE: JSON stored as TEXT; parsed by application layer
    is_active     INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    created_by_id TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    expires_at    TEXT,

    FOREIGN KEY (sponsor_id) REFERENCES sponsors(id) ON DELETE SET NULL,
    FOREIGN KEY (created_by_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_sponsor_links_token ON sponsor_links(token);
CREATE INDEX IF NOT EXISTS idx_sponsor_links_sponsor_id ON sponsor_links(sponsor_id);
CREATE INDEX IF NOT EXISTS idx_sponsor_links_is_active ON sponsor_links(is_active);

CREATE TABLE IF NOT EXISTS approvals (
    id              TEXT PRIMARY KEY,
    entity_type     TEXT NOT NULL CHECK(entity_type IN ('TRANSACTION','MEMBER_ADD','MEMBER_EDIT','MEMBER_DELETE','MEMBERSHIP')),
    entity_id       TEXT NOT NULL, -- NOTE: UUID stored as TEXT in SQLite
    action          TEXT NOT NULL,
    previous_data   TEXT, -- NOTE: JSON stored as TEXT; parsed by application layer
    new_data        TEXT, -- NOTE: JSON stored as TEXT; parsed by application layer
    requested_by_id TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING','APPROVED','REJECTED')),
    reviewed_by_id  TEXT,
    reviewed_at     TEXT,
    notes           TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (requested_by_id) REFERENCES users(id),
    FOREIGN KEY (reviewed_by_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_approvals_status ON approvals(status);
CREATE INDEX IF NOT EXISTS idx_approvals_entity ON approvals(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_approvals_requested_by_id ON approvals(requested_by_id);
CREATE INDEX IF NOT EXISTS idx_approvals_created_at ON approvals(created_at);

CREATE TABLE IF NOT EXISTS audit_logs (
    id                   TEXT PRIMARY KEY,
    transaction_id       TEXT NOT NULL,
    event_type           TEXT NOT NULL CHECK(event_type IN ('TRANSACTION_CREATED','TRANSACTION_APPROVED','TRANSACTION_REJECTED')),
    transaction_snapshot TEXT NOT NULL, -- NOTE: JSON stored as TEXT; parsed by application layer
    performed_by_id      TEXT NOT NULL,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (transaction_id) REFERENCES transactions(id),
    FOREIGN KEY (performed_by_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_transaction_id ON audit_logs(transaction_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_performed_by_id ON audit_logs(performed_by_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);

CREATE TABLE IF NOT EXISTS activity_logs (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    action      TEXT NOT NULL,
    description TEXT NOT NULL,
    metadata    TEXT, -- NOTE: JSON stored as TEXT; parsed by application layer
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),

    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_activity_logs_user_id ON activity_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_activity_logs_created_at ON activity_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_activity_logs_action ON activity_logs(action);

PRAGMA foreign_keys = ON;
