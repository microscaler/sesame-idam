/**
 * Tenant console API client (ADR-009 §7).
 *
 * Note what is missing from `SmsConfig`: there is no `auth_token`. The server
 * never returns it, so the console cannot display it, so a compromised console
 * session cannot exfiltrate it. The credential is write-only by construction
 * rather than by our remembering to filter it.
 */

// Same-origin by default: the frontend's nginx proxies /idam/ to the identity
// service, which serves its routes under the `/idam/v1` base path. Keeping the
// browser on one origin is what keeps the Gate A5 CORS allow-list small and
// makes the ADR-008 passkey story (WebAuthn is origin-bound) hold at the edge.
const API_BASE = import.meta.env.VITE_IDAM_BASE_URL ?? '/idam/v1';

/** The secret-free view of a tenant's SMS sender. */
export interface SmsConfig {
  tenant_id: string;
  environment: string;
  provider: string;
  custody_mode: 'connect' | 'envelope';
  status: 'pending_validation' | 'active' | 'revoked';
  /** A credential is stored — never the credential itself. */
  credential_configured: boolean;
  account_sid?: string | null;
  connected_account_sid?: string | null;
  messaging_service_sid?: string | null;
  from_number?: string | null;
  campaign_ref?: string | null;
  daily_spend_ceiling_cents: number;
  last_validated_at?: string | null;
}

/**
 * What the console may send. `auth_token` is the only write-only field: omit
 * it to edit everything else without re-entering the secret.
 */
export interface SmsConfigInput {
  custody_mode: 'connect' | 'envelope';
  connected_account_sid?: string;
  account_sid?: string;
  auth_token?: string;
  messaging_service_sid?: string;
  from_number?: string;
  campaign_ref?: string;
  daily_spend_ceiling_cents?: number;
}

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

/**
 * The tenant's own endpoints (`/tenant/sms/{environment}`, ADR-011): the tenant is taken from the bearer token, so
 * the console never names a tenant and cannot point at another one. The operator endpoints under `/platform` need
 * the platform key and are not reachable from a browser.
 */
async function request<T>(accessToken: string, path: string, init?: RequestInit): Promise<T | null> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${accessToken}`,
      ...(init?.headers ?? {}),
    },
  });
  if (res.status === 404) return null;
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new ApiError(res.status, body.error_description ?? `request failed (${res.status})`);
  }
  return (await res.json()) as T;
}

const path = (env: string) => `/tenant/sms/${encodeURIComponent(env)}`;

export const smsApi = {
  get: (token: string, env: string) => request<SmsConfig>(token, path(env)),

  save: (token: string, env: string, body: SmsConfigInput) =>
    request<SmsConfig>(token, path(env), { method: 'PUT', body: JSON.stringify(body) }),

  revoke: (token: string, env: string) =>
    request<SmsConfig>(token, path(env), { method: 'DELETE' }),
};
