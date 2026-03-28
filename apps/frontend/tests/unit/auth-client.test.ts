/**
 * Unit tests for lib/auth-client.ts
 *
 * Covers: login, logout, getMe, changePassword — endpoint URLs, request shape,
 *         success paths (2xx), and error paths (4xx).
 * Does NOT cover: token refresh, session expiry, or cookie management internals.
 * Protects: apps/frontend/lib/auth-client.ts
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { login, logout, getMe, changePassword } from "@/lib/auth-client";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeFetchOk(body: unknown, status = 200): Response {
  return {
    ok: true,
    status,
    json: () => Promise.resolve(body),
  } as unknown as Response;
}

function makeFetchFail(body: unknown, status: number): Response {
  return {
    ok: false,
    status,
    json: () => Promise.resolve(body),
  } as unknown as Response;
}

beforeEach(() => {
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// login
// ---------------------------------------------------------------------------

describe("login", () => {
  it("calls POST /api/auth/login with username and password in JSON body", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(makeFetchOk({ mustChangePassword: false }));
    vi.stubGlobal("fetch", fetchSpy);

    await login("alice", "secret");

    expect(fetchSpy).toHaveBeenCalledOnce();
    const [url, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    expect(url).toMatch(/\/api\/auth\/login$/);
    expect(init.method).toBe("POST");
    expect(init.credentials).toBe("include");
    const sent = JSON.parse(init.body as string);
    expect(sent).toEqual({ username: "alice", password: "secret" });
  });

  it("returns { ok: true, mustChangePassword: false } on 200", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchOk({ mustChangePassword: false }))
    );

    const result = await login("alice", "secret");

    expect(result.ok).toBe(true);
    expect(result.mustChangePassword).toBe(false);
    expect(result.error).toBeUndefined();
  });

  it("returns { ok: true, mustChangePassword: true } when backend signals forced change", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchOk({ mustChangePassword: true }))
    );

    const result = await login("alice", "secret");

    expect(result.ok).toBe(true);
    expect(result.mustChangePassword).toBe(true);
  });

  it("returns { ok: false, error } on 401", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchFail({ error: "Invalid credentials" }, 401))
    );

    const result = await login("alice", "wrong");

    expect(result.ok).toBe(false);
    expect(result.error).toBe("Invalid credentials");
  });

  it("returns { ok: false, error: 'Login failed' } when error body is missing", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        json: () => Promise.reject(new Error("no body")),
      } as unknown as Response)
    );

    const result = await login("alice", "wrong");

    expect(result.ok).toBe(false);
    expect(result.error).toBe("Login failed");
  });

  it("returns { ok: false, error: 'Login failed' } when error body has no error field", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchFail({}, 403))
    );

    const result = await login("alice", "wrong");

    expect(result.ok).toBe(false);
    expect(result.error).toBe("Login failed");
  });
});

// ---------------------------------------------------------------------------
// logout
// ---------------------------------------------------------------------------

describe("logout", () => {
  it("calls POST /api/auth/logout with credentials: include", async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: true } as Response);
    vi.stubGlobal("fetch", fetchSpy);

    await logout();

    expect(fetchSpy).toHaveBeenCalledOnce();
    const [url, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    expect(url).toMatch(/\/api\/auth\/logout$/);
    expect(init.method).toBe("POST");
    expect(init.credentials).toBe("include");
  });

  it("resolves without throwing even if response is not ok", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 500 } as Response)
    );

    await expect(logout()).resolves.toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// getMe
// ---------------------------------------------------------------------------

describe("getMe", () => {
  it("calls GET /api/auth/me with credentials: include", async () => {
    const user = {
      id: "user-1",
      username: "alice",
      role: "ADMIN",
      mustChangePassword: false,
    };
    const fetchSpy = vi.fn().mockResolvedValue(makeFetchOk(user));
    vi.stubGlobal("fetch", fetchSpy);

    await getMe();

    expect(fetchSpy).toHaveBeenCalledOnce();
    const [url, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    expect(url).toMatch(/\/api\/auth\/me$/);
    expect((init as RequestInit).credentials).toBe("include");
    // GET — no explicit method field (fetch defaults to GET)
    expect((init as RequestInit).method).toBeUndefined();
  });

  it("returns the user object on 200", async () => {
    const user = {
      id: "user-1",
      username: "alice",
      role: "ADMIN" as const,
      memberId: "BSDS-2026-0001-00",
      mustChangePassword: false,
    };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(makeFetchOk(user)));

    const result = await getMe();

    expect(result).toEqual(user);
  });

  it("returns null on 401", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchFail({ error: "Unauthorized" }, 401))
    );

    const result = await getMe();

    expect(result).toBeNull();
  });

  it("returns null on any non-ok response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchFail({}, 403))
    );

    const result = await getMe();

    expect(result).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// changePassword
// ---------------------------------------------------------------------------

describe("changePassword", () => {
  it("calls POST /api/auth/change-password with currentPassword and newPassword", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(makeFetchOk({}));
    vi.stubGlobal("fetch", fetchSpy);

    await changePassword("oldPass", "newPass");

    expect(fetchSpy).toHaveBeenCalledOnce();
    const [url, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    expect(url).toMatch(/\/api\/auth\/change-password$/);
    expect(init.method).toBe("POST");
    expect(init.credentials).toBe("include");
    const sent = JSON.parse(init.body as string);
    expect(sent).toEqual({ currentPassword: "oldPass", newPassword: "newPass" });
  });

  it("sends Content-Type: application/json header", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(makeFetchOk({}));
    vi.stubGlobal("fetch", fetchSpy);

    await changePassword("old", "new");

    const [, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Record<string, string>)["Content-Type"]).toBe("application/json");
  });

  it("returns { ok: true } on 200", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(makeFetchOk({})));

    const result = await changePassword("old", "new");

    expect(result.ok).toBe(true);
    expect(result.error).toBeUndefined();
  });

  it("returns { ok: false, error } on 400", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchFail({ error: "Current password is wrong" }, 400))
    );

    const result = await changePassword("wrong-old", "new");

    expect(result.ok).toBe(false);
    expect(result.error).toBe("Current password is wrong");
  });

  it("returns { ok: false, error: 'Failed to change password' } when error body is missing", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 400,
        json: () => Promise.reject(new Error("no body")),
      } as unknown as Response)
    );

    const result = await changePassword("wrong", "new");

    expect(result.ok).toBe(false);
    expect(result.error).toBe("Failed to change password");
  });

  it("returns { ok: false, error: 'Failed to change password' } when response has no error field", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(makeFetchFail({}, 422))
    );

    const result = await changePassword("old", "new");

    expect(result.ok).toBe(false);
    expect(result.error).toBe("Failed to change password");
  });
});
