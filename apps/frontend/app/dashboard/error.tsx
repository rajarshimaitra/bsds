"use client";

import { useEffect } from "react";

export default function DashboardError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("Dashboard route error:", error);
  }, [error]);

  return (
    <div className="flex min-h-[50vh] items-center justify-center p-6">
      <div className="max-w-md rounded-lg border bg-background p-6 text-center shadow-sm">
        <h2 className="text-lg font-semibold">Something went wrong</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          The approvals page hit an unexpected error. You can retry loading it.
        </p>
        <div className="mt-4 flex justify-center gap-3">
          <button
            type="button"
            onClick={reset}
            className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
          >
            Try again
          </button>
        </div>
        {process.env.NODE_ENV !== "production" && error.digest && (
          <p className="mt-4 break-all text-xs text-muted-foreground">
            Error digest: {error.digest}
          </p>
        )}
      </div>
    </div>
  );
}
