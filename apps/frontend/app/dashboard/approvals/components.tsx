"use client";

import { ENUM_LABELS, formatDate } from "./constants";

/** Single label+value row */
export function DetailRow({
  label,
  value,
  className,
}: {
  label: string;
  value: string;
  className?: string;
}) {
  return (
    <div className="grid grid-cols-1 gap-1 py-2 text-sm border-b border-muted/50 last:border-0 sm:grid-cols-[160px_1fr] sm:gap-2">
      <span className="text-muted-foreground shrink-0 text-xs font-medium sm:text-sm sm:font-normal">{label}</span>
      <span className={className ?? "font-medium break-words"}>{value}</span>
    </div>
  );
}

/** Old → new diff row (highlighted yellow if changed) */
export function DiffRow({
  label,
  oldText,
  newText,
  oldClass,
  newClass,
  changed,
}: {
  label: string;
  oldText: string;
  newText: string;
  oldClass?: string;
  newClass?: string;
  changed: boolean;
}) {
  return (
    <div
      className={`py-2 border-b border-muted/50 last:border-0 ${
        changed ? "rounded px-2 bg-amber-50" : ""
      }`}
    >
      <div className="text-xs font-medium text-muted-foreground mb-1">{label}</div>
      {changed ? (
        <div className="flex items-center gap-2 text-sm flex-wrap">
          <span className={`line-through text-rose-500 ${oldClass ?? ""}`}>{oldText || "—"}</span>
          <span className="text-muted-foreground text-xs">→</span>
          <span className={`font-medium text-emerald-700 ${newClass ?? ""}`}>{newText || "—"}</span>
        </div>
      ) : (
        <span className={`text-sm font-medium ${newClass ?? ""}`}>{newText || "—"}</span>
      )}
    </div>
  );
}

