"use client";

import { AlertTriangle } from "lucide-react";
import { ReceiptView } from "@/components/receipts/ReceiptView";
import type { ReceiptData } from "@/lib/receipt-utils";
import {
  ENUM_LABELS,
  FIELD_LABELS,
  SKIP_KEYS,
  formatFieldValue,
  type ApprovalRecord,
} from "./constants";
import {
  DetailRow,
  DiffRow,
  LiveEntityView,
  MemberLiveSection,
} from "./components";

export function ApprovalDetail({
  approval,
  liveEntity,
  entityLoading,
  memberData,
  memberLoading,
  receiptData,
  receiptLoading,
  receiptError,
  parentMemberData,
  parentMemberLoading,
}: {
  approval: ApprovalRecord;
  liveEntity: Record<string, unknown> | null;
  entityLoading: boolean;
  memberData?: Record<string, unknown> | null;
  memberLoading?: boolean;
  receiptData?: ReceiptData | null;
  receiptLoading?: boolean;
  receiptError?: string | null;
  parentMemberData?: Record<string, unknown> | null;
  parentMemberLoading?: boolean;
}) {
  const prev = approval.previousData ?? {};
  const next = approval.newData ?? {};
  const action = approval.action;

  const isDelete = action.includes("delete") || action.includes("remove");
  const isEdit =
    !isDelete && action.includes("edit") && Object.keys(prev).length > 0;
  const displayData = isDelete ? prev : next;
  const txType = (displayData as Record<string, unknown>).type;

  const allKeys = Array.from(new Set([...Object.keys(prev), ...Object.keys(next)])).filter(
    (k) => !SKIP_KEYS.has(k)
  );

  // ---- DELETE: show live entity (what will be removed) ----
  if (isDelete) {
    const warningText =
      action.includes("delete_transaction")
        ? "This transaction will be permanently voided."
        : action === "remove_sub_member"
        ? "This sub-member will be removed."
        : "This will suspend the member account. Data is retained.";

    const isSubMemberRemove = action === "remove_sub_member";

    return (
      <div className="space-y-3">
        <div className="flex items-center gap-2 rounded-xl bg-rose-50 p-3 text-sm text-rose-700">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          <span>{warningText}</span>
        </div>

        {isSubMemberRemove && (
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
              Parent Member
            </p>
            {parentMemberLoading ? (
              <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
                {[1, 2, 3].map((i) => (
                  <div key={i} className="h-4 bg-muted rounded w-3/4" />
                ))}
              </div>
            ) : parentMemberData ? (
              <MemberLiveSection member={parentMemberData} />
            ) : (
              <div className="rounded-md border px-3 py-3 text-sm text-muted-foreground">
                Parent member details unavailable
              </div>
            )}
          </div>
        )}

        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {isSubMemberRemove ? "Sub-member Being Removed" : "Current Record"}
        </p>
        {(entityLoading || liveEntity) ? (
          <LiveEntityView
            entityType={approval.entityType}
            liveEntity={liveEntity}
            loading={entityLoading}
          />
        ) : (
          <div className="rounded-md border px-3">
            {Object.keys(displayData)
              .filter((k) => !SKIP_KEYS.has(k))
              .map((k) => {
                const { text, className } = formatFieldValue(k, (displayData as Record<string, unknown>)[k], txType);
                return <DetailRow key={k} label={FIELD_LABELS[k] ?? k} value={text} className={className} />;
              })}
          </div>
        )}
      </div>
    );
  }

  // ---- EDIT: diff + full current record ----
  if (isEdit) {
    const isSubMemberEdit = action === "edit_sub_member";
    const changedKeys = allKeys.filter(
      (k) => String(prev[k] ?? "") !== String(next[k] ?? "")
    );
    const unchangedKeys = allKeys.filter((k) => !changedKeys.includes(k));

    return (
      <div className="space-y-5">
        {isSubMemberEdit && (
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
              Parent Member
            </p>
            {parentMemberLoading ? (
              <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
                {[1, 2, 3].map((i) => (
                  <div key={i} className="h-4 bg-muted rounded w-3/4" />
                ))}
              </div>
            ) : parentMemberData ? (
              <MemberLiveSection member={parentMemberData} />
            ) : (
              <div className="rounded-md border px-3 py-3 text-sm text-muted-foreground">
                Parent member details unavailable
              </div>
            )}
          </div>
        )}

        {changedKeys.length > 0 && (
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
              Changed Fields ({changedKeys.length})
            </p>
            <div className="rounded-md border px-3">
              {changedKeys.map((k) => {
                const { text: oldText, className: oldCls } = formatFieldValue(k, prev[k], txType);
                const { text: newText, className: newCls } = formatFieldValue(k, next[k], txType);
                return (
                  <DiffRow
                    key={k}
                    label={FIELD_LABELS[k] ?? k}
                    oldText={oldText}
                    newText={newText}
                    oldClass={oldCls}
                    newClass={newCls}
                    changed={true}
                  />
                );
              })}
            </div>
          </div>
        )}

        {unchangedKeys.length > 0 && (
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
              Unchanged
            </p>
            <div className="rounded-md border px-3">
              {unchangedKeys.map((k) => {
                const { text, className } = formatFieldValue(k, next[k], txType);
                return (
                  <DetailRow key={k} label={FIELD_LABELS[k] ?? k} value={text} className={className} />
                );
              })}
            </div>
          </div>
        )}

        {(entityLoading || liveEntity) && (
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
              Full Current Record
            </p>
            <LiveEntityView
              entityType={approval.entityType}
              liveEntity={liveEntity}
              loading={entityLoading}
            />
          </div>
        )}
      </div>
    );
  }

  // ---- MEMBERSHIP create: show full member profile + plan details ----
  if (approval.entityType === "MEMBERSHIP") {
    return (
      <div className="space-y-5">
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Member Profile
          </p>
          {memberLoading ? (
            <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-4 bg-muted rounded w-3/4" />
              ))}
            </div>
          ) : memberData ? (
            <MemberLiveSection member={memberData} />
          ) : (liveEntity?.member as Record<string, string> | null) ? (
            <div className="rounded-md border px-3">
              <DetailRow
                label="Full Name"
                value={(liveEntity!.member as Record<string, string>).name ?? "—"}
              />
              <DetailRow
                label="Email"
                value={(liveEntity!.member as Record<string, string>).email ?? "—"}
              />
            </div>
          ) : (
            <div className="rounded-md border px-3 py-3 text-sm text-muted-foreground">
              Member details unavailable
            </div>
          )}
        </div>

        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Membership Plan
          </p>
          {(entityLoading || liveEntity) ? (
            <LiveEntityView
              entityType="MEMBERSHIP"
              liveEntity={liveEntity}
              loading={entityLoading}
            />
          ) : (
            <div className="rounded-md border px-3">
              {typeof next.type === "string" && (
                <DetailRow
                  label="Plan"
                  value={ENUM_LABELS.type?.[next.type] ?? next.type}
                />
              )}
              {next.amount != null && (
                <DetailRow
                  label="Fee"
                  value={`₹${parseFloat(String(next.amount)).toLocaleString("en-IN", { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`}
                  className="font-semibold"
                />
              )}
              {typeof next.startDate === "string" && (
                <DetailRow label="Start Date" value={formatFieldValue("startDate", next.startDate).text} />
              )}
              {typeof next.endDate === "string" && (
                <DetailRow label="End Date" value={formatFieldValue("endDate", next.endDate).text} />
              )}
              {next.isApplicationFee === true && (
                <DetailRow label="Note" value="Includes application fee" />
              )}
            </div>
          )}
        </div>
      </div>
    );
  }

  // ---- TRANSACTION: prefer live entity over newData snapshot ----
  if (approval.entityType === "TRANSACTION") {
    return (
      <div className="space-y-3">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Transaction Details
        </p>
        {(entityLoading || liveEntity) ? (
          <LiveEntityView
            entityType="TRANSACTION"
            liveEntity={liveEntity}
            loading={entityLoading}
          />
        ) : (
          <div className="rounded-md border px-3">
            {Object.keys(displayData)
              .filter((k) => !SKIP_KEYS.has(k))
              .map((k) => {
                const { text, className } = formatFieldValue(k, (displayData as Record<string, unknown>)[k], txType);
                return <DetailRow key={k} label={FIELD_LABELS[k] ?? k} value={text} className={className} />;
              })}
            </div>
        )}

        <div className="space-y-2">
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Issued Receipt
          </p>
          {receiptLoading ? (
            <div className="rounded-md border px-3 py-4 text-sm text-muted-foreground">
              Loading receipt...
            </div>
          ) : receiptError ? (
            <div className="rounded-md border border-rose-200 bg-rose-50 px-3 py-3 text-sm text-rose-700">
              {receiptError}
            </div>
          ) : receiptData ? (
            <ReceiptView receipt={receiptData} />
          ) : (
            <div className="rounded-md border px-3 py-4 text-sm text-muted-foreground">
              No receipt is stored for this transaction.
            </div>
          )}
        </div>
      </div>
    );
  }

  // ---- ADD (member / sub-member): show proposed details ----
  const isSubMemberAdd = action === "add_sub_member";

  return (
    <div className="space-y-5">
      {isSubMemberAdd && (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Parent Member
          </p>
          {parentMemberLoading ? (
            <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-4 bg-muted rounded w-3/4" />
              ))}
            </div>
          ) : parentMemberData ? (
            <MemberLiveSection member={parentMemberData} />
          ) : (
            <div className="rounded-md border px-3 py-3 text-sm text-muted-foreground">
              Parent member details unavailable
            </div>
          )}
        </div>
      )}

      <div>
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
          {isSubMemberAdd ? "Proposed Sub-Member Details" : "Proposed Member Details"}
        </p>
        {(entityLoading || liveEntity) ? (
          <LiveEntityView
            entityType={approval.entityType}
            liveEntity={liveEntity}
            loading={entityLoading}
          />
        ) : isSubMemberAdd ? (
          <div className="rounded-md border px-3">
            <DetailRow label="Full Name" value={(displayData.name as string) || "—"} />
            <DetailRow label="Email" value={(displayData.email as string) || "—"} />
            <DetailRow label="Phone" value={(displayData.phone as string) || "—"} />
            {typeof displayData.relation === "string" && (
              <DetailRow label="Relation to Member" value={displayData.relation} />
            )}
          </div>
        ) : (
          <MemberLiveSection member={displayData as Record<string, unknown>} />
        )}
      </div>
    </div>
  );
}
