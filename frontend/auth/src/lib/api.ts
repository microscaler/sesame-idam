/**
 * Hosted auth surface → identity-login-service API (ADR-010 + OIDC complete).
 *
 * Every call carries X-Tenant-ID (ADR-004 tenant gate). Responses from the
 * OTP/magic-link SEND endpoints are deliberately generic (Gate A3: no
 * enumeration, no cap oracle) — the UI must therefore ALWAYS advance to the
 * "check your inbox/phone" step, never branch on whether the account existed.
 */

const BASE = import.meta.env.VITE_IDAM_BASE_URL ?? '/idam/v1';
const DEFAULT_CLIENT_ID = import.meta.env.VITE_DEFAULT_CLIENT_ID ?? 'hauliage-web';

export interface TokenResponse {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
  user_id: string;
  roles?: string[];
  token_type: string;
}

async function post<T>(
  path: string,
  tenantId: string,
  body: unknown,
  accessToken?: string,
): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'X-Tenant-ID': tenantId,
  };
  if (accessToken) {
    headers.Authorization = `Bearer ${accessToken}`;
  }
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers,
    body: JSON.stringify(body),
    credentials: 'include',
    redirect: 'manual',
  });
  if (res.status >= 300 && res.status < 400) {
    const location = res.headers.get('Location') ?? '';
    return { redirect: location } as T;
  }
  const json = (await res.json().catch(() => ({}))) as Record<string, unknown>;
  if (!res.ok) {
    throw new AuthError(
      String(json.error ?? 'request_failed'),
      String(json.error_description ?? 'Something went wrong'),
    );
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

/** Password login. Requires client_id (OIDC client registry binds tenant). */
export const login = (
  tenantId: string,
  email: string,
  password: string,
  clientId: string = DEFAULT_CLIENT_ID,
) =>
  post<TokenResponse>('/auth/login', tenantId, {
    email,
    password,
    client_id: clientId,
  });

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

/** Request a password-reset link. Always "succeeds" — advance the UI regardless. */
export const forgotPassword = (tenantId: string, email: string) =>
  post<{ success: boolean; message: string }>('/auth/password/forgot', tenantId, { email });

/** Consume a reset token and set a new password. Does NOT sign the user in. */
export const resetPassword = (tenantId: string, token: string, newPassword: string) =>
  post<{ success: boolean; message: string }>('/auth/password/reset', tenantId, {
    token,
    new_password: newPassword,
  });

/**
 * Mint a one-time code so the session can cross to the tenant app's origin
 * (ADR-010). Tokens never travel in the URL — only this single-use,
 * redirect_uri-bound code does.
 */
export const mintSessionCode = (
  tenantId: string,
  accessToken: string,
  refreshToken: string | undefined,
  redirectUri: string,
) =>
  post<{ code: string; expires_in: number }>('/auth/session/code', tenantId, {
    access_token: accessToken,
    refresh_token: refreshToken,
    redirect_uri: redirectUri,
  });

/**
 * Complete an OIDC authorization request after the user authenticates.
 * Returns the RP redirect Location (`redirect_uri?code=&state=`).
 *
 * Uses same-origin `/oauth/authorize/complete` on the auth host (edge rewrite
 * to login-service), not the `/idam/v1` API base.
 */
export async function completeOidcAuthorize(
  tenantId: string,
  accessToken: string,
  requestId: string,
): Promise<{ redirect: string }> {
  const res = await fetch('/oauth/authorize/complete', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Tenant-ID': tenantId,
      Authorization: `Bearer ${accessToken}`,
    },
    body: JSON.stringify({ request_id: requestId }),
    credentials: 'include',
    redirect: 'manual',
  });
  if (res.status >= 300 && res.status < 400) {
    const location = res.headers.get('Location') ?? '';
    if (!location) {
      throw new AuthError('invalid_request', 'Authorization complete returned no redirect');
    }
    return { redirect: location };
  }
  const json = (await res.json().catch(() => ({}))) as Record<string, unknown>;
  throw new AuthError(
    String(json.error ?? 'request_failed'),
    String(json.error_description ?? 'Authorization complete failed'),
  );
}
