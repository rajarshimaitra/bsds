"use client";

/**
 * /sponsor/[token]/receipt?paymentId=xxx — Public Sponsor Payment Receipt Page
 *
 * No authentication required.
 * Reads paymentId from URL query params.
 * Fetches receipt data from GET /api/sponsor-links/[token]/receipt?paymentId=xxx
 *
 * Displays:
 *   - "Thank You for Your Sponsorship!" header
 *   - Club name
 *   - Sponsor name / company
 *   - Amount paid (₹ formatted)
 *   - Payment date
 *   - Sponsorship purpose
 *   - Receipt number
 *   - Payment reference (Razorpay payment ID)
 *   - Print button
 *   - "Return to Homepage" link
 *
 * Note: The receipt page may briefly show a "processing" state if the Razorpay
 * webhook has not yet processed the payment (the webhook fires asynchronously).
 * We retry up to 5 times with a 2-second delay.
 */

import { useEffect, useState, useCallback } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
import { PrinterIcon, CheckCircleIcon, LoaderIcon, AlertCircleIcon, HomeIcon } from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ReceiptData {
  receiptNumber: string;
  sponsorName: string | null;
  sponsorCompany: string | null;
  amount: number;
  date: string;
  purpose: string;
  purposeLabel: string;
  paymentRef: string;
  clubName: string;
  clubAddress: string;
  paymentMode: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-IN", {
    style: "currency",
    currency: "INR",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(amount);
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString("en-IN", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
}

function formatDateShort(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString("en-IN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleTimeString("en-IN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: true,
  });
}

