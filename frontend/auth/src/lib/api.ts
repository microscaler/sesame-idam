/**
 * Hosted auth surface → identity-login-service API (ADR-010).
 *
 * Every call carries X-Tenant-ID (ADR-004 tenant gate). Responses from the
 * OTP/magic-link SEND endpoints are deliberately generic (Gate A3: no
 * enumeration, no cap oracle) — the UI must therefore ALWAYS advance to the
 * "check your inbox/phone" step, never branch on whether the account existed.
 */

const BASE = import.meta.env.VITE_IDAM_BASE_URL ?? '/idam/v1';

export interface TokenResponse {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
  user_id: string;
  roles?: string[];
  token_type: string;
}

async function post<T>(path: string, tenantId: string, body: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Tenant-ID': tenantId },
    body: JSON.stringify(body),
    credentials: 'include',
  });
  const json = (await res.json().catch(() => ({}))) as Record<string, unknown>;
  if (!res.ok) {
    // Generic by design — surface the server's message, never invent detail.
    throw new AuthError(String(json.error ?? 'request_failed'), String(json.error_description ?? 'Something went wrong'));
  }
  return json as T;
}

export class AuthError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
  }
}

/** Password login. 401 `invalid_credentials` covers wrong password, unknown user, AND lockout (A2). */
export const login = (tenantId: string, email: string, password: string) =>
  post<TokenResponse>('/auth/login', tenantId, { email, password });

/** Request an email OTP. Always "succeeds" — advance the UI regardless. */
export const sendEmailOtp = (tenantId: string, email: string) =>
  post<{ success: boolean; message: string }>('/auth/login/email-otp', tenantId, { email });

/** Verify an email OTP → tokens. */
export const verifyEmailOtp = (tenantId: string, email: string, code: string) =>
  post<TokenResponse>('/auth/verify/email-otp', tenantId, { email, code });

/** Request an SMS OTP. NOTE: per-login SMS is disabled by default (cost policy). */
export const sendPhoneOtp = (tenantId: string, phone: string) =>
  post<{ success: boolean; message: string }>('/auth/login/phone-otp', tenantId, { phone });

/** Verify an SMS OTP → tokens. */
export const verifyPhoneOtp = (tenantId: string, phone: string, code: string) =>
  post<TokenResponse>('/auth/verify/phone-otp', tenantId, { phone, code });

/** Request an email magic link. Always "succeeds". */
export const sendMagicLink = (tenantId: string, email: string) =>
  post<{ success: boolean; message: string }>('/auth/magic-link', tenantId, { email });

/** Consume a magic-link token (the "click") → tokens. */
export const verifyMagicLink = (tenantId: string, token: string) =>
  post<TokenResponse>('/auth/verify-magic', tenantId, { token });
