"use client";

/**
 * Print button for the membership application form.
 * Hidden automatically via .no-print class when the user prints.
 */
export default function PrintButton() {
  return (
    <button
      type="button"
      onClick={() => window.print()}
      className="no-print inline-flex items-center gap-2 rounded-xl bg-slate-900 px-4 py-1.5 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-sky-600 active:bg-sky-700"
    >
      🖨️ Print Form
    </button>
  );
}
