"use client";

/**
 * Login page — email + password form backed by the custom auth client.
 *
 * On success:
 *   - isTempPassword=true  → redirect to /change-password
 *   - isTempPassword=false → redirect to /dashboard
 *
 * Test mode auto-fill buttons (T33):
 *   Visible when NODE_ENV !== 'production' OR NEXT_PUBLIC_TEST_MODE=true.
 *   Clicking a button fills the form and immediately submits it.
 *
 * Note: useSearchParams() must live inside a Suspense boundary (Next.js 14
 * static generation requirement). The inner LoginForm component is
 * wrapped in Suspense below.
 */

import { Suspense, useState, useEffect, FormEvent } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useAuth } from "@/hooks/use-auth";
import Image from "next/image";
import Link from "next/link";
import { UI_DASHBOARD_NAME } from "@/lib/branding";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

// ---------------------------------------------------------------------------
// Test accounts — shown in development / test mode only
// ---------------------------------------------------------------------------

const TEST_ACCOUNTS = [
  { label: "Admin",      email: "admin@bsds.club",      password: "Admin@123" },
  { label: "Operator",   email: "operator@bsds.club",   password: "Operator@123" },
  { label: "Organiser",  email: "organiser@bsds.club",  password: "Organiser@123" },
  { label: "Member 1",   email: "member1@bsds.club",    password: "Member@123" },
  { label: "Member 2",   email: "member2@bsds.club",    password: "Member@123" },
  { label: "Member 3",   email: "member3@bsds.club",    password: "Member@123" },
  { label: "Member 4",   email: "member4@bsds.club",    password: "Member@123" },
  { label: "Member 5",   email: "member5@bsds.club",    password: "Member@123" },
] as const;

const isTestMode =
  process.env.NODE_ENV !== "production" ||
  process.env.NEXT_PUBLIC_TEST_MODE === "true";

// ---------------------------------------------------------------------------
// Inner form — uses useSearchParams(), must be inside Suspense
// ---------------------------------------------------------------------------

function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { login } = useAuth();

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Preserve the callbackUrl if set (e.g. redirect after auth)
  const callbackUrl = searchParams.get("callbackUrl") ?? "/dashboard";

  // Reset cursor on unmount (navigation away from login page)
  useEffect(() => {
    return () => {
      document.body.style.cursor = "";
    };
  }, []);

  async function doSignIn(emailValue: string, passwordValue: string) {
    setError(null);
    setLoading(true);
    document.body.style.cursor = "wait";

    try {
      const result = await login(
        emailValue.toLowerCase().trim(),
        passwordValue
      );

      if (!result.ok) {
        // Generic message — do not reveal whether email exists
        setError(result.error ?? "Invalid email or password.");
        setLoading(false);
        document.body.style.cursor = "";
        return;
      }

      // Success — keep loading=true and cursor=wait so the UI stays
      // locked while the router navigates.
      if (result.mustChangePassword) {
        router.push("/change-password");
      } else {
        router.push(callbackUrl);
      }
      router.refresh();
    } catch {
      setError("An unexpected error occurred. Please try again.");
      setLoading(false);
      document.body.style.cursor = "";
    }
  }

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    await doSignIn(email, password);
  }

  async function handleTestLogin(testEmail: string, testPassword: string) {
    setEmail(testEmail);
    setPassword(testPassword);
    await doSignIn(testEmail, testPassword);
  }

  return (
    <Card className="border-white/80 bg-white/85 shadow-[0_30px_80px_-40px_rgba(15,23,42,0.45)]">
      <CardHeader className="space-y-1">
        <CardTitle className="text-xl tracking-tight">Sign in</CardTitle>
        <CardDescription>
          Enter your registered email and password to access the dashboard.
        </CardDescription>
      </CardHeader>

      <form onSubmit={handleSubmit}>
        <CardContent className="space-y-4">
          {/* Error message */}
          {error && (
            <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
              {error}
            </div>
          )}

          {/* Email field */}
          <div className="space-y-2">
            <Label htmlFor="email">Email address</Label>
            <Input
              id="email"
              type="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              autoComplete="email"
              disabled={loading}
            />
          </div>

          {/* Password field */}
          <div className="space-y-2">
            <Label htmlFor="password">Password</Label>
            <Input
              id="password"
              type="password"
              placeholder="Your password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              autoComplete="current-password"
              disabled={loading}
            />
          </div>

          {/* Test mode auto-fill section */}
          {isTestMode && (
            <div className="space-y-2 rounded-2xl border border-dashed border-amber-200 bg-amber-50/70 px-4 py-3">
              <p className="text-xs font-semibold uppercase tracking-[0.22em] text-amber-700">
                Test accounts — development only
              </p>
              <div className="flex flex-wrap gap-2">
                {TEST_ACCOUNTS.map((account) => (
                  <button
                    key={account.email}
                    type="button"
                    onClick={() =>
                      handleTestLogin(account.email, account.password)
                    }
                    disabled={loading}
                    className="inline-flex items-center rounded-xl border border-white bg-white px-2.5 py-1.5 text-xs font-semibold text-slate-600 shadow-sm transition-colors hover:bg-amber-50 hover:text-slate-900 disabled:cursor-wait disabled:opacity-50"
                  >
                    {account.label}
                  </button>
                ))}
              </div>
            </div>
          )}
        </CardContent>

        <CardFooter className="flex flex-col gap-4">
          <Button
            type="submit"
            className="w-full"
            disabled={loading || !email || !password}
          >
            {loading ? "Signing in..." : "Sign in"}
          </Button>

          <Link
            href="/"
            className="text-center text-sm text-slate-500 hover:text-orange-700"
          >
            Back to home
          </Link>
        </CardFooter>
      </form>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Page — wraps LoginForm in Suspense to satisfy Next.js static generation
// ---------------------------------------------------------------------------

export default function LoginPage() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(234,88,12,0.18),_transparent_20rem),linear-gradient(180deg,#fff7ed_0%,#f8fafc_45%,#ffedd5_100%)] p-4">
      <div className="w-full max-w-md">
        <div className="mb-8 text-center">
          <Link href="/" className="inline-flex flex-col items-center gap-3">
            <Image
              src="/images/logo.jpg"
              alt="Deshapriya Park Sarbojanin Durgotsav"
              width={72}
              height={72}
              className="rounded-full border-2 border-slate-200 object-cover shadow-md"
            />
            <div>
              <h1 className="text-2xl font-bold tracking-tight text-slate-900">
                Deshapriya Park Durga Puja Club
              </h1>
              <p className="mt-1 text-sm uppercase tracking-[0.22em] text-orange-700">
                {UI_DASHBOARD_NAME}
              </p>
            </div>
          </Link>
        </div>

        <Suspense
          fallback={
            <Card className="border-white/80 bg-white/85 shadow-[0_30px_80px_-40px_rgba(15,23,42,0.45)]">
              <CardContent className="py-8 text-center text-sm text-slate-500">
                Loading...
              </CardContent>
            </Card>
          }
        >
          <LoginForm />
        </Suspense>
      </div>
    </div>
  );
}
