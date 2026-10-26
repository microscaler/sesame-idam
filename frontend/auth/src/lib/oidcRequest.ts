/**
 * The hosted login page's OIDC authorization request (`request_id`): is it still usable, and where does the
 * application start a fresh one when it is not?
 *
 * A request lives 30 minutes and is used once. A page left open longer (or reloaded after a sign-in) can no longer
 * complete, so the page checks on load, whenever its tab is shown again, and when the request is due to expire. An
 * expired request sends the browser to the client's registered `initiate_login_uri` (OIDC client metadata) at once,
 * before the user types anything; the application then begins a new request (new state, nonce and PKCE) and comes back
 * here. Only the registered URI, as Sesame returns it, is ever used: nothing from this page's own URL (no open redirect).
 *
 * Kept free of DOM and framework code so it runs under `node --test` (src/lib/oidcRequest.test.mjs).
 */

export interface AuthorizeStatus {
  active: boolean;
  expires_in: number;
  initiate_login_uri: string | null;
}

/** What the page should do about its request. */
export type RequestCheck =
  /** Usable; check again after `recheckInMs`. */
  | { kind: 'ok'; recheckInMs: number }
  /** Expired or used: go to the application's sign-in start now. */
  | { kind: 'restart'; to: string }
  /** Expired or used, and the application registered no sign-in start: say so. */
  | { kind: 'expired' }
  /** Could not tell (network, store unavailable): leave the page as it is. */
  | { kind: 'unknown' };

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

/** Same-origin `/oauth/authorize/status` on the auth host (edge rewrite to login-service). Never throws. */
export async function fetchAuthorizeStatus(
  requestId: string,
  clientId: string,
  fetchImpl: FetchLike = (input, init) => fetch(input, init),
): Promise<AuthorizeStatus | null> {
  const query = new URLSearchParams({ request_id: requestId, client_id: clientId });
  try {
    const res = await fetchImpl(`/oauth/authorize/status?${query.toString()}`, {
      headers: { Accept: 'application/json' },
      cache: 'no-store',
      credentials: 'omit',
    });
    if (!res.ok) return null;
    const json = (await res.json()) as Partial<AuthorizeStatus>;
    if (typeof json.active !== 'boolean') return null;
    return {
      active: json.active,
      expires_in: typeof json.expires_in === 'number' && json.expires_in > 0 ? json.expires_in : 0,
      initiate_login_uri: typeof json.initiate_login_uri === 'string' ? json.initiate_login_uri : null,
    };
  } catch {
    return null;
  }
}

/**
 * The registered sign-in start, if it is an absolute `https` URL (or loopback `http`, as Sesame's registry allows for
 * development). Anything else (relative, `javascript:`, malformed) is ignored.
 */
export function restartTarget(uri: string | null | undefined): string | null {
  if (!uri) return null;
  let url: URL;
  try {
    url = new URL(uri);
  } catch {
    return null;
  }
  const loopback = url.hostname === 'localhost' || url.hostname === '127.0.0.1' || url.hostname === '[::1]';
  if (url.protocol === 'https:' || (url.protocol === 'http:' && loopback)) return url.toString();
  return null;
}

/** Re-check one second after the request is due to expire (never sooner than 1 s, never later than 30 min). */
export function recheckDelayMs(expiresIn: number): number {
  const seconds = Math.min(Math.max(expiresIn, 0), 1800) + 1;
  return seconds * 1000;
}

export function decide(status: AuthorizeStatus | null): RequestCheck {
  if (!status) return { kind: 'unknown' };
  if (status.active) return { kind: 'ok', recheckInMs: recheckDelayMs(status.expires_in) };
  const to = restartTarget(status.initiate_login_uri);
  return to ? { kind: 'restart', to } : { kind: 'expired' };
}
