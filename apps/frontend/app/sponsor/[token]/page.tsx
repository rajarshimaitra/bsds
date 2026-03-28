"use client";

/**
 * /sponsor/[token] — Public Sponsor Checkout Page
 *
 * No authentication required.
 * Fetches sponsor link data from GET /api/sponsor-links/[token].
 * Shows club info, sponsorship purpose, amount (fixed or input), UPI details.
 * Provides "Pay via Razorpay" button that opens the Razorpay checkout modal.
 *
 * Flow:
 *   1. mount → fetch /api/sponsor-links/[token]
 *   2. If expired/inactive → show 410 error page
 *   3. If amount is null → show amount input
 *   4. "Pay via Razorpay" →
 *      a. POST /api/payments/sponsor-order  (create order, no auth needed)
 *      b. Open Razorpay checkout.js modal
 *      c. On success → POST /api/payments/sponsor-verify  (HMAC check)
 *      d. redirect → /sponsor/[token]/receipt?paymentId=xxx
 * 5. On failure → show error message
 *
 * Razorpay checkout.js is loaded via a <script> tag injected into <head>.
 * We declare the global Razorpay constructor via a Window interface extension.
 */

import { useEffect, useState, useRef } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { CheckCircleIcon, AlertCircleIcon, LoaderIcon, CopyIcon, CheckIcon, ExternalLinkIcon } from "lucide-react";
import { apiFetch } from "@/lib/api-client";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SponsorLinkData {
  token: string;
  sponsorName: string | null;
  sponsorCompany: string | null;
  amount: number | null;
  purpose: string;
  purposeLabel: string;
  upiId: string;
  bankDetails: {
    accountNumber?: string;
    bankName?: string;
    ifscCode?: string;
  } | null;
  isActive: boolean;
  isExpired: boolean;
  clubName: string;
}

interface RazorpayOptions {
  key: string;
  amount: number;
  currency: string;
  name: string;
  description: string;
  order_id: string;
  prefill?: {
    name?: string;
    email?: string;
    contact?: string;
  };
  notes?: Record<string, string>;
  theme?: { color: string };
  handler: (response: {
    razorpay_payment_id: string;
    razorpay_order_id: string;
    razorpay_signature: string;
  }) => void;
  modal?: {
    ondismiss?: () => void;
  };
}

