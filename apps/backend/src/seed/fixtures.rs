//! Seed fixtures — reproduces the exact data from dps-dashboard/prisma/seed.ts.
//!
//! Strategy: delete-then-insert (matches the TS seed). Safe for local dev only.
//! Run via: `cargo run --bin seed`

use sqlx::SqlitePool;
use uuid::Uuid;

fn uid() -> String {
    Uuid::new_v4().to_string()
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

pub async fn seed_users(pool: &SqlitePool) {
    // Wipe in dependency order
    sqlx::query("DELETE FROM audit_logs").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM activity_logs").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM approvals").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM receipts").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM transactions").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM memberships").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM sponsor_links").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM sponsors").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM sub_members").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM members").execute(pool).await.unwrap();
    sqlx::query("DELETE FROM users").execute(pool).await.unwrap();

    let admin_pw   = bcrypt::hash("Admin@123", 12).unwrap();
    let op_pw      = bcrypt::hash("Operator@123", 12).unwrap();
    let org_pw     = bcrypt::hash("Organiser@123", 12).unwrap();
    let member_pw  = bcrypt::hash("Member@123", 12).unwrap();

    // admin
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_start,annual_fee_expiry,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'ADMIN','ACTIVE','ANNUAL',
                 '2026-01-01','2026-12-31',18000,1,'2026-01-01','2026-12-31',1,?,?)"
    )
    .bind("user_admin_01").bind("BSDS-2026-0001-00").bind("Subhash Mukherjee")
    .bind("admin@bsds.club").bind("9830000001").bind("12 Deshapriya Park, Kolkata 700026")
    .bind(&admin_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();

    // operator
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_start,annual_fee_expiry,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'OPERATOR','ACTIVE','ANNUAL',
                 '2026-01-01','2026-12-31',18000,1,'2026-01-01','2026-12-31',1,?,?)"
    )
    .bind("user_operator_01").bind("BSDS-2026-0002-00").bind("Ramesh Chatterjee")
    .bind("operator@bsds.club").bind("9830000002").bind("45 Rashbehari Avenue, Kolkata 700026")
    .bind(&op_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();

    // organiser
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_start,annual_fee_expiry,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'ORGANISER','ACTIVE','ANNUAL',
                 '2026-01-01','2026-12-31',18000,1,'2026-01-01','2026-12-31',1,?,?)"
    )
    .bind("user_organiser_01").bind("BSDS-2026-0008-00").bind("Pratima Das")
    .bind("organiser@bsds.club").bind("9830000008").bind("18 Jodhpur Park, Kolkata 700068")
    .bind(&org_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();

    // member1
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_start,annual_fee_expiry,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'MEMBER','ACTIVE','ANNUAL',
                 '2026-01-01','2026-12-31',18000,1,'2026-01-15','2027-01-14',1,?,?)"
    )
    .bind("user_member1_01").bind("BSDS-2026-0003-00").bind("Arijit Banerjee")
    .bind("member1@bsds.club").bind("9830000003").bind("78 Lake Gardens, Kolkata 700045")
    .bind(&member_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();

    // member2
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_start,annual_fee_expiry,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'MEMBER','ACTIVE','HALF_YEARLY',
                 '2026-01-01','2026-06-30',16500,1,'2026-01-20','2027-01-19',1,?,?)"
    )
    .bind("user_member2_01").bind("BSDS-2026-0004-00").bind("Priya Sen")
    .bind("member2@bsds.club").bind("9830000004").bind("22 Tollygunge, Kolkata 700033")
    .bind(&member_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();

    // member3
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_start,annual_fee_expiry,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'MEMBER','EXPIRED','MONTHLY',
                 '2025-12-01','2025-12-31',15250,1,'2025-01-01','2025-12-31',0,?,?)"
    )
    .bind("user_member3_01").bind("BSDS-2026-0005-00").bind("Debashis Roy")
    .bind("member3@bsds.club").bind("9830000005").bind("5 Gariahat Road, Kolkata 700019")
    .bind(&member_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();

    // member4
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'MEMBER','PENDING_PAYMENT',NULL,NULL,NULL,0,0,0,?,?)"
    )
    .bind("user_member4_01").bind("BSDS-2026-0006-00").bind("Suchitra Ghosh")
    .bind("member4@bsds.club").bind("9830000006").bind("99 Ballygunge Place, Kolkata 700019")
    .bind(&member_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();

    // member5
    sqlx::query(
        "INSERT INTO users (id,member_id,name,email,phone,address,password,is_temp_password,role,
         membership_status,membership_type,membership_start,membership_expiry,total_paid,
         application_fee_paid,annual_fee_start,annual_fee_expiry,annual_fee_paid,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,0,'MEMBER','ACTIVE','ANNUAL',
                 '2026-01-01','2026-12-31',18000,1,'2026-01-10','2027-01-09',1,?,?)"
    )
    .bind("user_member5_01").bind("BSDS-2026-0007-00").bind("Kaushik Dey")
    .bind("member5@bsds.club").bind("9830000007").bind("31 Hindustan Park, Kolkata 700029")
    .bind(&member_pw).bind(now()).bind(now())
    .execute(pool).await.unwrap();
}

// ---------------------------------------------------------------------------
// Sub-members
// ---------------------------------------------------------------------------

