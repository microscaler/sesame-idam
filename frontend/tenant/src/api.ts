/**
 * Tenant console API client (ADR-009 §7).
 *
 * Note what is missing from `SmsConfig`: there is no `auth_token`. The server
 * never returns it, so the console cannot display it, so a compromised console
 * session cannot exfiltrate it. The credential is write-only by construction
 * rather than by our remembering to filter it.
 */

const API_BASE = import.meta.env.VITE_IDAM_BASE_URL ?? 'https://sesame-idam.dev.microscaler.local';

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

async function request<T>(path: string, init?: RequestInit): Promise<T | null> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    credentials: 'include',
    headers: { 'content-type': 'application/json', ...(init?.headers ?? {}) },
  });
  if (res.status === 404) return null;
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new ApiError(res.status, body.error_description ?? `request failed (${res.status})`);
  }
  return (await res.json()) as T;
}

const path = (tenant: string, env: string) =>
  `/platform/tenants/${encodeURIComponent(tenant)}/sms/${encodeURIComponent(env)}`;

export const smsApi = {
  get: (tenant: string, env: string) => request<SmsConfig>(path(tenant, env)),

  save: (tenant: string, env: string, body: SmsConfigInput) =>
    request<SmsConfig>(path(tenant, env), { method: 'PUT', body: JSON.stringify(body) }),

  revoke: (tenant: string, env: string) =>
    request<SmsConfig>(path(tenant, env), { method: 'DELETE' }),
};
