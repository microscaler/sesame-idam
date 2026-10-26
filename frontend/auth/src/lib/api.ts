/**
 * Hosted auth surface → identity-login-service API (ADR-010 + OIDC complete).
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

/**
 * A sign-in that needs a further step (ADR-008, HTTP 202): a second factor (`methods`), or the enrolment of one
 * (`enrollment_required`) when the account's organisation requires a method it does not have. No token exists yet.
 */
export interface MfaChallenge {
  mfa_required: true;
  mfa_token: string;
  expires_in: number;
  methods: Array<'totp' | 'passkey' | 'recovery_code'>;
  enrollment_required?: 'mfa' | 'passkey' | null;
}

/** Every first factor answers tokens or the next step. */
export type SignInResult = TokenResponse | MfaChallenge;

export const isChallenge = (r: SignInResult): r is MfaChallenge =>
  (r as MfaChallenge).mfa_required === true && typeof (r as MfaChallenge).mfa_token === 'string';

export interface MfaFactor {
  id: string;
  kind: 'totp' | 'passkey';
  label: string;
  created_at: number;
  last_used_at?: number | null;
  synced?: boolean | null;
}

export interface MfaFactors {
  factors: MfaFactor[];
  recovery_codes_left: number;
  required: 'none' | 'mfa' | 'passkey';
  totp_available: boolean;
  passkeys_available: boolean;
}

/** A factor was enrolled: the account's first recovery codes (once), and the tokens when a sign-in was waiting. */
export interface Enrolled {
  factor: MfaFactor;
  recovery_codes?: string[];
  tokens?: TokenResponse;
}

export interface PasskeyOptions {
  challenge_id: string;
  options: Record<string, unknown>;
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
  clientId: string,
) =>
  post<SignInResult>('/auth/login', tenantId, {
    email,
    password,
    client_id: clientId,
  });

/** Request an email OTP. Always "succeeds" — advance the UI regardless. */
export const sendEmailOtp = (tenantId: string, email: string) =>
  post<{ success: boolean; message: string }>('/auth/login/email-otp', tenantId, { email });

/** Verify an email OTP → tokens. */
export const verifyEmailOtp = (tenantId: string, email: string, code: string) =>
  post<SignInResult>('/auth/verify/email-otp', tenantId, { email, code });

/** Request an SMS OTP. NOTE: per-login SMS is disabled by default (cost policy). */
export const sendPhoneOtp = (tenantId: string, phone: string) =>
  post<{ success: boolean; message: string }>('/auth/login/phone-otp', tenantId, { phone });

/** Verify an SMS OTP → tokens. */
export const verifyPhoneOtp = (tenantId: string, phone: string, code: string) =>
  post<SignInResult>('/auth/verify/phone-otp', tenantId, { phone, code });

/** Request an email magic link. Always "succeeds". */
export const sendMagicLink = (tenantId: string, email: string) =>
  post<{ success: boolean; message: string }>('/auth/magic-link', tenantId, { email });

/** Consume a magic-link token (the "click") → tokens. */
export const verifyMagicLink = (tenantId: string, token: string) =>
  post<SignInResult>('/auth/verify-magic', tenantId, { token });

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
 * Returns the RP redirect (`redirect_uri?code=&state=`) for the page to navigate to.
 *
 * The endpoint answers `200 {redirect}`, not a 302: a browser never exposes a redirect's Location to fetch().
 * Same-origin `/oauth/authorize/complete` on the auth host (edge rewrite to login-service), not the `/idam/v1` base.
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
  });
  const json = (await res.json().catch(() => ({}))) as Record<string, unknown>;
  if (res.ok && typeof json.redirect === 'string' && json.redirect) {
    return { redirect: json.redirect };
  }
  throw new AuthError(
    String(json.error ?? 'request_failed'),
    String(json.error_description ?? 'Authorization complete failed'),
  );
}

// ── Second factors and passkeys (ADR-008) ─────────────────────────────────────────────────────────────────────────

/** Second step of a sign-in: an authenticator-app code or a recovery code. */
export const mfaVerify = (
  tenantId: string,
  mfaToken: string,
  method: 'totp' | 'recovery_code',
  code: string,
) => post<SignInResult>('/auth/mfa/verify', tenantId, { mfa_token: mfaToken, method, code });

/** Passkey request options: username-less for `clientId`, or the second factor of `mfaToken`'s sign-in. */
export const passkeyChallenge = (tenantId: string, args: { clientId?: string; mfaToken?: string }) =>
  post<PasskeyOptions>('/auth/passkeys/challenge', tenantId, {
    client_id: args.clientId ?? null,
    mfa_token: args.mfaToken ?? null,
  });

/** Finish a passkey sign-in. */
export const passkeyVerify = (
  tenantId: string,
  challengeId: string,
  credential: Record<string, unknown>,
  mfaToken?: string,
) =>
  post<SignInResult>('/auth/passkeys/verify', tenantId, {
    challenge_id: challengeId,
    credential,
    mfa_token: mfaToken ?? null,
  });

/** Open a security session for the signed-in user (recent sign-in required). */
export const mfaManage = (tenantId: string, accessToken: string) =>
  post<{ mfa_token: string; expires_in: number }>('/auth/mfa/manage', tenantId, {}, accessToken);

export const mfaFactors = (tenantId: string, mfaToken: string) =>
  post<MfaFactors>('/auth/mfa/factors', tenantId, { mfa_token: mfaToken });

export const mfaRemoveFactor = (tenantId: string, mfaToken: string, factorId: string) =>
  post<MfaFactors>('/auth/mfa/factors/remove', tenantId, { mfa_token: mfaToken, factor_id: factorId });

export const mfaRenameFactor = (tenantId: string, mfaToken: string, factorId: string, label: string) =>
  post<MfaFactors>('/auth/mfa/factors/rename', tenantId, { mfa_token: mfaToken, factor_id: factorId, label });

export const mfaRecoveryCodes = (tenantId: string, mfaToken: string) =>
  post<{ recovery_codes: string[] }>('/auth/mfa/recovery-codes', tenantId, { mfa_token: mfaToken });

export const totpEnroll = (tenantId: string, mfaToken: string, label?: string) =>
  post<{ factor_id: string; secret: string; otpauth_uri: string; digits: number; period: number }>(
    '/auth/mfa/totp/enroll',
    tenantId,
    { mfa_token: mfaToken, label: label ?? null },
  );

export const totpConfirm = (tenantId: string, mfaToken: string, factorId: string, code: string) =>
  post<Enrolled>('/auth/mfa/totp/confirm', tenantId, { mfa_token: mfaToken, factor_id: factorId, code });

export const passkeyRegisterOptions = (tenantId: string, mfaToken: string) =>
  post<PasskeyOptions>('/auth/passkeys/register/options', tenantId, { mfa_token: mfaToken });

export const passkeyRegister = (
  tenantId: string,
  mfaToken: string,
  challengeId: string,
  credential: Record<string, unknown>,
  label?: string,
) =>
  post<Enrolled>('/auth/passkeys/register', tenantId, {
    mfa_token: mfaToken,
    challenge_id: challengeId,
    credential,
    label: label ?? null,
  });
