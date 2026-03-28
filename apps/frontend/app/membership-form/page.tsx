import type { Metadata } from "next";
import Link from "next/link";
import PrintButton from "@/components/landing/PrintButton";

export const metadata: Metadata = {
  title: "Membership Application Form — Deshapriya Park Sarbojanin Durgotsav",
  description:
    "Print and fill this membership application form to apply for membership at Deshapriya Park Sarbojanin Durgotsav, Kolkata.",
};

export default function MembershipFormPage() {
  return (
    <>
      {/* Global print styles injected via a style tag — safe in server components */}
      <style
        dangerouslySetInnerHTML={{
          __html: `
            @media print {
              .no-print { display: none !important; }
              body { background: white !important; }
              .print-page {
                max-width: 100% !important;
                margin: 0 !important;
                padding: 16mm 20mm !important;
                box-shadow: none !important;
                border: none !important;
              }
              @page {
                size: A4;
                margin: 10mm;
              }
            }
          `,
        }}
      />

      {/* Back link + Print button — hidden when printing */}
      <div className="no-print flex items-center justify-between border-b border-sky-100 bg-sky-50/80 px-4 py-3">
        <Link
          href="/"
          className="inline-flex items-center gap-2 text-sm font-medium text-sky-700 transition-colors hover:text-slate-900"
        >
          ← Back to Home
        </Link>
        <PrintButton />
      </div>

      {/* Page wrapper — centres the A4 card on screen */}
      <div className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.16),_transparent_20rem),linear-gradient(180deg,#eff6ff_0%,#f8fafc_55%,#eef2ff_100%)] px-4 py-8 print:bg-white print:py-0">
        <div className="print-page mx-auto max-w-[794px] rounded-sm border border-sky-100 bg-white p-10 shadow-[0_24px_60px_-32px_rgba(15,23,42,0.28)] sm:p-12 print:max-w-none print:rounded-none print:border-none print:shadow-none">

          {/* ── Header ─────────────────────────────────────────── */}
          <header className="mb-8 border-b-2 border-sky-600 pb-6 text-center">
            <div className="text-3xl mb-2" aria-hidden="true">🪔</div>
            <h1 className="mb-1 text-xl font-extrabold uppercase leading-tight tracking-wide text-slate-900 sm:text-2xl">
              Deshapriya Park Sarbojanin Durgotsav
            </h1>
            <p className="text-sm text-slate-600">
              Deshapriya Park, Tilak Road / 34A Manoharpukur Road, Ballygunge, Kolkata — 700029
            </p>
            <p className="mt-1 text-xs text-slate-400">
              Opposite Priya Cinema, near Rash Behari Avenue &nbsp;|&nbsp; Est. 1938
            </p>
            <div className="mt-5 inline-block rounded border-2 border-sky-600 px-6 py-2">
              <span className="text-base font-bold uppercase tracking-widest text-sky-700 sm:text-lg">
                Membership Application Form
              </span>
            </div>
          </header>

          {/* ── Instructions Block ──────────────────────────────── */}
          <section className="mb-8 space-y-2 rounded-lg border border-sky-200 bg-sky-50 p-5 text-sm text-slate-700 print:border print:border-gray-300 print:bg-white">
            <h2 className="mb-3 text-base font-bold uppercase tracking-wide text-slate-900">
              Instructions
            </h2>
            <ol className="list-decimal list-inside space-y-1.5 text-sm leading-relaxed">
              <li>Please fill this form in <strong>BLOCK LETTERS</strong> using a blue or black ballpoint pen.</li>
              <li>
                Submit the completed form at the{" "}
                <strong>Club Office, Deshapriya Park, Ballygunge, Kolkata — 700029</strong>.
              </li>
              <li>
                For queries, contact the club operator at:{" "}
                <strong>+91 94330 82863</strong>.
              </li>
              <li>
                <strong>Application Fee (one-time): ₹10,000</strong> &nbsp;|&nbsp; Paid separately upon approval.
              </li>
              <li>
                <strong>Membership Fee:</strong>&nbsp;
                Monthly ₹250 &nbsp;/&nbsp; Half-yearly ₹1,500 &nbsp;/&nbsp; Annual ₹3,000.
              </li>
              <li>
                <strong>Accepted Payment Modes:</strong> UPI, Bank Transfer, Cash.
              </li>
              <li>
                All phone number fields refer to <strong>WhatsApp numbers</strong> (required for
                membership notifications).
              </li>
            </ol>
          </section>

          {/* ── Primary Member Details ──────────────────────────── */}
          <section className="mb-8">
            <h2 className="mb-5 border-b border-slate-300 pb-1 text-base font-bold uppercase tracking-wide text-slate-900">
              Primary Member Details
            </h2>

            <div className="space-y-6 text-sm text-slate-700">
              <FormField label="Full Name (as per ID proof)" wide />
              <FormField label="WhatsApp Number" />
              <FormField label="Email Address" />
              <div>
                <div className="font-medium mb-1">
                  Address <span className="font-normal text-slate-400">(full residential address)</span>
                </div>
                <div className="mb-3 mt-4 h-0 border-b border-slate-400" />
                <div className="mb-3 h-0 border-b border-slate-400" />
                <div className="h-0 border-b border-slate-400" />
              </div>
            </div>
          </section>

          {/* ── Membership Type ─────────────────────────────────── */}
          <section className="mb-8">
            <h2 className="mb-5 border-b border-slate-300 pb-1 text-base font-bold uppercase tracking-wide text-slate-900">
              Membership Type Selection
            </h2>
            <p className="mb-4 text-sm text-slate-600">
              Please tick (&nbsp;&#10003;&nbsp;) your preferred membership period:
            </p>
            <div className="flex flex-wrap gap-8 text-sm text-slate-700">
              <CheckboxField label="Monthly — ₹250 / month" />
              <CheckboxField label="Half-yearly — ₹1,500 / 6 months" />
              <CheckboxField label="Annual — ₹3,000 / year" />
            </div>
          </section>

          {/* ── Sub-Members ─────────────────────────────────────── */}
          <section className="mb-8">
            <h2 className="mb-2 border-b border-slate-300 pb-1 text-base font-bold uppercase tracking-wide text-slate-900">
              Sub-Members
            </h2>
            <p className="mb-5 text-sm text-slate-500">
              Up to 3 sub-members (family members) may be added. No additional fee required for sub-members.
            </p>

            {[1, 2, 3].map((n) => (
              <div key={n} className="mb-7">
                <div className="mb-3 text-sm font-semibold text-slate-700">
                  Sub-member {n}
                </div>
                <div className="grid grid-cols-1 gap-6 text-sm text-slate-700 sm:grid-cols-3">
                  <FormField label="Full Name" />
                  <FormField label="WhatsApp Number" />
                  <FormField label="Relation to Primary Member" />
                </div>
              </div>
            ))}
          </section>

          {/* ── Declaration ─────────────────────────────────────── */}
          <section className="mb-8 rounded-lg border border-slate-200 bg-slate-50/80 p-5 print:border print:border-gray-300 print:bg-white">
            <h2 className="mb-3 text-base font-bold uppercase tracking-wide text-slate-900">
              Declaration
            </h2>
            <p className="text-sm leading-relaxed text-slate-700">
              I, the undersigned, hereby apply for membership of{" "}
              <strong>Deshapriya Park Sarbojanin Durgotsav</strong> and declare that the
              information furnished above is true and correct to the best of my knowledge. I
              agree to abide by the rules and regulations of the club and understand that my
              membership is subject to the approval of the club committee.
            </p>
          </section>

          {/* ── Signature Line ──────────────────────────────────── */}
          <section>
            <div className="grid grid-cols-1 gap-10 text-sm text-slate-700 sm:grid-cols-2">
              <div>
                <div className="mb-1 font-medium">Date</div>
                <div className="border-b border-gray-400 mt-8 mb-1" />
                <div className="text-xs text-slate-400">DD / MM / YYYY</div>
              </div>
              <div>
                <div className="mb-1 font-medium">Signature of Applicant</div>
                <div className="border-b border-gray-400 mt-8 mb-1" />
                <div className="text-xs text-slate-400">&nbsp;</div>
              </div>
            </div>

            {/* Office use only block */}
            <div className="mt-10 rounded-lg border-2 border-dashed border-slate-300 p-4 text-sm text-slate-500">
              <p className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-600">
                For Office Use Only
              </p>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-6">
                <div>
                  <div className="mb-1">Member ID</div>
                  <div className="border-b border-gray-300 mt-6" />
                </div>
                <div>
                  <div className="mb-1">Received by</div>
                  <div className="border-b border-gray-300 mt-6" />
                </div>
                <div>
                  <div className="mb-1">Date of Entry</div>
                  <div className="border-b border-gray-300 mt-6" />
                </div>
              </div>
            </div>
          </section>

          {/* ── Footer note ─────────────────────────────────────── */}
          <div className="mt-8 border-t border-slate-100 pt-4 text-center text-xs text-slate-400">
            Deshapriya Park Sarbojanin Durgotsav · Est. 1938 · Ballygunge, Kolkata 700029 · +91 94330 82863
          </div>
        </div>

        {/* Bottom navigation — hidden when printing */}
        <div className="no-print mt-6 text-center">
          <Link
            href="/"
            className="text-sm font-medium text-sky-700 hover:underline"
          >
            ← Return to Home
          </Link>
        </div>
      </div>
    </>
  );
}

/* ─── Helper Components ──────────────────────────────────────────────────── */

/** A labelled form field rendered as a blank underline for handwriting. */
function FormField({ label, wide = false }: { label: string; wide?: boolean }) {
  return (
    <div className={wide ? "col-span-full" : ""}>
      <div className="font-medium mb-1">{label}</div>
      <div className="mt-5 border-b border-slate-400" />
    </div>
  );
}

/** A checkbox with a label, rendered as an empty square for ticking. */
function CheckboxField({ label }: { label: string }) {
  return (
    <label className="flex items-center gap-2 cursor-default">
      <span
        className="inline-block h-4 w-4 flex-shrink-0 rounded-sm border-2 border-slate-500"
        aria-hidden="true"
      />
      <span>{label}</span>
    </label>
  );
}