function paymentModeLabel(mode: string): string {
  const map: Record<string, string> = {
    UPI: "UPI",
    BANK_TRANSFER: "Bank Transfer (NEFT/IMPS/RTGS)",
    CASH: "Cash",
  };
  return map[mode] ?? mode;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function SponsorReceiptPage({
  params,
}: {
  params: { token: string };
}) {
  const searchParams = useSearchParams();
  const paymentId = searchParams.get("paymentId");
  const { token } = params;

  const [receipt, setReceipt] = useState<ReceiptData | null>(null);
  const [loading, setLoading] = useState(true);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);
  const [retrying, setRetrying] = useState(false);

  const MAX_RETRIES = 5;
  const RETRY_DELAY_MS = 3000;

  const fetchReceipt = useCallback(
    async (attempt: number) => {
      if (!paymentId) {
        setErrorMsg("No payment ID provided. Please return to the payment page.");
        setLoading(false);
        return;
      }

      try {
        const res = await fetch(
          `/api/sponsor-links/${token}/receipt?paymentId=${encodeURIComponent(paymentId)}`
        );

        if (res.status === 404) {
          // Payment may not have been processed by webhook yet — retry
          if (attempt < MAX_RETRIES) {
            setRetrying(true);
            setRetryCount(attempt + 1);
            setTimeout(() => fetchReceipt(attempt + 1), RETRY_DELAY_MS);
            return;
          }
          // Exhausted retries
          setErrorMsg(
            "Your payment was received but the receipt is still being generated. " +
              "Please check back in a few moments or contact the club with your payment reference."
          );
          setRetrying(false);
          setLoading(false);
          return;
        }

        if (!res.ok) {
          const data = await res.json().catch(() => ({}));
          setErrorMsg(
            data.error ?? "Unable to load receipt. Please contact the club with your payment reference."
          );
          setRetrying(false);
          setLoading(false);
          return;
        }

        const data: ReceiptData = await res.json();
        setReceipt(data);
        setRetrying(false);
        setLoading(false);
      } catch {
        if (attempt < MAX_RETRIES) {
          setTimeout(() => fetchReceipt(attempt + 1), RETRY_DELAY_MS);
          return;
        }
        setErrorMsg("Network error. Please check your connection and try again.");
        setRetrying(false);
        setLoading(false);
      }
    },
    [token, paymentId]
  );

  useEffect(() => {
    fetchReceipt(0);
  }, [fetchReceipt]);

  // =========================================================================
  // Render states
  // =========================================================================

  // Loading / retrying
  if (loading || retrying) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.2),_transparent_20rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)] p-4">
        <div className="w-full max-w-md rounded-[1.75rem] border border-white/80 bg-white/90 p-8 text-center shadow-[0_32px_80px_-40px_rgba(15,23,42,0.45)]">
          <LoaderIcon className="mx-auto mb-4 h-8 w-8 animate-spin text-sky-500" />
          <h2 className="mb-2 text-lg font-bold text-slate-900">
            {retryCount > 0 ? "Generating Receipt..." : "Loading..."}
          </h2>
          {retryCount > 0 && (
            <p className="text-sm text-slate-500">
              Your payment was received. The receipt is being generated
              {retryCount > 1 ? ` (attempt ${retryCount}/${MAX_RETRIES})` : ""}...
            </p>
          )}
        </div>
      </div>
    );
  }

  // Error state
  if (errorMsg || !receipt) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.2),_transparent_20rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)] p-4">
        <div className="w-full max-w-md rounded-[1.75rem] border border-white/80 bg-white/90 p-8 shadow-[0_32px_80px_-40px_rgba(15,23,42,0.45)]">
          {/* Payment was received even if receipt generation failed */}
          {paymentId && (
            <div className="mb-6 rounded-xl border border-emerald-200 bg-emerald-50 p-4">
              <div className="flex items-center gap-3">
                <CheckCircleIcon className="h-5 w-5 flex-shrink-0 text-emerald-600" />
                <div>
                  <p className="text-sm font-semibold text-emerald-800">Payment Received</p>
                  <p className="mt-1 font-mono text-xs text-emerald-600">{paymentId}</p>
                </div>
              </div>
            </div>
          )}

          <AlertCircleIcon className="h-10 w-10 text-amber-400 mx-auto mb-3" />
          <h1 className="mb-2 text-center text-lg font-bold text-slate-900">Receipt Unavailable</h1>
          <p className="mb-6 text-center text-sm text-slate-500">
            {errorMsg ?? "Unable to load receipt data."}
          </p>

          {paymentId && (
            <div className="mb-6 rounded-lg bg-slate-50 p-3">
              <p className="mb-1 text-xs text-slate-400">Your Payment Reference</p>
              <p className="break-all font-mono text-sm font-semibold text-slate-800">{paymentId}</p>
              <p className="mt-2 text-xs text-slate-400">
                Please save this reference number for your records.
              </p>
            </div>
          )}

          <Link
            href="/"
            className="flex w-full items-center justify-center gap-2 rounded-xl bg-slate-900 px-4 py-3 text-sm font-semibold text-white transition-colors hover:bg-sky-600"
          >
            <HomeIcon className="h-4 w-4" />
            Return to Homepage
          </Link>
        </div>
      </div>
    );
  }

  // =========================================================================
  // Success — show receipt
  // =========================================================================

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.22),_transparent_24rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)]">
      {/* Print-friendly CSS */}
      <style>{`
        @media print {
          body * { visibility: hidden; }
          #receipt-print-area,
          #receipt-print-area * { visibility: visible; }
          #receipt-print-area {
            position: fixed;
            top: 0;
            left: 0;
            width: 148mm;
            min-height: 210mm;
            margin: 0;
            padding: 12mm;
            box-sizing: border-box;
            background: white;
          }
          .no-print { display: none !important; }
          @page {
            size: A5 portrait;
            margin: 0;
          }
        }
      `}</style>

      {/* Header bar — hidden on print */}
      <div className="no-print bg-gradient-to-r from-orange-600 via-orange-500 to-amber-500 py-4 px-6 shadow-md">
        <div className="max-w-lg mx-auto flex items-center justify-between">
          <div>
            <p className="text-white/80 text-xs uppercase tracking-widest font-medium">
              Payment Confirmed
            </p>
            <h1 className="text-white text-lg font-bold leading-tight">
              {receipt.clubName}
            </h1>
          </div>
          <CheckCircleIcon className="h-8 w-8 text-white/90" />
        </div>
      </div>

      <div className="max-w-lg mx-auto p-4 pt-6">
        {/* Thank you card — hidden on print */}
        <div className="no-print mb-6 flex items-center gap-4 rounded-xl border border-emerald-200 bg-emerald-50 p-4">
          <CheckCircleIcon className="h-10 w-10 flex-shrink-0 text-emerald-500" />
          <div>
            <h2 className="text-base font-bold text-emerald-800">Thank You for Your Sponsorship!</h2>
            <p className="mt-0.5 text-sm text-emerald-600">
              Your contribution helps keep our heritage alive. We are grateful for your support.
            </p>
          </div>
        </div>

        {/* Action buttons — hidden on print */}
        <div className="no-print flex gap-3 mb-6">
          <button
            type="button"
            onClick={() => window.print()}
            className="flex-1 flex items-center justify-center gap-2 rounded-xl border border-slate-300 bg-white px-4 py-3 text-sm font-semibold text-slate-700 shadow-sm transition-colors hover:border-sky-400 hover:text-sky-600"
          >
            <PrinterIcon className="h-4 w-4" />
            Print Receipt
          </button>
          <Link
            href="/"
            className="flex-1 flex items-center justify-center gap-2 rounded-xl bg-slate-900 px-4 py-3 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-sky-600"
          >
            <HomeIcon className="h-4 w-4" />
            Homepage
          </Link>
        </div>

        {/* ============================================================
            RECEIPT PRINT AREA
            ============================================================ */}
        <div
          id="receipt-print-area"
          className="bg-white rounded-2xl shadow-lg overflow-hidden"
          style={{ fontFamily: "Georgia, serif" }}
        >
          {/* Receipt header */}
          <div className="bg-gradient-to-r from-orange-600 to-amber-500 p-6 text-white no-print">
            <div className="text-center">
              <div className="text-2xl mb-1">ॐ</div>
              <h2 className="text-xl font-bold uppercase tracking-wide">{receipt.clubName}</h2>
              <p className="text-white/75 text-xs mt-1">{receipt.clubAddress}</p>
            </div>
          </div>

          {/* Print-only header */}
          <div
            className="hidden"
            style={{
              display: "none",
              textAlign: "center",
              borderBottom: "2px solid #1a1a1a",
              paddingBottom: "8px",
              marginBottom: "10px",
            }}
          >
            <div style={{ fontSize: "15px", fontWeight: "bold", textTransform: "uppercase" }}>
              {receipt.clubName}
            </div>
            <div style={{ fontSize: "10px", color: "#444", marginTop: "3px" }}>
              {receipt.clubAddress}
            </div>
          </div>

          {/* Receipt body */}
          <div className="p-6">
            {/* Receipt title + number */}
            <div className="flex items-start justify-between mb-6">
              <div>
                <h3 className="text-lg font-bold uppercase tracking-wide text-slate-900 underline">
                  Sponsorship Receipt
                </h3>
                <p className="mt-1 text-sm text-slate-500">
                  {formatDate(receipt.date)} at {formatTime(receipt.date)}
                </p>
              </div>
              <div className="text-right">
                <p className="text-xs text-slate-400">Receipt No.</p>
                <p className="text-sm font-bold font-mono text-slate-800">
                  {receipt.receiptNumber}
                </p>
              </div>
            </div>

            <div className="border-t border-dashed border-gray-200 mb-5" />

            {/* Sponsor details */}
            <div className="space-y-3 mb-5">
              {receipt.sponsorName && (
                <div className="flex items-baseline gap-2">
                  <span className="w-32 flex-shrink-0 text-xs uppercase tracking-wide text-slate-400">
                    Received From
                  </span>
                  <span className="text-sm font-bold text-slate-900">{receipt.sponsorName}</span>
                </div>
              )}
              {receipt.sponsorCompany && (
                <div className="flex items-baseline gap-2">
                  <span className="w-32 flex-shrink-0 text-xs uppercase tracking-wide text-slate-400">
                    Company
                  </span>
                  <span className="text-sm text-slate-800">{receipt.sponsorCompany}</span>
                </div>
              )}
              <div className="flex items-baseline gap-2">
                <span className="w-32 flex-shrink-0 text-xs uppercase tracking-wide text-slate-400">
                  Sponsorship Type
                </span>
                <span className="text-sm font-semibold text-sky-700">
                  {receipt.purposeLabel}
                </span>
              </div>
            </div>

            <div className="border-t border-dashed border-gray-200 mb-5" />

            {/* Amount block */}
            <div className="mb-5 rounded-xl bg-sky-50 p-4">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-slate-600">Amount Paid</span>
                <span className="text-2xl font-bold text-slate-900">
                  {formatCurrency(receipt.amount)}
                </span>
              </div>
              <div className="flex items-center justify-between mt-2">
                <span className="text-xs text-slate-400">Payment Mode</span>
                <span className="text-xs font-semibold text-slate-700">
                  {paymentModeLabel(receipt.paymentMode)}
                </span>
              </div>
            </div>

            {/* Payment reference */}
            <div className="mb-5 rounded-xl bg-slate-50 p-4">
              <p className="mb-1 text-xs uppercase tracking-wide text-slate-400">Payment Reference</p>
              <p className="break-all font-mono text-sm font-semibold text-slate-800">
                {receipt.paymentRef}
              </p>
              <p className="mt-1 text-xs text-slate-400">Date: {formatDateShort(receipt.date)}</p>
            </div>

            <div className="border-t border-dashed border-gray-200 mb-5" />

            {/* Footer */}
            <div className="flex items-end justify-between">
              <div>
                <p className="max-w-48 text-xs leading-relaxed text-slate-400">
                  This is a computer-generated receipt and does not require a physical signature.
                </p>
              </div>
              <div className="text-center">
                <div className="border-t border-gray-400 pt-2 w-24 text-center">
                  <p className="text-xs text-slate-500">Authorised</p>
                  <p className="text-xs text-slate-500">Signatory</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Footer links — hidden on print */}
        <div className="no-print text-center mt-6 pb-8">
          <p className="mb-2 text-xs text-slate-400">
            A receipt has been generated for your records.
          </p>
          <p className="text-xs text-slate-400">
            {receipt.clubName} — Est. 1938 — Deshapriya Park, Kolkata
          </p>
        </div>
      </div>
    </div>
  );
}
