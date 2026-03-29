import useSWR, { mutate as globalMutate, type SWRConfiguration } from "swr";

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:5000";

export class ApiError extends Error {
  status: number;
  details: unknown;

  constructor(message: string, status: number, details: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.details = details;
  }
}

async function readResponseBody(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return null;
  }

  const contentType = response.headers.get("content-type") ?? "";

  if (contentType.includes("application/json")) {
    return response.json();
  }

  const text = await response.text();
  return text || null;
}

function getErrorMessage(body: unknown, status: number): string {
  if (body && typeof body === "object" && "error" in body && typeof body.error === "string") {
    return body.error;
  }

  if (typeof body === "string" && body.trim()) {
    return body;
  }

  return `Request failed with status ${status}`;
}

export async function apiFetcher<Data>(url: string): Promise<Data> {
  const response = await fetch(`${API_BASE}${url}`, {
    credentials: "include",
  });

  const body = await readResponseBody(response);

  if (!response.ok) {
    throw new ApiError(getErrorMessage(body, response.status), response.status, body);
  }

  return body as Data;
}

export function useApi<Data>(
  key: string | null,
  config?: SWRConfiguration<Data, ApiError>
) {
  return useSWR<Data, ApiError>(key, apiFetcher, config);
}

export function matchesApiPrefix(key: unknown, prefixes: string[]): boolean {
  if (typeof key !== "string") {
    return false;
  }

  return prefixes.some((prefix) => {
    if (!key.startsWith(prefix)) {
      return false;
    }

    const nextChar = key.charAt(prefix.length);
    return nextChar === "" || nextChar === "?" || nextChar === "/";
  });
}

export async function revalidateApiPrefixes(...prefixes: string[]) {
  return globalMutate(
    (key) => matchesApiPrefix(key, prefixes),
    undefined,
    { revalidate: true }
  );
}
