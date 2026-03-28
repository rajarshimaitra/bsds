"use client";

import { useEffect } from "react";

export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("App error:", error);
  }, [error]);

  return (
    <html lang="en">
      <body>
        <div className="flex min-h-screen items-center justify-center p-6">
          <div className="max-w-md rounded-lg border bg-background p-6 text-center shadow-sm">
            <h1 className="text-lg font-semibold">Application error</h1>
            <p className="mt-2 text-sm text-muted-foreground">
              The app shell hit an unexpected error. Please retry loading the page.
            </p>
            <div className="mt-4 flex justify-center gap-3">
              <button
                type="button"
                onClick={reset}
                className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
              >
                Reload
              </button>
            </div>
            {process.env.NODE_ENV !== "production" && error.digest && (
              <p className="mt-4 break-all text-xs text-muted-foreground">
                Error digest: {error.digest}
              </p>
            )}
          </div>
        </div>
      </body>
    </html>
  );
}
