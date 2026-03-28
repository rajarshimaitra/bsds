export type UserRole = "ADMIN" | "OPERATOR" | "ORGANISER" | "MEMBER";

export interface AuthUser {
  id: string;
  username: string;
  role: UserRole;
  memberId?: string | null;
  mustChangePassword: boolean;
}

export interface AuthState {
  user: AuthUser | null;
  loading: boolean;
}
