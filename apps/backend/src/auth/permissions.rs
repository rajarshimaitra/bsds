use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Role enum — matches the roles used in the legacy TypeScript source:
//   ADMIN > OPERATOR > ORGANISER > MEMBER
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Admin,
    Operator,
    Organiser,
    Member,
}

impl Role {
    /// Parse a role string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Role> {
        match s.to_uppercase().as_str() {
            "ADMIN" => Some(Role::Admin),
            "OPERATOR" => Some(Role::Operator),
            "ORGANISER" => Some(Role::Organiser),
            "MEMBER" => Some(Role::Member),
            _ => None,
        }
    }

    /// Return the role as its canonical SCREAMING_SNAKE_CASE string.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "ADMIN",
            Role::Operator => "OPERATOR",
            Role::Organiser => "ORGANISER",
            Role::Member => "MEMBER",
        }
    }

    /// Hierarchy level — higher values have more privileges.
    fn level(self) -> u8 {
        match self {
            Role::Admin => 4,
            Role::Operator => 3,
            Role::Organiser => 2,
            Role::Member => 1,
        }
    }

    /// Returns true if `self` has equal or greater privilege than `other`.
    pub fn has_at_least(self, other: Role) -> bool {
        self.level() >= other.level()
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Route → Role mapping (ported from TS ROUTE_PERMISSIONS)
// ---------------------------------------------------------------------------

/// Build the route-to-roles permission map. The keys are path prefixes; more
/// specific prefixes should win when multiple match (see `can_access_route`).
pub fn route_permissions() -> HashMap<&'static str, Vec<Role>> {
    let mut m = HashMap::new();

    // Admin-only
    m.insert("/dashboard/approvals", vec![Role::Admin]);
    m.insert("/api/approvals", vec![Role::Admin]);
    m.insert("/api/cron", vec![Role::Admin]);
    m.insert("/api/notifications", vec![Role::Admin]);

    // Admin + Operator
    m.insert("/dashboard/sponsorship", vec![Role::Admin, Role::Operator]);
    m.insert("/api/sponsors", vec![Role::Admin, Role::Operator]);
    m.insert("/api/sponsor-links", vec![Role::Admin, Role::Operator]);
    m.insert("/api/receipts", vec![Role::Admin, Role::Operator]);

    // Admin + Operator + Organiser
    let aoo = vec![Role::Admin, Role::Operator, Role::Organiser];
    m.insert("/dashboard/members", aoo.clone());
    m.insert("/dashboard/cash", aoo.clone());
    m.insert("/dashboard/audit-log", aoo.clone());
    m.insert("/dashboard/activity-log", aoo.clone());
    m.insert("/api/members", aoo.clone());
    m.insert("/api/transactions", aoo.clone());
    m.insert("/api/audit-log", aoo.clone());
    m.insert("/api/activity-log", aoo);

    // All authenticated
    let all = vec![Role::Admin, Role::Operator, Role::Organiser, Role::Member];
    m.insert("/dashboard/my-membership", all.clone());
    m.insert("/dashboard", all.clone());
    m.insert("/api/my-membership", all.clone());
    m.insert("/api/memberships", all.clone());
    m.insert("/api/payments/create-order", all.clone());
    m.insert("/api/payments/verify", all.clone());
    m.insert("/api/dashboard/stats", all.clone());
    m.insert("/api/auth/change-password", all);

    m
}

// ---------------------------------------------------------------------------
// Permission check helpers
// ---------------------------------------------------------------------------

/// Returns true if `role` is allowed to access `route` according to
/// `ROUTE_PERMISSIONS`. If no rule matches, authenticated users are allowed
/// (default-permissive for unlisted routes, matching the TS behaviour).
pub fn can_access_route(role: Role, route: &str) -> bool {
    let perms = route_permissions();

    let best_match = perms
        .keys()
        .filter(|prefix| route.starts_with(**prefix))
        .max_by_key(|prefix| prefix.len());

    match best_match {
        Some(prefix) => perms[prefix].contains(&role),
        None => true, // no rule — allow authenticated users
    }
}

/// Returns true if `role` is one of the `allowed` roles.
pub fn has_role(role: Role, allowed: &[Role]) -> bool {
    allowed.contains(&role)
}

/// Returns true if the user's role is ADMIN.
pub fn is_admin(role: Role) -> bool {
    role == Role::Admin
}

/// Returns true if the user's role is OPERATOR.
pub fn is_operator(role: Role) -> bool {
    role == Role::Operator
}

/// Returns true if the user's role is MEMBER.
pub fn is_member(role: Role) -> bool {
    role == Role::Member
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_from_str_roundtrip() {
        for role in [Role::Admin, Role::Operator, Role::Organiser, Role::Member] {
            assert_eq!(Role::from_str(role.as_str()), Some(role));
        }
        assert_eq!(Role::from_str("unknown"), None);
    }

    #[test]
    fn role_hierarchy() {
        assert!(Role::Admin.has_at_least(Role::Operator));
        assert!(Role::Operator.has_at_least(Role::Organiser));
        assert!(Role::Organiser.has_at_least(Role::Member));
        assert!(!Role::Member.has_at_least(Role::Admin));
    }

    #[test]
    fn admin_only_routes() {
        assert!(can_access_route(Role::Admin, "/dashboard/approvals"));
        assert!(!can_access_route(Role::Operator, "/dashboard/approvals"));
        assert!(!can_access_route(Role::Member, "/api/approvals/123"));
    }

    #[test]
    fn all_auth_routes() {
        assert!(can_access_route(Role::Member, "/dashboard"));
        assert!(can_access_route(Role::Organiser, "/api/memberships"));
    }

    #[test]
    fn unlisted_route_allows_authenticated() {
        assert!(can_access_route(Role::Member, "/api/some-unlisted-thing"));
    }
}