/** Full transaction record fetched live from the DB */
export function TransactionLiveSection({ tx }: { tx: Record<string, unknown> }) {
  const member = tx.member as { name: string; email: string; phone?: string } | null;
  const sponsor = tx.sponsor as { name: string; company: string | null } | null;
  const enteredBy = tx.enteredBy as { name: string } | null;
  const txType = tx.type as string;
  const txCategory = tx.category as string;
  const isMembership = txCategory === "MEMBERSHIP";
  const isSponsorship = txCategory === "SPONSORSHIP";
  const isExpense = txCategory === "EXPENSE" || txType === "CASH_OUT";
  const sponsorPurpose = typeof tx.sponsorPurpose === "string" ? tx.sponsorPurpose : null;
  const senderName = typeof tx.senderName === "string" ? tx.senderName : null;
  const receiverContact = typeof tx.senderPhone === "string" ? tx.senderPhone : null;
  const sponsorSenderName = typeof tx.sponsorSenderName === "string" ? tx.sponsorSenderName : null;
  const sponsorSenderContact = typeof tx.sponsorSenderContact === "string" ? tx.sponsorSenderContact : null;
  const receiptNumber = typeof tx.receiptNumber === "string" ? tx.receiptNumber : null;
  const amountNum = parseFloat(String(tx.amount));
  const amountStr = isNaN(amountNum)
    ? String(tx.amount)
    : `₹${amountNum.toLocaleString("en-IN", { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;
  const amountCls =
    txType === "CASH_IN"
      ? "text-emerald-700 font-semibold"
      : txType === "CASH_OUT"
      ? "text-rose-700 font-semibold"
      : "font-semibold";

  return (
    <div className="rounded-md border px-3">
      <DetailRow label="Type" value={ENUM_LABELS.type?.[txType] ?? txType} />
      <DetailRow label="Category" value={ENUM_LABELS.category?.[tx.category as string] ?? (tx.category as string)} />
      <DetailRow label="Amount" value={amountStr} className={amountCls} />
      <DetailRow label="Payment Mode" value={ENUM_LABELS.paymentMode?.[tx.paymentMode as string] ?? (tx.paymentMode as string)} />
      <DetailRow label="Purpose" value={(tx.purpose as string) || (tx.description as string) || "—"} />
      {sponsorPurpose && (
        <DetailRow
          label="Sponsor Purpose"
          value={ENUM_LABELS.sponsorPurpose?.[sponsorPurpose] ?? sponsorPurpose}
        />
      )}
      {member && <DetailRow label="Member" value={`${member.name} (${member.email})`} />}
      {sponsor && (
        <DetailRow
          label="Sponsor"
          value={sponsor.company ? `${sponsor.name} — ${sponsor.company}` : sponsor.name}
        />
      )}
      {typeof tx.remark === "string" && tx.remark && <DetailRow label="Remark" value={tx.remark} />}
      {/* Sent By — inferred from category */}
      {isMembership && member && (
        <>
          <DetailRow label="Sent By" value={member.name} />
          {member.phone && <DetailRow label="Sender's Contact" value={member.phone} />}
        </>
      )}
      {isSponsorship && sponsorSenderName && (
        <>
          <DetailRow label="Sent By" value={sponsorSenderName} />
          {sponsorSenderContact && <DetailRow label="Sender's Contact" value={sponsorSenderContact} />}
        </>
      )}
      {isExpense && !isSponsorship && sponsorSenderName && (
        <>
          <DetailRow label="Sent By" value={sponsorSenderName} />
          {sponsorSenderContact && <DetailRow label="Sender's Phone" value={sponsorSenderContact} />}
        </>
      )}
      {/* Received By — always shown */}
      <DetailRow label="Received By" value={senderName || "—"} />
      {receiverContact && <DetailRow label="Receiver's Contact" value={receiverContact} />}
      {receiptNumber && <DetailRow label="Receipt #" value={receiptNumber} />}
      {enteredBy && <DetailRow label="Entered By" value={enteredBy.name} />}
      <DetailRow
        label="Date"
        value={formatDate(tx.createdAt as string)}
      />
    </div>
  );
}

/** Full member record fetched live from the DB */
export function MemberLiveSection({ member }: { member: Record<string, unknown> }) {
  const subMembers = (member.subMembers ?? member.childMembers) as Array<Record<string, unknown>> | undefined;

  return (
    <div className="space-y-3">
      <div className="rounded-md border px-3">
        <DetailRow label="Full Name" value={(member.name as string) || "—"} />
        <DetailRow label="Email" value={(member.email as string) || "—"} />
        <DetailRow label="Phone" value={(member.phone as string) || "—"} />
        {typeof member.address === "string" && <DetailRow label="Address" value={member.address} />}
        <DetailRow
          label="Status"
          value={
            (member.displayMembershipStatus as string) ||
            ((member.user as Record<string, unknown> | undefined)?.membershipStatus as string) ||
            "PENDING_APPROVAL"
          }
        />
        <DetailRow
          label="Member Since"
          value={member.createdAt ? new Date(member.createdAt as string).toLocaleDateString("en-IN", { day: "2-digit", month: "2-digit", year: "numeric" }) : "—"}
        />
      </div>

      {subMembers && subMembers.length > 0 && (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Sub-Members ({subMembers.length})
          </p>
          <div className="space-y-2">
            {subMembers.map((sub, i) => (
              <div key={i} className="rounded-md border px-3 bg-muted/20">
                <DetailRow label="Name" value={(sub.name as string) || "—"} />
                {typeof sub.relation === "string" && <DetailRow label="Relation" value={sub.relation} />}
                {typeof sub.phone === "string" && <DetailRow label="Phone" value={sub.phone} />}
                {typeof sub.email === "string" && <DetailRow label="Email" value={sub.email} />}
                {typeof sub.memberId === "string" && (
                  <DetailRow label="Member ID" value={sub.memberId} className="font-mono text-xs font-medium" />
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/** Membership plan details */
export function MembershipPlanSection({ ms }: { ms: Record<string, unknown> }) {
  const amountNum = parseFloat(String(ms.amount));
  const amountStr = isNaN(amountNum)
    ? String(ms.amount)
    : `₹${amountNum.toLocaleString("en-IN", { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;

  function fmtDate(d: unknown) {
    if (!d) return "—";
    try {
      return new Date(d as string).toLocaleDateString("en-IN", { day: "2-digit", month: "2-digit", year: "numeric" });
    } catch { return "—"; }
  }

  return (
    <div className="rounded-md border px-3">
      <DetailRow
        label="Plan"
        value={ENUM_LABELS.type?.[ms.type as string] ?? (ms.type as string) ?? "—"}
      />
      <DetailRow label="Fee" value={amountStr} className="font-semibold" />
      <DetailRow label="Start Date" value={fmtDate(ms.startDate)} />
      <DetailRow label="End Date" value={fmtDate(ms.endDate)} />
      {ms.isApplicationFee === true && <DetailRow label="Note" value="Includes application fee" />}
    </div>
  );
}

/** Renders the appropriate live-entity view or a loading skeleton */
export function LiveEntityView({
  entityType,
  liveEntity,
  loading,
}: {
  entityType: string;
  liveEntity: Record<string, unknown> | null;
  loading: boolean;
}) {
  if (loading) {
    return (
      <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="h-4 bg-muted rounded w-3/4" />
        ))}
      </div>
    );
  }
  if (!liveEntity) return null;
  if (entityType === "TRANSACTION") return <TransactionLiveSection tx={liveEntity} />;
  if (entityType === "MEMBER_ADD" || entityType === "MEMBER_EDIT" || entityType === "MEMBER_DELETE") return <MemberLiveSection member={liveEntity} />;
  if (entityType === "MEMBERSHIP") return <MembershipPlanSection ms={liveEntity} />;
  return null;
}
