"use client";

import { PrinterIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ReceiptData } from "@/lib/receipt-utils";
import { amountToWords } from "@/lib/receipt-utils";

interface ReceiptViewProps {
  receipt: ReceiptData;
  showPrintButton?: boolean;
}

function formatDate(date: Date | string): string {
  const d = typeof date === "string" ? new Date(date) : date;
  return d.toLocaleDateString("en-IN", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
}

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-IN", {
    style: "currency",
    currency: "INR",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(amount);
}

function paymentModeLabel(mode: string): string {
  return mode.replaceAll("_", " ");
}

function formatBreakdownLabel(label: string): string {
  return label;
}

function FieldRow({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="grid grid-cols-[minmax(0,11rem)_1fr] gap-3 border-b border-slate-200/80 py-3 last:border-b-0">
      <div className="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
        {label}
      </div>
      <div
        className={[
          "text-sm leading-6 text-slate-900 sm:text-[15px]",
          mono ? "font-mono text-[13px] sm:text-sm" : "",
        ].join(" ")}
      >
        {value}
      </div>
    </div>
  );
}

function MemberReceiptBody({ receipt }: { receipt: ReceiptData }) {
  const breakdown = receipt.breakdown ?? [];

  return (
    <>
      <FieldRow label="Received From" value={receipt.memberName ?? "—"} />
      {receipt.memberId ? (
        <FieldRow label="Member ID" value={receipt.memberId} mono />
      ) : null}
      <FieldRow label="Purpose" value={receipt.purpose ?? receipt.category} />
      {breakdown.length > 0 ? (
        <div className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3">
          <div className="mb-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
            Breakdown
          </div>
          <div className="space-y-2">
            {breakdown.map((item) => (
              <div key={`${item.label}-${item.amount}`} className="flex items-start justify-between gap-4 text-sm text-slate-800">
                <span className="pr-3 leading-6">{formatBreakdownLabel(item.label)}</span>
                <span className="font-medium tabular-nums">{formatCurrency(item.amount)}</span>
              </div>
            ))}
            <div className="flex items-start justify-between gap-4 border-t border-slate-200 pt-2 text-sm font-semibold text-slate-950">
              <span>Total</span>
              <span className="tabular-nums">{formatCurrency(breakdown.reduce((sum, item) => sum + item.amount, 0))}</span>
            </div>
          </div>
        </div>
      ) : null}
      {receipt.membershipStart && receipt.membershipEnd ? (
        <FieldRow
          label="Period"
          value={`${formatDate(receipt.membershipStart)} to ${formatDate(receipt.membershipEnd)}`}
        />
      ) : null}
      {receipt.remark ? <FieldRow label="Remark" value={receipt.remark} /> : null}
    </>
  );
}

function SponsorReceiptBody({ receipt }: { receipt: ReceiptData }) {
  return (
    <>
      <FieldRow label="Received From" value={receipt.sponsorName ?? "—"} />
      {receipt.sponsorCompany ? (
        <FieldRow label="Company" value={receipt.sponsorCompany} />
      ) : null}
      <FieldRow label="Sponsorship Type" value={receipt.sponsorPurpose ?? "—"} />
      <FieldRow
        label="Purpose"
        value={receipt.purpose ?? receipt.category}
      />
      {receipt.remark ? <FieldRow label="Remark" value={receipt.remark} /> : null}
    </>
  );
}