declare global {
  interface Window {
    Razorpay: new (options: RazorpayOptions) => { open: () => void };
  }
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

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function SponsorCheckoutPage({
  params,
}: {
  params: { token: string };
}) {
  const router = useRouter();
  const { token } = params;

  // ---- State ----
  const [linkData, setLinkData] = useState<SponsorLinkData | null>(null);
  const [loading, setLoading] = useState(true);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [isGone, setIsGone] = useState(false); // 410 expired/inactive

  // Amount input (used when linkData.amount is null)
  const [customAmount, setCustomAmount] = useState("");
  const [amountError, setAmountError] = useState<string | null>(null);

  // Payment state
  const [paying, setPaying] = useState(false);
  const [payError, setPayError] = useState<string | null>(null);

  // Copy UPI state
  const [copiedUpi, setCopiedUpi] = useState(false);

  const scriptLoadedRef = useRef(false);

  // ---- Load Razorpay checkout.js ----
  useEffect(() => {
    if (scriptLoadedRef.current) return;
    scriptLoadedRef.current = true;

    const script = document.createElement("script");
    script.src = "https://checkout.razorpay.com/v1/checkout.js";
    script.async = true;
    document.head.appendChild(script);

    return () => {
      // Leave script tag — it's safe to keep
    };
  }, []);

  // ---- Fetch link data ----
  useEffect(() => {
    async function fetchLink() {
      try {
        const res = await apiFetch(`/api/sponsor-links/${token}`);

        if (res.status === 410) {
          const data = await res.json();
          setIsGone(true);
          setLinkData(data);
          setLoading(false);
          return;
        }

        if (res.status === 404) {
          setErrorMsg("This payment link does not exist. Please check the URL.");
          setLoading(false);
          return;
        }

        if (!res.ok) {
          setErrorMsg("Unable to load payment details. Please try again.");
          setLoading(false);
          return;
        }

        const data: SponsorLinkData = await res.json();
        setLinkData(data);
      } catch {
        setErrorMsg("Network error. Please check your connection and try again.");
      } finally {
        setLoading(false);
      }
    }

    fetchLink();
  }, [token]);

  // ---- Copy UPI ID ----
  async function handleCopyUpi(upiId: string) {
    try {
      await navigator.clipboard.writeText(upiId);
      setCopiedUpi(true);
      setTimeout(() => setCopiedUpi(false), 2000);
    } catch {
      // Silent fail
    }
  }

  // ---- Validate custom amount ----
  function getPaymentAmount(): number | null {
    if (!linkData) return null;

    if (linkData.amount !== null) {
      return linkData.amount;
    }

    const parsed = parseFloat(customAmount);
    if (!customAmount || isNaN(parsed) || parsed < 1) {
      setAmountError("Please enter a valid amount (minimum ₹1)");
      return null;
    }

    setAmountError(null);
    return parsed;
  }

  // ---- Pay via Razorpay ----
  async function handlePayWithRazorpay() {
    if (!linkData) return;

    const amount = getPaymentAmount();
    if (amount === null) return;

    setPayError(null);
    setPaying(true);

    try {
      // Step 1: Create order
      const orderRes = await apiFetch("/api/payments/sponsor-order", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token,
          amount,
          sponsorPurpose: linkData.purpose,
          sponsorId: null, // resolved server-side from link
        }),
      });

      const orderData = await orderRes.json();

      if (!orderRes.ok) {
        setPayError(orderData.error ?? "Failed to create payment order. Please try again.");
        setPaying(false);
        return;
      }

      const { orderId, keyId } = orderData;

      // Step 2: Open Razorpay modal
      if (typeof window === "undefined" || !window.Razorpay) {
        setPayError("Payment system is loading. Please try again in a moment.");
        setPaying(false);
        return;
      }

      const rzp = new window.Razorpay({
        key: keyId,
        amount: Math.round(amount * 100), // paise
        currency: "INR",
        name: linkData.clubName,
        description: `${linkData.purposeLabel}${linkData.sponsorName ? ` — ${linkData.sponsorName}` : ""}`,
        order_id: orderId,
        prefill: {
          name: linkData.sponsorName ?? undefined,
        },
        notes: {
          sponsorLinkToken: token,
          sponsorPurpose: linkData.purpose,
          ...(linkData.sponsorName ? { sponsorName: linkData.sponsorName } : {}),
        },
        theme: { color: "#0f172a" },
        handler: async (response) => {
          // Step 3: Verify payment
          try {
            const verifyRes = await apiFetch("/api/payments/sponsor-verify", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                razorpay_order_id: response.razorpay_order_id,
                razorpay_payment_id: response.razorpay_payment_id,
                razorpay_signature: response.razorpay_signature,
              }),
            });

            // Even if verify fails, the payment is captured by webhook.
            // Redirect to receipt with the payment ID regardless.
            router.push(
              `/sponsor/${token}/receipt?paymentId=${response.razorpay_payment_id}`
            );

            if (!verifyRes.ok) {
              console.warn("[checkout] verify returned non-200 but payment was captured");
            }
          } catch (err) {
            console.error("[checkout] verify error:", err);
            // Still redirect — webhook will handle the actual processing
            router.push(
              `/sponsor/${token}/receipt?paymentId=${response.razorpay_payment_id}`
            );
          }
        },
        modal: {
          ondismiss: () => {
            setPaying(false);
          },
        },
      });

      rzp.open();
    } catch (err) {
      console.error("[checkout] payment error:", err);
      setPayError("An unexpected error occurred. Please try again.");
      setPaying(false);
    }
  }

  // =========================================================================
  // Render states
  // =========================================================================

  // Loading skeleton
  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.2),_transparent_20rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)] p-4">
        <div className="w-full max-w-md rounded-[1.75rem] border border-white/80 bg-white/90 p-8 text-center shadow-[0_32px_80px_-40px_rgba(15,23,42,0.45)]">
          <LoaderIcon className="mx-auto mb-4 h-8 w-8 animate-spin text-sky-500" />
          <p className="text-sm text-slate-500">Loading payment details...</p>
        </div>
      </div>
    );
  }

  // Not found
  if (errorMsg) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.2),_transparent_20rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)] p-4">
        <div className="w-full max-w-md rounded-[1.75rem] border border-white/80 bg-white/90 p-8 text-center shadow-[0_32px_80px_-40px_rgba(15,23,42,0.45)]">
          <AlertCircleIcon className="mx-auto mb-4 h-12 w-12 text-rose-400" />
          <h1 className="mb-2 text-xl font-bold text-slate-900">Link Not Found</h1>
          <p className="mb-6 text-sm text-slate-500">{errorMsg}</p>
          <Link
            href="/"
            className="inline-flex items-center gap-2 text-sm font-medium text-sky-700 hover:text-slate-900"
          >
            Return to Homepage
          </Link>
        </div>
      </div>
    );
  }

  // Expired or inactive (410)
  if (isGone && linkData) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.2),_transparent_20rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)] p-4">
        <div className="w-full max-w-md rounded-[1.75rem] border border-white/80 bg-white/90 p-8 text-center shadow-[0_32px_80px_-40px_rgba(15,23,42,0.45)]">
          <AlertCircleIcon className="mx-auto mb-4 h-12 w-12 text-amber-400" />
          <h1 className="mb-2 text-xl font-bold text-slate-900">
            {linkData.isExpired ? "Payment Link Expired" : "Payment Link Inactive"}
          </h1>
          <p className="mb-2 text-sm text-slate-500">
            {linkData.clubName}
          </p>
          <p className="mb-6 text-sm text-slate-400">
            {linkData.isExpired
              ? "This sponsorship payment link has expired. Please contact the club for a new link."
              : "This payment link has been deactivated. Please contact the club for assistance."}
          </p>
          <Link
            href="/"
            className="inline-flex items-center gap-2 text-sm font-medium text-sky-700 hover:text-slate-900"
          >
            Return to Homepage
          </Link>
        </div>
      </div>
    );
  }

  if (!linkData) return null;

  // =========================================================================
  // Main checkout UI
  // =========================================================================

  const isFixedAmount = linkData.amount !== null;
  const displayAmount = isFixedAmount ? linkData.amount! : parseFloat(customAmount) || 0;

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.22),_transparent_24rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)]">
      {/* Header bar */}
      <div className="bg-gradient-to-r from-slate-950 via-slate-900 to-sky-700 px-6 py-4 shadow-md">
        <div className="max-w-lg mx-auto flex items-center justify-between">
          <div>
            <p className="text-white/80 text-xs uppercase tracking-widest font-medium">
              Sponsor Payment
            </p>
            <h1 className="text-white text-lg font-bold leading-tight">
              {linkData.clubName}
            </h1>
          </div>
          <div className="w-10 h-10 rounded-full bg-white/20 flex items-center justify-center">
            <span className="text-white text-xl" aria-label="Om">ॐ</span>
          </div>
        </div>
      </div>

      {/* Main card */}
      <div className="max-w-lg mx-auto p-4 pt-6">
        <div className="bg-white rounded-2xl shadow-lg overflow-hidden">
          {/* Sponsorship info block */}
          <div className="p-6 border-b border-gray-100">
            <div className="flex items-start justify-between mb-4">
              <div>
                <span className="mb-2 inline-block rounded-full bg-sky-100 px-3 py-1 text-xs font-semibold uppercase tracking-wide text-sky-700">
                  {linkData.purposeLabel}
                </span>
                {linkData.sponsorName && (
                  <div className="mt-1">
                    <p className="text-lg font-bold text-slate-900">{linkData.sponsorName}</p>
                    {linkData.sponsorCompany && (
                      <p className="text-sm text-slate-500">{linkData.sponsorCompany}</p>
                    )}
                  </div>
                )}
              </div>
              {isFixedAmount && (
                <div className="text-right">
                  <p className="text-xs uppercase tracking-wide text-slate-400">Amount</p>
                  <p className="text-2xl font-bold text-slate-900">
                    {formatCurrency(linkData.amount!)}
                  </p>
                </div>
              )}
            </div>

            <p className="text-sm leading-relaxed text-slate-600">
              Your sponsorship supports the annual Durga Puja celebration at Deshapriya Park,
              one of Kolkata&apos;s most heritage-rich cultural festivities since 1938.
            </p>
          </div>

          {/* Amount input (open-ended only) */}
          {!isFixedAmount && (
            <div className="border-b border-slate-100 p-6">
              <label className="mb-2 block text-sm font-semibold text-slate-700">
                Sponsorship Amount (INR) *
              </label>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-lg font-medium text-slate-400">
                  ₹
                </span>
                <input
                  type="number"
                  min="1"
                  step="1"
                  value={customAmount}
                  onChange={(e) => {
                    setCustomAmount(e.target.value);
                    setAmountError(null);
                  }}
                  placeholder="Enter amount"
                  className="w-full rounded-xl border border-white/70 bg-white/80 py-3 pl-8 pr-4 text-lg font-semibold text-slate-900 focus:border-sky-400 focus:outline-none focus:ring-2 focus:ring-sky-400"
                />
              </div>
              {amountError && (
                <p className="mt-1 text-sm text-red-600">{amountError}</p>
              )}
            </div>
          )}

          {/* Payment methods */}
          <div className="p-6 space-y-4">
            {/* UPI section */}
            <div className="rounded-xl border border-sky-200 bg-sky-50 p-4">
              <h3 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-700">
                Option 1 — Pay via UPI
              </h3>
              <div className="flex items-center gap-3 rounded-lg border border-sky-200 bg-white px-4 py-3">
                <div className="flex-1">
                  <p className="mb-1 text-xs text-slate-400">UPI ID</p>
                  <p className="text-base font-mono font-semibold text-slate-900">
                    {linkData.upiId}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => handleCopyUpi(linkData.upiId)}
                  className="flex-shrink-0 rounded-lg bg-sky-100 p-2 transition-colors hover:bg-sky-200"
                  title="Copy UPI ID"
                >
                  {copiedUpi ? (
                    <CheckIcon className="h-4 w-4 text-emerald-600" />
                  ) : (
                    <CopyIcon className="h-4 w-4 text-sky-700" />
                  )}
                </button>
              </div>
              <p className="mt-2 text-xs text-slate-500">
                Open any UPI app (GPay, PhonePe, Paytm, BHIM) and pay to the UPI ID above.
              </p>
            </div>

            {/* Bank transfer section */}
            {linkData.bankDetails &&
              (linkData.bankDetails.accountNumber ||
                linkData.bankDetails.bankName ||
                linkData.bankDetails.ifscCode) && (
              <div className="rounded-xl border border-indigo-200 bg-indigo-50/70 p-4">
                <h3 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-700">
                  Option 2 — Bank Transfer (NEFT/IMPS/RTGS)
                </h3>
                <div className="space-y-2 text-sm">
                  {linkData.bankDetails.accountNumber && (
                    <div className="flex justify-between">
                      <span className="text-slate-500">Account Number</span>
                      <span className="font-mono font-semibold text-slate-900">
                        {linkData.bankDetails.accountNumber}
                      </span>
                    </div>
                  )}
                  {linkData.bankDetails.ifscCode && (
                    <div className="flex justify-between">
                      <span className="text-slate-500">IFSC Code</span>
                      <span className="font-mono font-semibold text-slate-900">
                        {linkData.bankDetails.ifscCode}
                      </span>
                    </div>
                  )}
                  {linkData.bankDetails.bankName && (
                    <div className="flex justify-between">
                      <span className="text-slate-500">Bank</span>
                      <span className="font-semibold text-slate-900">
                        {linkData.bankDetails.bankName}
                      </span>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Razorpay payment button */}
            <div className="pt-2">
              <h3 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-700">
                Option 3 — Pay Online via Razorpay
              </h3>

              {payError && (
                <div className="mb-3 rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-600">
                  {payError}
                </div>
              )}

              {!isFixedAmount && displayAmount > 0 && (
                <p className="mb-3 text-sm text-slate-500">
                  You will be charged{" "}
                  <span className="font-bold text-slate-800">{formatCurrency(displayAmount)}</span>
                </p>
              )}

              <button
                type="button"
                onClick={handlePayWithRazorpay}
                disabled={paying || (!isFixedAmount && !customAmount)}
                className="flex w-full items-center justify-center gap-3 rounded-xl bg-gradient-to-r from-slate-950 to-sky-700 px-6 py-4 text-base font-bold text-white shadow-md transition-all duration-200 hover:shadow-lg hover:from-slate-900 hover:to-sky-600 disabled:cursor-not-allowed disabled:from-slate-300 disabled:to-slate-300"
              >
                {paying ? (
                  <>
                    <LoaderIcon className="h-5 w-5 animate-spin" />
                    Processing...
                  </>
                ) : (
                  <>
                    <ExternalLinkIcon className="h-5 w-5" />
                    Pay{isFixedAmount ? ` ${formatCurrency(linkData.amount!)}` : customAmount ? ` ${formatCurrency(parseFloat(customAmount))}` : ""} via Razorpay
                  </>
                )}
              </button>

              <p className="mt-3 text-center text-xs text-slate-400">
                Secured by Razorpay. Supports UPI, cards, net banking &amp; wallets.
              </p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="text-center mt-6 pb-8 space-y-2">
          <p className="text-xs text-slate-400">
            {linkData.clubName} — Est. 1938
          </p>
          <Link
            href="/"
            className="inline-flex items-center gap-1 text-xs text-slate-400 transition-colors hover:text-sky-600"
          >
            Return to Homepage
          </Link>
        </div>
      </div>
    </div>
  );
}
