"use client";

import { Suspense, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import { ReceiptView } from "@/components/receipts/ReceiptView";
import type { ReceiptData } from "@/lib/receipt-utils";

function decodeReceiptPayload(payload: string | null): ReceiptData | null {
  if (!payload) return null;

  try {
    const json = decodeURIComponent(escape(window.atob(decodeURIComponent(payload))));
    return JSON.parse(json) as ReceiptData;
  } catch {
    return null;
  }
}

function ReceiptPrintContent() {
  const searchParams = useSearchParams();
  const receipt = useMemo(
    () => decodeReceiptPayload(searchParams.get("payload")),
    [searchParams]
  );
  const [hasPrinted, setHasPrinted] = useState(false);

  useEffect(() => {
    if (!receipt || hasPrinted) return;

    const timer = window.setTimeout(() => {
      window.print();
      setHasPrinted(true);
    }, 300);

    return () => window.clearTimeout(timer);
  }, [receipt, hasPrinted]);

  return receipt ? (
    <ReceiptView receipt={receipt} showPrintButton={false} />
  ) : (
    <div className="mx-auto max-w-xl rounded-2xl border border-rose-200 bg-rose-50 px-5 py-4 text-sm text-rose-700">
      Receipt data could not be loaded for printing.
    </div>
  );
}

export default function ReceiptPrintPage() {
  return (
    <main className="min-h-screen bg-white px-3 py-4 sm:px-6 print:p-0 print:bg-white">
      <Suspense fallback={<div className="text-center py-8 text-muted-foreground">Loading receipt…</div>}>
        <ReceiptPrintContent />
      </Suspense>
    </main>
  );
}