export function ReceiptView({
  receipt,
  showPrintButton = true,
}: ReceiptViewProps) {
  function handlePrint() {
    window.print();
  }

  return (
    <>
      <style>{`
        @page {
          size: A4 portrait;
          margin: 10mm;
        }

        @media print {
          .no-print { display: none !important; }
          body { background: white !important; }

          /* Hide every body child that doesn't contain the receipt */
          body > *:not(:has(#receipt-print-root)) {
            display: none !important;
          }

          /* Inside the portal wrapper, hide the overlay (keeps only the dialog) */
          body > *:has([role="dialog"] #receipt-print-root) > *:not([role="dialog"]) {
            display: none !important;
          }

          /* Static — fixed position repeats on every printed page */
          [role="dialog"]:has(#receipt-print-root) {
            position: static !important;
            transform: none !important;
            width: 100% !important;
            max-width: none !important;
            margin: 0 !important;
            padding: 0 !important;
            border: none !important;
            border-radius: 0 !important;
            box-shadow: none !important;
            background: white !important;
            backdrop-filter: none !important;
            overflow: visible !important;
          }

          /* Hide all dialog descendants that are not the receipt, not an ancestor of
             the receipt, and not inside the receipt — covers both the dedicated
             receipt dialog (cash management) and the embedded case (approval queue)
             where the receipt lives inside a larger detail dialog. */
          [role="dialog"]:has(#receipt-print-root) *:not(:has(#receipt-print-root)):not(#receipt-print-root):not(#receipt-print-root *) {
            display: none !important;
          }

          /* Remove scroll constraint from every ancestor of the receipt inside the dialog */
          [role="dialog"]:has(#receipt-print-root) *:has(#receipt-print-root) {
            max-height: none !important;
            overflow: visible !important;
            padding: 0 !important;
          }

          /* ── Receipt print layout ── */
          #receipt-print-root {
            width: 100% !important;
            max-width: none !important;
            margin: 0 !important;
            padding: 0 !important;
            background: white !important;
          }

          #receipt-screen-shell {
            max-width: 100% !important;
            margin: 0 !important;
            padding: 0 !important;
            border: none !important;
            border-radius: 0 !important;
            background: transparent !important;
            box-shadow: none !important;
            overflow: visible !important;
          }

          #receipt-paper {
            max-width: 100% !important;
            width: 100% !important;
            min-height: auto !important;
            margin: 0 !important;
            padding: 8mm 12mm !important;
            border: none !important;
            border-radius: 0 !important;
            box-shadow: none !important;
            break-inside: avoid !important;
            page-break-inside: avoid !important;
            font-size: 12px !important;
          }

          /* Compact header */
          #receipt-paper > header {
            padding-bottom: 0.75rem !important;
          }

          /* Force two-column grid and tighten gaps */
          #receipt-paper > div.grid {
            grid-template-columns: 1.45fr 0.95fr !important;
            gap: 0.75rem !important;
            padding-top: 0.75rem !important;
            padding-bottom: 0.75rem !important;
          }

          /* Compact inner sections */
          #receipt-paper section,
          #receipt-paper aside section {
            padding: 0.5rem 0.75rem !important;
            border-radius: 0.5rem !important;
          }

          /* Compact footer */
          #receipt-paper > footer {
            margin-top: 0.5rem !important;
            padding-top: 0.75rem !important;
            gap: 0.75rem !important;
          }
          #receipt-paper > footer > div {
            padding: 0.5rem 0.75rem !important;
            border-radius: 0.5rem !important;
          }
        }
      `}</style>

      <section id="receipt-print-root" className="space-y-4">
        <div className="no-print flex flex-wrap items-center justify-between gap-3">
          <div className="space-y-1">
            <div className="text-sm font-semibold text-slate-900">
              {receipt.receiptNumber}
            </div>
            <div className="text-xs text-slate-500">
              Sized for clear preview and clean PDF export
            </div>
          </div>
          <div className="flex items-center gap-3">
            {receipt.status === "CANCELLED" ? (
              <div className="rounded-full border border-rose-200 bg-rose-50 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-rose-700">
                Cancelled
              </div>
            ) : null}
            {showPrintButton ? (
              <Button onClick={handlePrint} size="sm" className="gap-2 rounded-full px-4">
                <PrinterIcon className="h-4 w-4" />
                Print
              </Button>
            ) : null}
          </div>
        </div>

        <div
          id="receipt-screen-shell"
          className="mx-auto w-full max-w-[860px] overflow-x-auto rounded-[1.5rem] border border-slate-200 bg-[linear-gradient(180deg,#f8fafc_0%,#eef2ff_100%)] p-3 shadow-[0_24px_80px_-48px_rgba(15,23,42,0.45)] sm:p-5"
        >
          <article
            id="receipt-paper"
            className="mx-auto flex min-h-[1120px] w-full max-w-[794px] flex-col border border-slate-300 bg-white p-6 text-slate-900 shadow-[0_28px_70px_-40px_rgba(15,23,42,0.38)] sm:p-10"
          >
            <header className="border-b border-slate-900 pb-6">
              <div className="flex flex-wrap items-start justify-between gap-6">
                <div className="max-w-[32rem] space-y-2">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.35em] text-orange-600">
                    Deshapriya Park
                  </p>
                  <h2 className="text-2xl font-semibold uppercase tracking-[0.08em] text-slate-950 sm:text-[32px]">
                    {receipt.clubName}
                  </h2>
                  <p className="max-w-[34rem] text-sm leading-6 text-slate-600 sm:text-[15px]">
                    {receipt.clubAddress}
                  </p>
                </div>

                <div className="w-full max-w-[19rem] rounded-2xl border border-slate-200 bg-slate-50 p-4 sm:p-5">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">
                    Payment Receipt
                  </p>
                  <p className="mt-2 font-mono text-sm font-semibold text-slate-950 sm:text-base">
                    {receipt.receiptNumber}
                  </p>
                  <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
                    <div>
                      <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
                        Date
                      </p>
                      <p className="mt-1 text-slate-900">{formatDate(receipt.date)}</p>
                    </div>
                    <div>
                      <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
                        Status
                      </p>
                      <p className="mt-1 text-slate-900">
                        {receipt.status === "CANCELLED" ? "Cancelled" : "Active"}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </header>

            <div className="grid gap-6 py-8 lg:grid-cols-[1.45fr_0.95fr]">
              <section className="rounded-[1.25rem] border border-slate-200 bg-white p-5 sm:p-6">
                <div className="mb-4">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">
                    Receipt Details
                  </p>
                </div>

                {receipt.type === "MEMBER" ? (
                  <MemberReceiptBody receipt={receipt} />
                ) : (
                  <SponsorReceiptBody receipt={receipt} />
                )}
              </section>

              <aside className="space-y-5">
                <section className="rounded-[1.25rem] border border-orange-200 bg-[linear-gradient(180deg,#fff7ed_0%,#ffffff_100%)] p-5 sm:p-6">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.24em] text-orange-700">
                    Amount Received
                  </p>
                  <p className="mt-3 text-3xl font-semibold tracking-tight text-slate-950 sm:text-[40px]">
                    {formatCurrency(receipt.amount)}
                  </p>
                  <p className="mt-4 text-sm leading-6 text-slate-600">
                    {amountToWords(receipt.amount)}
                  </p>
                </section>

                <section className="rounded-[1.25rem] border border-slate-200 bg-slate-50 p-5 sm:p-6">
                  <p className="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">
                    Payment Summary
                  </p>
                  <div className="mt-4 space-y-4">
                    <div>
                      <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
                        Payment Mode
                      </p>
                      <p className="mt-1 text-sm text-slate-900 sm:text-[15px]">
                        {paymentModeLabel(receipt.paymentMode)}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
                        Received By
                      </p>
                      <p className="mt-1 text-sm text-slate-900 sm:text-[15px]">
                        {receipt.receivedBy}
                      </p>
                    </div>
                    <div>
                      <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
                        Category
                      </p>
                      <p className="mt-1 text-sm text-slate-900 sm:text-[15px]">
                        {receipt.category}
                      </p>
                    </div>
                  </div>
                </section>
              </aside>
            </div>

            <footer className="mt-auto grid gap-6 border-t border-slate-200 pt-8 sm:grid-cols-[1.2fr_0.8fr]">
              <div className="rounded-[1.25rem] border border-slate-200 bg-slate-50 p-5">
                <p className="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">
                  Note
                </p>
                <p className="mt-3 text-sm leading-6 text-slate-600">
                  This is a computer-generated receipt and does not require a physical
                  signature.
                </p>
              </div>

              <div className="flex items-end justify-start sm:justify-end">
                <div className="w-full max-w-[240px] border-t border-slate-900 pt-4 text-center">
                  <p className="text-sm font-semibold text-slate-900">
                    Authorised Signatory
                  </p>
                  <p className="mt-1 text-sm text-slate-600">Treasurer</p>
                </div>
              </div>
            </footer>
          </article>
        </div>
      </section>
    </>
  );
}

export default ReceiptView;