pub async fn seed_sub_members(pool: &SqlitePool) {
    let sub_pw = bcrypt::hash("SubMember@123", 12).unwrap();

    let subs = vec![
        ("sub_m1_01","BSDS-2026-0003-01","user_member1_01","Mitali Banerjee","mitali.banerjee@bsds.club","9830100001","Spouse"),
        ("sub_m1_02","BSDS-2026-0003-02","user_member1_01","Rohan Banerjee","rohan.banerjee@bsds.club","9830100002","Son"),
        ("sub_m1_03","BSDS-2026-0003-03","user_member1_01","Riya Banerjee","riya.banerjee@bsds.club","9830100003","Daughter"),
        ("sub_m2_01","BSDS-2026-0004-01","user_member2_01","Sourav Sen","sourav.sen@bsds.club","9830100004","Husband"),
        ("sub_m2_02","BSDS-2026-0004-02","user_member2_01","Kamala Devi Sen","kamala.sen@bsds.club","9830100005","Mother"),
        ("sub_m5_01","BSDS-2026-0007-01","user_member5_01","Ananya Dey","ananya.dey@bsds.club","9830100006","Wife"),
    ];

    for (id, member_id, parent_id, name, email, phone, relation) in subs {
        sqlx::query(
            "INSERT INTO sub_members (id,member_id,parent_user_id,name,email,phone,password,
             is_temp_password,relation,can_login,created_at)
             VALUES (?,?,?,?,?,?,?,0,?,1,?)"
        )
        .bind(id).bind(member_id).bind(parent_id).bind(name).bind(email)
        .bind(phone).bind(&sub_pw).bind(relation).bind(now())
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Member records
// ---------------------------------------------------------------------------

pub async fn seed_member_records(pool: &SqlitePool) {
    let records = vec![
        ("mr_admin_01",    "user_admin_01",    "Subhash Mukherjee",  "9830000001", "admin@bsds.club",    "12 Deshapriya Park, Kolkata 700026",    "2026-01-01T00:00:00Z"),
        ("mr_operator_01", "user_operator_01", "Ramesh Chatterjee",  "9830000002", "operator@bsds.club", "45 Rashbehari Avenue, Kolkata 700026",   "2026-01-01T00:00:00Z"),
        ("mr_organiser_01","user_organiser_01","Pratima Das",         "9830000008", "organiser@bsds.club","18 Jodhpur Park, Kolkata 700068",        "2026-01-01T00:00:00Z"),
        ("mr_member1_01",  "user_member1_01",  "Arijit Banerjee",    "9830000003", "member1@bsds.club",  "78 Lake Gardens, Kolkata 700045",        "2026-01-15T00:00:00Z"),
        ("mr_member2_01",  "user_member2_01",  "Priya Sen",           "9830000004", "member2@bsds.club",  "22 Tollygunge, Kolkata 700033",          "2026-01-20T00:00:00Z"),
        ("mr_member3_01",  "user_member3_01",  "Debashis Roy",        "9830000005", "member3@bsds.club",  "5 Gariahat Road, Kolkata 700019",        "2025-12-01T00:00:00Z"),
        ("mr_member4_01",  "user_member4_01",  "Suchitra Ghosh",      "9830000006", "member4@bsds.club",  "99 Ballygunge Place, Kolkata 700019",    "2026-02-01T00:00:00Z"),
        ("mr_member5_01",  "user_member5_01",  "Kaushik Dey",         "9830000007", "member5@bsds.club",  "31 Hindustan Park, Kolkata 700029",      "2026-01-10T00:00:00Z"),
    ];

    for (id, user_id, name, phone, email, address, joined_at) in records {
        sqlx::query(
            "INSERT INTO members (id,user_id,name,phone,email,address,joined_at,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?)"
        )
        .bind(id).bind(user_id).bind(name).bind(phone).bind(email)
        .bind(address).bind(joined_at).bind(now()).bind(now())
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Memberships
// ---------------------------------------------------------------------------

pub async fn seed_memberships(pool: &SqlitePool) {
    // (id, member_id, type, fee_type, amount, start_date, end_date, is_application_fee, status)
    let memberships: Vec<(&str,&str,&str,&str,f64,&str,&str,i64,&str)> = vec![
        // Application fees
        ("ms_m1_app",  "mr_member1_01","ANNUAL",      "SUBSCRIPTION",10000.0,"2026-01-15","2026-01-15",1,"APPROVED"),
        ("ms_m2_app",  "mr_member2_01","HALF_YEARLY", "SUBSCRIPTION",10000.0,"2026-01-20","2026-01-20",1,"APPROVED"),
        ("ms_m3_app",  "mr_member3_01","MONTHLY",     "SUBSCRIPTION",10000.0,"2025-12-01","2025-12-01",1,"APPROVED"),
        ("ms_m5_app",  "mr_member5_01","ANNUAL",      "SUBSCRIPTION",10000.0,"2026-01-10","2026-01-10",1,"APPROVED"),
        // Subscriptions
        ("ms_m1_sub",  "mr_member1_01","ANNUAL",      "SUBSCRIPTION", 3000.0,"2026-01-15","2026-12-31",0,"APPROVED"),
        ("ms_m2_sub",  "mr_member2_01","HALF_YEARLY", "SUBSCRIPTION", 1500.0,"2026-01-20","2026-06-30",0,"APPROVED"),
        ("ms_m3_sub",  "mr_member3_01","MONTHLY",     "SUBSCRIPTION",  250.0,"2025-12-01","2025-12-31",0,"APPROVED"),
        ("ms_m4_pend", "mr_member4_01","ANNUAL",      "SUBSCRIPTION",10000.0,"2026-02-01","2026-02-01",1,"PENDING"),
        ("ms_m5_sub",  "mr_member5_01","ANNUAL",      "SUBSCRIPTION", 3000.0,"2026-01-10","2026-12-31",0,"APPROVED"),
        // Annual fees
        ("ms_admin_af","mr_admin_01",  "ANNUAL","ANNUAL_FEE",          5000.0,"2026-01-01","2026-12-31",0,"APPROVED"),
        ("ms_op_af",   "mr_operator_01","ANNUAL","ANNUAL_FEE",         5000.0,"2026-01-01","2026-12-31",0,"APPROVED"),
        ("ms_m1_af",   "mr_member1_01","ANNUAL","ANNUAL_FEE",          5000.0,"2026-01-15","2027-01-14",0,"APPROVED"),
        ("ms_m2_af",   "mr_member2_01","ANNUAL","ANNUAL_FEE",          5000.0,"2026-01-20","2027-01-19",0,"APPROVED"),
        ("ms_m3_af",   "mr_member3_01","ANNUAL","ANNUAL_FEE",          5000.0,"2025-01-01","2025-12-31",0,"APPROVED"),
        ("ms_m5_af",   "mr_member5_01","ANNUAL","ANNUAL_FEE",          5000.0,"2026-01-10","2027-01-09",0,"APPROVED"),
    ];

    for (id, member_id, mtype, fee_type, amount, start_date, end_date, is_app_fee, status) in memberships {
        sqlx::query(
            "INSERT INTO memberships (id,member_id,type,fee_type,amount,start_date,end_date,
             is_application_fee,status,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?)"
        )
        .bind(id).bind(member_id).bind(mtype).bind(fee_type).bind(amount)
        .bind(start_date).bind(end_date).bind(is_app_fee).bind(status)
        .bind(now()).bind(now())
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Sponsors
// ---------------------------------------------------------------------------

pub async fn seed_sponsors(pool: &SqlitePool) {
    let sponsors = vec![
        ("sponsor_01","Balaram Das & Sons","9830200001","contact@balarambdas.com","Balaram Das & Sons Pvt. Ltd.","user_admin_01"),
        ("sponsor_02","Kolkata Sweets","9830200002","info@kolkatasweets.com","Kolkata Sweets Co.","user_operator_01"),
        ("sponsor_03","ABP Media Group","9830200003","partnerships@abp.in","ABP Media Group Ltd.","user_admin_01"),
        ("sponsor_04","Priya Textiles","9830200004","priyatex@gmail.com","Priya Textiles","user_operator_01"),
    ];
    for (id, name, phone, email, company, created_by) in sponsors {
        sqlx::query(
            "INSERT INTO sponsors (id,name,phone,email,company,created_by_id,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?)"
        )
        .bind(id).bind(name).bind(phone).bind(email).bind(company).bind(created_by)
        .bind(now()).bind(now())
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Sponsor links
// ---------------------------------------------------------------------------

pub async fn seed_sponsor_links(pool: &SqlitePool) {
    let bank = r#"{"accountNumber":"1234567890","bankName":"Axis Bank","ifscCode":"UTIB0000123"}"#;

    sqlx::query(
        "INSERT INTO sponsor_links (id,sponsor_id,token,amount,upi_id,bank_details,is_active,
         created_by_id,expires_at,created_at,updated_at)
         VALUES (?,?,?,?,?,?,1,?,?,?,?)"
    )
    .bind("sl_01").bind("sponsor_01").bind("sl_balaram_title_2026_abc123def456")
    .bind(500000.0_f64).bind("bsds.club@axisbank").bind(bank)
    .bind("user_admin_01").bind("2026-12-31T23:59:59Z").bind(now()).bind(now())
    .execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO sponsor_links (id,sponsor_id,token,amount,upi_id,bank_details,is_active,
         created_by_id,expires_at,created_at,updated_at)
         VALUES (?,?,?,?,?,?,0,?,?,?,?)"
    )
    .bind("sl_02").bind("sponsor_02").bind("sl_kolkatasweets_food_2026_xyz789")
    .bind(100000.0_f64).bind("bsds.club@axisbank").bind(bank)
    .bind("user_admin_01").bind("2026-01-31T23:59:59Z").bind(now()).bind(now())
    .execute(pool).await.unwrap();

    sqlx::query(
        "INSERT INTO sponsor_links (id,sponsor_id,token,upi_id,bank_details,is_active,
         created_by_id,expires_at,created_at,updated_at)
         VALUES (?,?,?,?,?,1,?,?,?,?)"
    )
    .bind("sl_03").bind("sponsor_04").bind("sl_priyatex_stall_2026_openamt")
    .bind("bsds.club@axisbank").bind(bank)
    .bind("user_operator_01").bind("2026-10-31T23:59:59Z").bind(now()).bind(now())
    .execute(pool).await.unwrap();
}

// ---------------------------------------------------------------------------
// Transactions (24)
// ---------------------------------------------------------------------------

pub async fn seed_transactions(pool: &SqlitePool) {
    // (id, type, category, amount, payment_mode, purpose, member_id, sponsor_id,
    //  sponsor_purpose, entered_by_id, approval_status, approval_source,
    //  approved_by_id, approved_at, razorpay_payment_id, razorpay_order_id,
    //  sender_name, sender_phone, sender_upi_id, sender_bank_account, sender_bank_name,
    //  receipt_number, includes_application_fee, includes_subscription, created_at)
    struct Txn<'a> {
        id: &'a str, typ: &'a str, cat: &'a str, amount: f64,
        mode: &'a str, purpose: &'a str,
        member_id: Option<&'a str>, sponsor_id: Option<&'a str>,
        sponsor_purpose: Option<&'a str>,
        entered_by: &'a str, approval_status: &'a str, approval_source: &'a str,
        approved_by: Option<&'a str>, approved_at: Option<&'a str>,
        rp_payment: Option<&'a str>, rp_order: Option<&'a str>,
        sender_name: Option<&'a str>, sender_phone: Option<&'a str>,
        sender_upi: Option<&'a str>, sender_bank_acc: Option<&'a str>, sender_bank_name: Option<&'a str>,
        sponsor_sender_name: Option<&'a str>, sponsor_sender_contact: Option<&'a str>,
        receipt_number: Option<&'a str>,
        incl_app_fee: i64, incl_sub: i64,
        created_at: &'a str,
    }

    let txns = vec![
        Txn { id:"txn_01", typ:"CASH_IN", cat:"MEMBERSHIP", amount:10000.0, mode:"UPI",
              purpose:"Application fee — Arijit Banerjee",
              member_id:Some("mr_member1_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"RAZORPAY_WEBHOOK",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-15T10:30:00Z"),
              rp_payment:Some("pay_QP123456789001"), rp_order:Some("order_QP123456789001"),
              sender_name:Some("Arijit Banerjee"), sender_phone:Some("9830000003"),
              sender_upi:Some("arijit.banerjee@oksbi"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0001"), incl_app_fee:1, incl_sub:0,
              created_at:"2026-01-15T10:28:00Z" },
        Txn { id:"txn_02", typ:"CASH_IN", cat:"MEMBERSHIP", amount:3000.0, mode:"UPI",
              purpose:"Annual subscription — Arijit Banerjee",
              member_id:Some("mr_member1_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"RAZORPAY_WEBHOOK",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-15T10:35:00Z"),
              rp_payment:Some("pay_QP123456789002"), rp_order:Some("order_QP123456789002"),
              sender_name:Some("Arijit Banerjee"), sender_phone:Some("9830000003"),
              sender_upi:Some("arijit.banerjee@oksbi"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0002"), incl_app_fee:0, incl_sub:1,
              created_at:"2026-01-15T10:33:00Z" },
        Txn { id:"txn_03", typ:"CASH_IN", cat:"MEMBERSHIP", amount:10000.0, mode:"BANK_TRANSFER",
              purpose:"Application fee — Priya Sen (NEFT)",
              member_id:Some("mr_member2_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-20T14:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Priya Sen"), sender_phone:Some("9830000004"),
              sender_upi:None, sender_bank_acc:Some("XXXX1234"), sender_bank_name:Some("SBI"),
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0003"), incl_app_fee:1, incl_sub:0,
              created_at:"2026-01-20T13:45:00Z" },
        Txn { id:"txn_04", typ:"CASH_IN", cat:"MEMBERSHIP", amount:1500.0, mode:"BANK_TRANSFER",
              purpose:"Half-yearly subscription — Priya Sen",
              member_id:Some("mr_member2_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-20T14:10:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Priya Sen"), sender_phone:Some("9830000004"),
              sender_upi:None, sender_bank_acc:Some("XXXX1234"), sender_bank_name:Some("SBI"),
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0004"), incl_app_fee:0, incl_sub:1,
              created_at:"2026-01-20T14:05:00Z" },
        Txn { id:"txn_05", typ:"CASH_IN", cat:"SPONSORSHIP", amount:500000.0, mode:"UPI",
              purpose:"Title sponsorship — Balaram Das & Sons",
              member_id:None, sponsor_id:Some("sponsor_01"), sponsor_purpose:Some("TITLE_SPONSOR"),
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"RAZORPAY_WEBHOOK",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-02-01T09:00:00Z"),
              rp_payment:Some("pay_QP123456789005"), rp_order:Some("order_QP123456789005"),
              sender_name:Some("Balaram Das"), sender_phone:Some("9830200001"),
              sender_upi:Some("balarambdas@icici"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:Some("Balaram Das"), sponsor_sender_contact:Some("9830200001"),
              receipt_number:Some("BSDS-REC-2026-0005"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-02-01T08:50:00Z" },
        Txn { id:"txn_06", typ:"CASH_IN", cat:"SPONSORSHIP", amount:100000.0, mode:"CASH",
              purpose:"Food partnership sponsorship — Kolkata Sweets",
              member_id:None, sponsor_id:Some("sponsor_02"), sponsor_purpose:Some("FOOD_PARTNER"),
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-02-05T11:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Ratan Mondal"), sender_phone:None, sender_upi:None,
              sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:Some("Ratan Mondal"), sponsor_sender_contact:Some("9830200002"),
              receipt_number:Some("BSDS-REC-2026-0006"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-02-05T10:45:00Z" },
        Txn { id:"txn_07", typ:"CASH_IN", cat:"SPONSORSHIP", amount:200000.0, mode:"BANK_TRANSFER",
              purpose:"Media partnership — ABP Media Group",
              member_id:None, sponsor_id:Some("sponsor_03"), sponsor_purpose:Some("MEDIA_PARTNER"),
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-02-10T15:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("ABP Finance Dept"), sender_phone:Some("9830200003"),
              sender_upi:None, sender_bank_acc:Some("XXXX5678"), sender_bank_name:Some("HDFC"),
              sponsor_sender_name:Some("ABP Finance Dept"), sponsor_sender_contact:Some("finance@abpgroup.in"),
              receipt_number:Some("BSDS-REC-2026-0007"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-02-10T14:30:00Z" },
        Txn { id:"txn_08", typ:"CASH_IN", cat:"MEMBERSHIP", amount:10000.0, mode:"UPI",
              purpose:"Application fee — Debashis Roy",
              member_id:Some("mr_member3_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"RAZORPAY_WEBHOOK",
              approved_by:Some("user_admin_01"), approved_at:Some("2025-12-01T11:00:00Z"),
              rp_payment:Some("pay_QP123456789008"), rp_order:Some("order_QP123456789008"),
              sender_name:Some("Debashis Roy"), sender_phone:Some("9830000005"),
              sender_upi:Some("debashis.roy@paytm"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0008"), incl_app_fee:1, incl_sub:0,
              created_at:"2025-12-01T10:55:00Z" },
        Txn { id:"txn_09", typ:"CASH_IN", cat:"MEMBERSHIP", amount:250.0, mode:"CASH",
              purpose:"Monthly subscription — Debashis Roy",
              member_id:Some("mr_member3_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2025-12-01T11:15:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Debashis Roy"), sender_phone:None, sender_upi:None,
              sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0009"), incl_app_fee:0, incl_sub:1,
              created_at:"2025-12-01T11:10:00Z" },
        Txn { id:"txn_10", typ:"CASH_IN", cat:"MEMBERSHIP", amount:10000.0, mode:"UPI",
              purpose:"Application fee — Kaushik Dey",
              member_id:Some("mr_member5_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"RAZORPAY_WEBHOOK",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-10T09:00:00Z"),
              rp_payment:Some("pay_QP123456789010"), rp_order:Some("order_QP123456789010"),
              sender_name:Some("Kaushik Dey"), sender_phone:Some("9830000007"),
              sender_upi:Some("kaushik.dey@gpay"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0010"), incl_app_fee:1, incl_sub:0,
              created_at:"2026-01-10T08:55:00Z" },
        Txn { id:"txn_11", typ:"CASH_IN", cat:"MEMBERSHIP", amount:3000.0, mode:"UPI",
              purpose:"Annual subscription — Kaushik Dey",
              member_id:Some("mr_member5_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"RAZORPAY_WEBHOOK",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-10T09:10:00Z"),
              rp_payment:Some("pay_QP123456789011"), rp_order:Some("order_QP123456789011"),
              sender_name:Some("Kaushik Dey"), sender_phone:Some("9830000007"),
              sender_upi:Some("kaushik.dey@gpay"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0011"), incl_app_fee:0, incl_sub:1,
              created_at:"2026-01-10T09:05:00Z" },
        Txn { id:"txn_12", typ:"CASH_OUT", cat:"EXPENSE", amount:45000.0, mode:"BANK_TRANSFER",
              purpose:"Pandal decoration materials — Om Decorators",
              member_id:None, sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-03-01T10:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Om Decorators"), sender_phone:None, sender_upi:None,
              sender_bank_acc:Some("XXXX9012"), sender_bank_name:Some("Canara Bank"),
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0012"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-02-28T17:00:00Z" },
        Txn { id:"txn_13", typ:"CASH_OUT", cat:"EXPENSE", amount:25000.0, mode:"CASH",
              purpose:"Sound system rental — 5 days",
              member_id:None, sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-03-05T12:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Rhythm Sound Systems"), sender_phone:None, sender_upi:None,
              sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0013"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-05T11:00:00Z" },
        Txn { id:"txn_14", typ:"CASH_OUT", cat:"EXPENSE", amount:75000.0, mode:"UPI",
              purpose:"Durga idol advance payment — Shilpa Studio",
              member_id:None, sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"PENDING", approval_source:"MANUAL",
              approved_by:None, approved_at:None, rp_payment:None, rp_order:None,
              sender_name:Some("Shilpa Studio"), sender_phone:Some("9830300001"),
              sender_upi:Some("shilpastudio@phonepe"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:None, incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-10T14:00:00Z" },
        Txn { id:"txn_15", typ:"CASH_IN", cat:"SPONSORSHIP", amount:50000.0, mode:"UPI",
              purpose:"Stall vendor fee — Priya Textiles",
              member_id:None, sponsor_id:Some("sponsor_04"), sponsor_purpose:Some("STALL_VENDOR"),
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"RAZORPAY_WEBHOOK",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-03-08T16:00:00Z"),
              rp_payment:Some("pay_QP123456789015"), rp_order:Some("order_QP123456789015"),
              sender_name:Some("Priya Textiles"), sender_phone:Some("9830200004"),
              sender_upi:Some("priyatex@phonepe"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:Some("Priya Textiles Rep"), sponsor_sender_contact:Some("9830200004"),
              receipt_number:Some("BSDS-REC-2026-0014"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-08T15:55:00Z" },
        Txn { id:"txn_16", typ:"CASH_IN", cat:"OTHER", amount:5000.0, mode:"CASH",
              purpose:"Cultural programme ticket sales",
              member_id:None, sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-03-12T18:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Gate Collection"), sender_phone:None, sender_upi:None,
              sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0015"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-12T17:30:00Z" },
        Txn { id:"txn_17", typ:"CASH_OUT", cat:"OTHER", amount:2500.0, mode:"CASH",
              purpose:"Stationery and printing costs",
              member_id:None, sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-03-07T10:00:00Z"),
              rp_payment:None, rp_order:None, sender_name:None, sender_phone:None,
              sender_upi:None, sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0016"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-07T09:45:00Z" },
        Txn { id:"txn_18", typ:"CASH_IN", cat:"MEMBERSHIP", amount:10000.0, mode:"CASH",
              purpose:"Application fee — Subhash Mukherjee (admin)",
              member_id:Some("mr_admin_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-01T09:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Subhash Mukherjee"), sender_phone:None, sender_upi:None,
              sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0017"), incl_app_fee:1, incl_sub:0,
              created_at:"2026-01-01T08:45:00Z" },
        Txn { id:"txn_19", typ:"CASH_IN", cat:"MEMBERSHIP", amount:3000.0, mode:"CASH",
              purpose:"Annual subscription — Subhash Mukherjee",
              member_id:Some("mr_admin_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-01T09:15:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Subhash Mukherjee"), sender_phone:None, sender_upi:None,
              sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0018"), incl_app_fee:0, incl_sub:1,
              created_at:"2026-01-01T09:10:00Z" },
        Txn { id:"txn_20", typ:"CASH_IN", cat:"MEMBERSHIP", amount:10000.0, mode:"UPI",
              purpose:"Application fee — Ramesh Chatterjee",
              member_id:Some("mr_operator_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-01-02T10:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Ramesh Chatterjee"), sender_phone:None,
              sender_upi:Some("ramesh.chatt@gpay"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:Some("BSDS-REC-2026-0019"), incl_app_fee:1, incl_sub:0,
              created_at:"2026-01-02T09:50:00Z" },
        Txn { id:"txn_21", typ:"CASH_IN", cat:"MEMBERSHIP", amount:10000.0, mode:"UPI",
              purpose:"Application fee — Suchitra Ghosh (pending)",
              member_id:Some("mr_member4_01"), sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"PENDING", approval_source:"MANUAL",
              approved_by:None, approved_at:None, rp_payment:None, rp_order:None,
              sender_name:Some("Suchitra Ghosh"), sender_phone:Some("9830000006"),
              sender_upi:Some("suchitra.ghosh@paytm"), sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:None, incl_app_fee:1, incl_sub:0,
              created_at:"2026-02-01T12:00:00Z" },
        Txn { id:"txn_22", typ:"CASH_OUT", cat:"EXPENSE", amount:15000.0, mode:"CASH",
              purpose:"Entertainment expenses — rejected, insufficient documentation",
              member_id:None, sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"REJECTED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-03-11T14:00:00Z"),
              rp_payment:None, rp_order:None, sender_name:None, sender_phone:None,
              sender_upi:None, sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:None, incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-11T12:00:00Z" },
        Txn { id:"txn_23", typ:"CASH_IN", cat:"SPONSORSHIP", amount:150000.0, mode:"BANK_TRANSFER",
              purpose:"Gold sponsorship partial payment — Kalyan Jewellers",
              member_id:None, sponsor_id:None, sponsor_purpose:Some("GOLD_SPONSOR"),
              entered_by:"user_admin_01", approval_status:"APPROVED", approval_source:"MANUAL",
              approved_by:Some("user_admin_01"), approved_at:Some("2026-03-13T11:00:00Z"),
              rp_payment:None, rp_order:None,
              sender_name:Some("Kalyan Jewellers"), sender_phone:None, sender_upi:None,
              sender_bank_acc:Some("XXXX3456"), sender_bank_name:Some("ICICI Bank"),
              sponsor_sender_name:Some("Kalyan Jewellers"), sponsor_sender_contact:Some("accounts@kalyanjewellers.net"),
              receipt_number:Some("BSDS-REC-2026-0020"), incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-13T10:30:00Z" },
        Txn { id:"txn_24", typ:"CASH_OUT", cat:"EXPENSE", amount:8500.0, mode:"BANK_TRANSFER",
              purpose:"Electricity charges — generator hire advance",
              member_id:None, sponsor_id:None, sponsor_purpose:None,
              entered_by:"user_operator_01", approval_status:"PENDING", approval_source:"MANUAL",
              approved_by:None, approved_at:None, rp_payment:None, rp_order:None,
              sender_name:Some("Sundarban Power"), sender_phone:Some("9830300002"),
              sender_upi:None, sender_bank_acc:None, sender_bank_name:None,
              sponsor_sender_name:None, sponsor_sender_contact:None,
              receipt_number:None, incl_app_fee:0, incl_sub:0,
              created_at:"2026-03-14T09:00:00Z" },
    ];

    for t in txns {
        sqlx::query(
            "INSERT INTO transactions (id,type,category,amount,payment_mode,purpose,
             member_id,sponsor_id,sponsor_purpose,entered_by_id,approval_status,approval_source,
             approved_by_id,approved_at,razorpay_payment_id,razorpay_order_id,
             sender_name,sender_phone,sender_upi_id,sender_bank_account,sender_bank_name,
             sponsor_sender_name,sponsor_sender_contact,
             receipt_number,includes_application_fee,includes_subscription,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
        )
        .bind(t.id).bind(t.typ).bind(t.cat).bind(t.amount).bind(t.mode).bind(t.purpose)
        .bind(t.member_id).bind(t.sponsor_id).bind(t.sponsor_purpose)
        .bind(t.entered_by).bind(t.approval_status).bind(t.approval_source)
        .bind(t.approved_by).bind(t.approved_at)
        .bind(t.rp_payment).bind(t.rp_order)
        .bind(t.sender_name).bind(t.sender_phone).bind(t.sender_upi)
        .bind(t.sender_bank_acc).bind(t.sender_bank_name)
        .bind(t.sponsor_sender_name).bind(t.sponsor_sender_contact)
        .bind(t.receipt_number)
        .bind(t.incl_app_fee).bind(t.incl_sub)
        .bind(t.created_at).bind(t.created_at)
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Approvals (10)
// ---------------------------------------------------------------------------

pub async fn seed_approvals(pool: &SqlitePool) {
    let approvals: Vec<(&str,&str,&str,&str,&str,&str,Option<&str>,Option<&str>,Option<&str>,&str)> = vec![
        // (id, entity_type, entity_id, action, new_data_json, requested_by,
        //  reviewed_by, reviewed_at, notes, status, created_at) -> split below
        ("ap_01","TRANSACTION",  "txn_12",     "approve_transaction",r#"{"approvalStatus":"APPROVED"}"#,"user_operator_01",None,None,None,"PENDING"),
        ("ap_02","MEMBER_ADD",   "mr_member4_01","add_member",r#"{"name":"Suchitra Ghosh","email":"member4@bsds.club"}"#,"user_operator_01",None,None,None,"PENDING"),
        ("ap_03","MEMBERSHIP",   "mr_member4_01","approve_membership",r#"{"type":"ANNUAL","amount":"10000"}"#,"user_operator_01",None,None,None,"PENDING"),
        ("ap_04","MEMBER_ADD",   "mr_member1_01","add_member",r#"{"name":"Arijit Banerjee","email":"member1@bsds.club"}"#,"user_operator_01",Some("user_admin_01"),Some("2026-01-15T09:00:00Z"),Some("Member verified and approved."),"APPROVED"),
        ("ap_05","TRANSACTION",  "txn_05",     "approve_transaction",r#"{"approvalStatus":"APPROVED"}"#,"user_operator_01",Some("user_admin_01"),Some("2026-02-01T09:05:00Z"),Some("Confirmed via Razorpay webhook."),"APPROVED"),
        ("ap_06","MEMBER_ADD",   "mr_member2_01","add_member",r#"{"name":"Priya Sen","email":"member2@bsds.club"}"#,"user_operator_01",Some("user_admin_01"),Some("2026-01-20T13:00:00Z"),Some("Approved — documents verified."),"APPROVED"),
        ("ap_07","MEMBER_EDIT",  "mr_member1_01","edit_member",r#"{"address":"78A Lake Gardens, Flat 3B, Kolkata 700045"}"#,"user_operator_01",Some("user_admin_01"),Some("2026-02-15T11:00:00Z"),None,"APPROVED"),
        ("ap_08","MEMBERSHIP",   "mr_member5_01","approve_membership",r#"{"type":"ANNUAL","amount":"10000"}"#,"user_operator_01",Some("user_admin_01"),Some("2026-01-10T09:05:00Z"),Some("Razorpay payment confirmed."),"APPROVED"),
        ("ap_09","MEMBER_DELETE","mr_member3_01","delete_member",r#"{"reason":"Test rejection"}"#,"user_operator_01",Some("user_admin_01"),Some("2026-03-01T09:00:00Z"),Some("Rejected — member has active history."),"REJECTED"),
        ("ap_10","TRANSACTION",  "txn_14",     "approve_transaction",r#"{"approvalStatus":"REJECTED"}"#,"user_operator_01",Some("user_admin_01"),Some("2026-03-10T16:00:00Z"),Some("Insufficient documentation."),"REJECTED"),
        ("ap_11","MEMBER_ADD",   "mr_member5_01","add_sub_member",r#"{"name":"Priti Dey","email":"priti.dey@bsds.club","phone":"9830100007","relation":"Daughter","parentMemberId":"mr_member5_01","parentUserId":"user_member5_01"}"#,"user_operator_01",None,None,None,"PENDING"),
    ];

    let created_ats = [
        "2026-03-10T14:05:00Z","2026-02-01T11:00:00Z","2026-02-01T12:05:00Z",
        "2026-01-14T17:00:00Z","2026-02-01T08:58:00Z","2026-01-19T16:00:00Z",
        "2026-02-14T15:00:00Z","2026-01-10T09:00:00Z","2026-02-28T09:00:00Z",
        "2026-03-10T14:30:00Z","2026-03-20T10:00:00Z",
    ];

    for (i, (id, entity_type, entity_id, action, new_data, requested_by, reviewed_by, reviewed_at, notes, status)) in approvals.iter().enumerate() {
        sqlx::query(
            "INSERT INTO approvals (id,entity_type,entity_id,action,previous_data,new_data,
             requested_by_id,status,reviewed_by_id,reviewed_at,notes,created_at,updated_at)
             VALUES (?,?,?,?,NULL,?,?,?,?,?,?,?,?)"
        )
        .bind(id).bind(entity_type).bind(entity_id).bind(action).bind(new_data)
        .bind(requested_by).bind(status).bind(reviewed_by).bind(reviewed_at).bind(notes)
        .bind(created_ats[i]).bind(created_ats[i])
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Audit logs (20 entries, approved transactions only)
// ---------------------------------------------------------------------------

pub async fn seed_audit_logs(pool: &SqlitePool) {
    let approved_txns = [
        "txn_01","txn_02","txn_03","txn_04","txn_05","txn_06","txn_07",
        "txn_08","txn_09","txn_10","txn_11","txn_12","txn_13","txn_15",
        "txn_16","txn_17","txn_18","txn_19","txn_20","txn_23",
    ];
    let actions = [
        "TRANSACTION_CREATED","TRANSACTION_APPROVED","TRANSACTION_CREATED","TRANSACTION_APPROVED",
        "TRANSACTION_CREATED","TRANSACTION_APPROVED","TRANSACTION_CREATED","TRANSACTION_APPROVED",
        "TRANSACTION_CREATED","TRANSACTION_APPROVED","TRANSACTION_CREATED","TRANSACTION_APPROVED",
        "TRANSACTION_CREATED","TRANSACTION_APPROVED","TRANSACTION_CREATED","TRANSACTION_APPROVED",
        "TRANSACTION_CREATED","TRANSACTION_APPROVED","TRANSACTION_CREATED","TRANSACTION_APPROVED",
    ];
    for (i, txn_id) in approved_txns.iter().enumerate() {
        sqlx::query(
            "INSERT INTO audit_logs (id,transaction_id,event_type,transaction_snapshot,performed_by_id,created_at)
             VALUES (?,?,?,?,?,?)"
        )
        .bind(uid()).bind(txn_id).bind(actions[i])
        .bind(format!(r#"{{"transactionId":"{}","action":"{}"}}"#, txn_id, actions[i]))
        .bind("user_admin_01")
        .bind(now())
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Activity logs (20 entries)
// ---------------------------------------------------------------------------

pub async fn seed_activity_logs(pool: &SqlitePool) {
    let activities = [
        ("user_admin_01","login_success","User logged in",r#"{"ip":"127.0.0.1"}"#),
        ("user_operator_01","login_success","User logged in",r#"{"ip":"127.0.0.1"}"#),
        ("user_admin_01","member_created","Member Arijit Banerjee created",r#"{"memberId":"mr_member1_01"}"#),
        ("user_operator_01","member_created","Member Priya Sen created",r#"{"memberId":"mr_member2_01"}"#),
        ("user_operator_01","member_created","Member Debashis Roy created",r#"{"memberId":"mr_member3_01"}"#),
        ("user_operator_01","member_created","Member Suchitra Ghosh created",r#"{"memberId":"mr_member4_01"}"#),
        ("user_operator_01","member_created","Member Kaushik Dey created",r#"{"memberId":"mr_member5_01"}"#),
        ("user_admin_01","transaction_approved","Transaction txn_01 approved",r#"{"txnId":"txn_01"}"#),
        ("user_admin_01","transaction_approved","Transaction txn_05 approved",r#"{"txnId":"txn_05"}"#),
        ("user_admin_01","transaction_approved","Transaction txn_06 approved",r#"{"txnId":"txn_06"}"#),
        ("user_admin_01","transaction_approved","Transaction txn_07 approved",r#"{"txnId":"txn_07"}"#),
        ("user_admin_01","approval_approved","Approval ap_04 approved",r#"{"approvalId":"ap_04"}"#),
        ("user_admin_01","approval_approved","Approval ap_05 approved",r#"{"approvalId":"ap_05"}"#),
        ("user_admin_01","approval_approved","Approval ap_06 approved",r#"{"approvalId":"ap_06"}"#),
        ("user_admin_01","approval_approved","Approval ap_07 approved",r#"{"approvalId":"ap_07"}"#),
        ("user_admin_01","approval_approved","Approval ap_08 approved",r#"{"approvalId":"ap_08"}"#),
        ("user_admin_01","approval_rejected","Approval ap_09 rejected",r#"{"approvalId":"ap_09"}"#),
        ("user_admin_01","approval_rejected","Approval ap_10 rejected",r#"{"approvalId":"ap_10"}"#),
        ("user_operator_01","sponsor_created","Sponsor Balaram Das & Sons created",r#"{"sponsorId":"sponsor_01"}"#),
        ("user_admin_01","sponsor_link_created","Sponsor link sl_01 created",r#"{"linkId":"sl_01"}"#),
    ];

    for (user_id, action, description, metadata) in activities {
        sqlx::query(
            "INSERT INTO activity_logs (id,user_id,action,description,metadata,created_at)
             VALUES (?,?,?,?,?,?)"
        )
        .bind(uid()).bind(user_id).bind(action).bind(description).bind(metadata).bind(now())
        .execute(pool).await.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Receipts — one per eligible CASH_IN MEMBERSHIP/SPONSORSHIP transaction
// ---------------------------------------------------------------------------

const CLUB_NAME: &str = "Deshapriya Park Sarbojanin Durgotsav";
const CLUB_ADDR: &str = "Deshapriya Park, Bhawanipur, Kolkata - 700 026, West Bengal";

pub async fn seed_receipts(pool: &SqlitePool) {
    // Give txn_21 (PENDING operator transaction) a receipt number now that
    // receipts are generated at creation time rather than after approval.
    sqlx::query(
        "UPDATE transactions SET receipt_number = 'BSDS-REC-2026-0021' WHERE id = 'txn_21'"
    )
    .execute(pool).await.unwrap();

    struct Rec<'a> {
        id: &'a str,
        txn_id: &'a str,
        rn: &'a str,               // receipt_number
        issued_by: &'a str,
        issued_at: &'a str,
        rtype: &'a str,            // MEMBER | SPONSOR
        member_name: Option<&'a str>,
        member_code: Option<&'a str>,
        mstart: Option<&'a str>,
        mend: Option<&'a str>,
        sponsor_name: Option<&'a str>,
        sponsor_company: Option<&'a str>,
        sponsor_purpose: Option<&'a str>,
        amount: f64,
        mode: &'a str,             // already human-readable (UPI / Cash / Bank Transfer)
        category: &'a str,         // already human-readable (Membership / Sponsorship)
        purpose: &'a str,
        breakdown: &'a str,        // JSON string
        received_by: &'a str,
    }

    let receipts = vec![
        // ── MEMBERSHIP receipts ────────────────────────────────────────────
        Rec { id:"rec_s_01", txn_id:"txn_01", rn:"BSDS-REC-2026-0001",
              issued_by:"user_admin_01", issued_at:"2026-01-15T10:28:00Z",
              rtype:"MEMBER",
              member_name:Some("Arijit Banerjee"), member_code:Some("BSDS-2026-0003-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:10000.0, mode:"UPI", category:"Membership",
              purpose:"Application Fee",
              breakdown:r#"[{"label":"Application Fee","amount":10000}]"#,
              received_by:"Subhash Mukherjee" },

        Rec { id:"rec_s_02", txn_id:"txn_02", rn:"BSDS-REC-2026-0002",
              issued_by:"user_admin_01", issued_at:"2026-01-15T10:33:00Z",
              rtype:"MEMBER",
              member_name:Some("Arijit Banerjee"), member_code:Some("BSDS-2026-0003-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:3000.0, mode:"UPI", category:"Membership",
              purpose:"Annual Subscription",
              breakdown:r#"[{"label":"Annual Subscription","amount":3000}]"#,
              received_by:"Subhash Mukherjee" },

        Rec { id:"rec_s_03", txn_id:"txn_03", rn:"BSDS-REC-2026-0003",
              issued_by:"user_operator_01", issued_at:"2026-01-20T13:45:00Z",
              rtype:"MEMBER",
              member_name:Some("Priya Sen"), member_code:Some("BSDS-2026-0004-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-06-30"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:10000.0, mode:"Bank Transfer", category:"Membership",
              purpose:"Application Fee",
              breakdown:r#"[{"label":"Application Fee","amount":10000}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_04", txn_id:"txn_04", rn:"BSDS-REC-2026-0004",
              issued_by:"user_operator_01", issued_at:"2026-01-20T14:05:00Z",
              rtype:"MEMBER",
              member_name:Some("Priya Sen"), member_code:Some("BSDS-2026-0004-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-06-30"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:1500.0, mode:"Bank Transfer", category:"Membership",
              purpose:"Half-yearly Subscription",
              breakdown:r#"[{"label":"Half-yearly Subscription","amount":1500}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_08", txn_id:"txn_08", rn:"BSDS-REC-2026-0008",
              issued_by:"user_operator_01", issued_at:"2025-12-01T10:55:00Z",
              rtype:"MEMBER",
              member_name:Some("Debashis Roy"), member_code:Some("BSDS-2026-0005-00"),
              mstart:Some("2025-12-01"), mend:Some("2025-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:10000.0, mode:"UPI", category:"Membership",
              purpose:"Application Fee",
              breakdown:r#"[{"label":"Application Fee","amount":10000}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_09", txn_id:"txn_09", rn:"BSDS-REC-2026-0009",
              issued_by:"user_operator_01", issued_at:"2025-12-01T11:10:00Z",
              rtype:"MEMBER",
              member_name:Some("Debashis Roy"), member_code:Some("BSDS-2026-0005-00"),
              mstart:Some("2025-12-01"), mend:Some("2025-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:250.0, mode:"Cash", category:"Membership",
              purpose:"Monthly Subscription",
              breakdown:r#"[{"label":"Monthly Subscription","amount":250}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_10", txn_id:"txn_10", rn:"BSDS-REC-2026-0010",
              issued_by:"user_operator_01", issued_at:"2026-01-10T08:55:00Z",
              rtype:"MEMBER",
              member_name:Some("Kaushik Dey"), member_code:Some("BSDS-2026-0007-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:10000.0, mode:"UPI", category:"Membership",
              purpose:"Application Fee",
              breakdown:r#"[{"label":"Application Fee","amount":10000}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_11", txn_id:"txn_11", rn:"BSDS-REC-2026-0011",
              issued_by:"user_operator_01", issued_at:"2026-01-10T09:05:00Z",
              rtype:"MEMBER",
              member_name:Some("Kaushik Dey"), member_code:Some("BSDS-2026-0007-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:3000.0, mode:"UPI", category:"Membership",
              purpose:"Annual Subscription",
              breakdown:r#"[{"label":"Annual Subscription","amount":3000}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_18", txn_id:"txn_18", rn:"BSDS-REC-2026-0017",
              issued_by:"user_admin_01", issued_at:"2026-01-01T08:45:00Z",
              rtype:"MEMBER",
              member_name:Some("Subhash Mukherjee"), member_code:Some("BSDS-2026-0001-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:10000.0, mode:"Cash", category:"Membership",
              purpose:"Application Fee",
              breakdown:r#"[{"label":"Application Fee","amount":10000}]"#,
              received_by:"Subhash Mukherjee" },

        Rec { id:"rec_s_19", txn_id:"txn_19", rn:"BSDS-REC-2026-0018",
              issued_by:"user_admin_01", issued_at:"2026-01-01T09:10:00Z",
              rtype:"MEMBER",
              member_name:Some("Subhash Mukherjee"), member_code:Some("BSDS-2026-0001-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:3000.0, mode:"Cash", category:"Membership",
              purpose:"Annual Subscription",
              breakdown:r#"[{"label":"Annual Subscription","amount":3000}]"#,
              received_by:"Subhash Mukherjee" },

        Rec { id:"rec_s_20", txn_id:"txn_20", rn:"BSDS-REC-2026-0019",
              issued_by:"user_admin_01", issued_at:"2026-01-02T09:50:00Z",
              rtype:"MEMBER",
              member_name:Some("Ramesh Chatterjee"), member_code:Some("BSDS-2026-0002-00"),
              mstart:Some("2026-01-01"), mend:Some("2026-12-31"),
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:10000.0, mode:"UPI", category:"Membership",
              purpose:"Application Fee",
              breakdown:r#"[{"label":"Application Fee","amount":10000}]"#,
              received_by:"Subhash Mukherjee" },

        // Pending operator transaction — receipt issued at creation time
        Rec { id:"rec_s_21", txn_id:"txn_21", rn:"BSDS-REC-2026-0021",
              issued_by:"user_operator_01", issued_at:"2026-02-01T12:00:00Z",
              rtype:"MEMBER",
              member_name:Some("Suchitra Ghosh"), member_code:Some("BSDS-2026-0006-00"),
              mstart:None, mend:None,
              sponsor_name:None, sponsor_company:None, sponsor_purpose:None,
              amount:10000.0, mode:"UPI", category:"Membership",
              purpose:"Application Fee",
              breakdown:r#"[{"label":"Application Fee","amount":10000}]"#,
              received_by:"Ramesh Chatterjee" },

        // ── SPONSORSHIP receipts ───────────────────────────────────────────
        Rec { id:"rec_s_05", txn_id:"txn_05", rn:"BSDS-REC-2026-0005",
              issued_by:"user_admin_01", issued_at:"2026-02-01T08:50:00Z",
              rtype:"SPONSOR",
              member_name:None, member_code:None, mstart:None, mend:None,
              sponsor_name:Some("Balaram Das & Sons"),
              sponsor_company:Some("Balaram Das & Sons Pvt. Ltd."),
              sponsor_purpose:Some("Title Sponsor"),
              amount:500000.0, mode:"UPI", category:"Sponsorship",
              purpose:"Title Sponsor",
              breakdown:r#"[{"label":"Title Sponsor","amount":500000}]"#,
              received_by:"Subhash Mukherjee" },

        Rec { id:"rec_s_06", txn_id:"txn_06", rn:"BSDS-REC-2026-0006",
              issued_by:"user_operator_01", issued_at:"2026-02-05T10:45:00Z",
              rtype:"SPONSOR",
              member_name:None, member_code:None, mstart:None, mend:None,
              sponsor_name:Some("Kolkata Sweets"),
              sponsor_company:Some("Kolkata Sweets Co."),
              sponsor_purpose:Some("Food Partner"),
              amount:100000.0, mode:"Cash", category:"Sponsorship",
              purpose:"Food Partner",
              breakdown:r#"[{"label":"Food Partner","amount":100000}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_07", txn_id:"txn_07", rn:"BSDS-REC-2026-0007",
              issued_by:"user_admin_01", issued_at:"2026-02-10T14:30:00Z",
              rtype:"SPONSOR",
              member_name:None, member_code:None, mstart:None, mend:None,
              sponsor_name:Some("ABP Media Group"),
              sponsor_company:Some("ABP Media Group Ltd."),
              sponsor_purpose:Some("Media Partner"),
              amount:200000.0, mode:"Bank Transfer", category:"Sponsorship",
              purpose:"Media Partner",
              breakdown:r#"[{"label":"Media Partner","amount":200000}]"#,
              received_by:"Subhash Mukherjee" },

        Rec { id:"rec_s_15", txn_id:"txn_15", rn:"BSDS-REC-2026-0014",
              issued_by:"user_operator_01", issued_at:"2026-03-08T15:55:00Z",
              rtype:"SPONSOR",
              member_name:None, member_code:None, mstart:None, mend:None,
              sponsor_name:Some("Priya Textiles"),
              sponsor_company:Some("Priya Textiles"),
              sponsor_purpose:Some("Stall Vendor"),
              amount:50000.0, mode:"UPI", category:"Sponsorship",
              purpose:"Stall Vendor",
              breakdown:r#"[{"label":"Stall Vendor","amount":50000}]"#,
              received_by:"Ramesh Chatterjee" },

        Rec { id:"rec_s_23", txn_id:"txn_23", rn:"BSDS-REC-2026-0020",
              issued_by:"user_admin_01", issued_at:"2026-03-13T10:30:00Z",
              rtype:"SPONSOR",
              member_name:None, member_code:None, mstart:None, mend:None,
              sponsor_name:Some("Kalyan Jewellers"),
              sponsor_company:None,
              sponsor_purpose:Some("Gold Sponsor"),
              amount:150000.0, mode:"Bank Transfer", category:"Sponsorship",
              purpose:"Gold Sponsor",
              breakdown:r#"[{"label":"Gold Sponsor","amount":150000}]"#,
              received_by:"Subhash Mukherjee" },
    ];

    for r in receipts {
        sqlx::query(
            "INSERT INTO receipts (
                 id, transaction_id, receipt_number, issued_by_id, issued_at,
                 status, \"type\",
                 member_name, member_code, membership_start, membership_end,
                 sponsor_name, sponsor_company, sponsor_purpose,
                 amount, payment_mode, category, purpose, breakdown, remark,
                 received_by, club_name, club_address,
                 created_at, updated_at
             ) VALUES (
                 ?,?,?,?,?,
                 'ACTIVE',?,
                 ?,?,?,?,
                 ?,?,?,
                 ?,?,?,?,?,NULL,
                 ?,?,?,
                 ?,?
             )"
        )
        .bind(r.id).bind(r.txn_id).bind(r.rn)
        .bind(r.issued_by).bind(r.issued_at)
        .bind(r.rtype)
        .bind(r.member_name).bind(r.member_code).bind(r.mstart).bind(r.mend)
        .bind(r.sponsor_name).bind(r.sponsor_company).bind(r.sponsor_purpose)
        .bind(r.amount).bind(r.mode).bind(r.category)
        .bind(r.purpose).bind(r.breakdown)
        .bind(r.received_by).bind(CLUB_NAME).bind(CLUB_ADDR)
        .bind(r.issued_at).bind(r.issued_at)
        .execute(pool).await.unwrap();
    }
}
