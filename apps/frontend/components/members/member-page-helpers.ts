import { formatMembershipStatus } from "@/lib/utils";
import type { MembershipStatus } from "@/types";

export interface SubMemberData {
  id: string;
  memberId: string;
  name: string;
  email: string;
  phone: string;
  relation: string;
  canLogin: boolean;
  createdAt: string;
}

export interface MemberData {
  id: string;
  name: string;
  email: string;
  phone: string;
  address: string;
  displayMembershipStatus: MembershipStatus;
  joinedAt: string;
  createdAt: string;
  updatedAt: string;
  user: {
    memberId: string;
    membershipStatus: MembershipStatus;
    totalPaid: string;
    applicationFeePaid: boolean;
  } | null;
  subMembers?: SubMemberData[];
}

export interface PaginatedMembersResponse {
  data: MemberData[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
}

export function statusBadgeVariant(
  status: MembershipStatus
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "ACTIVE":
      return "default";
    case "PENDING_APPROVAL":
      return "secondary";
    case "PENDING_PAYMENT":
    case "EXPIRED":
      return "destructive";
    case "SUSPENDED":
      return "outline";
  }
}

export function statusLabel(status: MembershipStatus): string {
  return formatMembershipStatus(status);
}

export const emptyMemberForm = {
  name: "",
  email: "",
  phone: "",
  address: "",
};

export const emptySubMemberForm = {
  name: "",
  email: "",
  phone: "",
  relation: "",
};
