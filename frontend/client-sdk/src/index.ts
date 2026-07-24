/**
 * @sesame/idam-client — the tenant-facing integration surface (ADR-010 §2.3).
 *
 * VENDORED: private workspace package, not published to npm until GA.
 *
 * What this is: redirect + session handling. A tenant's /signin becomes a
 * branded button that calls `login()`; the credential ceremony (password,
 * OTP, magic link, passkey) happens on the HOSTED auth surface — never in the
 * tenant's origin. That is what makes ADR-008 passkeys work (WebAuthn is
 * origin-bound) and delivers the README's "zero auth logic in your app".
 *
 * Typical tenant usage:
 *
 *   const sesame = createClient({
 *     authBaseUrl: 'https://login.tenant.com',   // ADR-007 verified domain
 *     tenantId: 'hauliage',
 *     redirectUri: 'https://app.tenant.com/callback',
 *   });
 *
 *   // /signin
 *   <button onClick={() => sesame.login()}>Sign in</button>
 *
 *   // /callback
 *   const session = await sesame.handleCallback();
 *
 *   // anywhere
 *   const session = await sesame.getSession();   // silent refresh if needed
 */

export interface SesameClientOptions {
  /** Hosted auth surface origin (Sesame's, or the tenant's ADR-007 domain). */
  authBaseUrl: string;
  /** Tenant slug — sent as X-Tenant-ID / carried through the redirect. */
  tenantId: string;
  /** Where the hosted surface returns the user after authentication. */
  redirectUri: string;
  /** Storage for session material. Default: sessionStorage. */
  storage?: Storage;
}

export interface Session {
  accessToken: string;
  refreshToken?: string;
  expiresAt: number; // epoch seconds
  userId: string;
  tenantId: string;
  roles: string[];
}

/** Authentication method hint for the hosted surface's initial screen. */
export type LoginHint = 'password' | 'email-otp' | 'magic-link' | 'passkey' | 'phone-otp';

export interface LoginOptions {
  /** Pre-select a method on the hosted surface. */
  method?: LoginHint;
  /** Opaque value round-tripped through the redirect (CSRF + return-to). */
  state?: string;
  /** Prefill the identifier field. */
  loginHint?: string;
}

const STORAGE_KEY = 'sesame.session';
const STATE_KEY = 'sesame.state';

export class SesameClient {
  private readonly opts: Required<Pick<SesameClientOptions, 'authBaseUrl' | 'tenantId' | 'redirectUri'>> &
    SesameClientOptions;
  private readonly storage: Storage;

  constructor(options: SesameClientOptions) {
    this.opts = options as SesameClient['opts'];
    this.storage =
      options.storage ?? (typeof sessionStorage !== 'undefined' ? sessionStorage : (undefined as unknown as Storage));
  }

  /**
   * Redirect to the hosted auth surface. Returns never (navigates away).
   * A random `state` is generated when not supplied and verified on return.
   */
  login(options: LoginOptions = {}): void {
    const state = options.state ?? crypto.randomUUID();
    this.storage?.setItem(STATE_KEY, state);

    const url = new URL('/authorize', this.opts.authBaseUrl);
    url.searchParams.set('tenant', this.opts.tenantId);
    url.searchParams.set('redirect_uri', this.opts.redirectUri);
    url.searchParams.set('state', state);
    if (options.method) url.searchParams.set('method', options.method);
    if (options.loginHint) url.searchParams.set('login_hint', options.loginHint);

    window.location.assign(url.toString());
  }

  /**
   * Complete the round trip on the tenant's callback route: verifies `state`,
   * exchanges the one-time code for a session, stores it.
   *
   * @throws when `state` does not match (possible CSRF) or exchange fails.
   */
  async handleCallback(search: string = window.location.search): Promise<Session> {
    const params = new URLSearchParams(search);
    const code = params.get('code');
    const state = params.get('state');
    const expected = this.storage?.getItem(STATE_KEY);
    this.storage?.removeItem(STATE_KEY);

    if (!code) throw new Error('sesame: callback missing code');
    if (!state || !expected || state !== expected) {
      throw new Error('sesame: state mismatch — possible CSRF, authentication rejected');
    }

    const res = await fetch(new URL('/session/exchange', this.opts.authBaseUrl).toString(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Tenant-ID': this.opts.tenantId },
      body: JSON.stringify({ code, redirect_uri: this.opts.redirectUri }),
      credentials: 'include',
    });
    if (!res.ok) throw new Error(`sesame: session exchange failed (${res.status})`);

    const session = normalizeSession(await res.json(), this.opts.tenantId);
    this.persist(session);
    return session;
  }

  /** Current session, silently refreshed when near expiry. `null` when signed out. */
  async getSession(): Promise<Session | null> {
    const raw = this.storage?.getItem(STORAGE_KEY);
    if (!raw) return null;
    let session: Session;
    try {
      session = JSON.parse(raw) as Session;
    } catch {
      return null;
    }
    // Refresh 60s before expiry.
    if (session.expiresAt - 60 > Math.floor(Date.now() / 1000)) return session;
    return this.refresh(session);
  }

  /** Exchange the refresh token for a new access token. */
  async refresh(session: Session): Promise<Session | null> {
    if (!session.refreshToken) return null;
    const res = await fetch(new URL('/session/refresh', this.opts.authBaseUrl).toString(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Tenant-ID': this.opts.tenantId },
      body: JSON.stringify({ refresh_token: session.refreshToken }),
      credentials: 'include',
    });
    if (!res.ok) {
      this.clear();
      return null;
    }
    const next = normalizeSession(await res.json(), this.opts.tenantId);
    this.persist(next);
    return next;
  }

  /** Clear local session and redirect to the hosted surface's logout. */
  logout(returnTo?: string): void {
    this.clear();
    const url = new URL('/logout', this.opts.authBaseUrl);
    url.searchParams.set('tenant', this.opts.tenantId);
    url.searchParams.set('return_to', returnTo ?? this.opts.redirectUri);
    window.location.assign(url.toString());
  }

  private persist(session: Session): void {
    this.storage?.setItem(STORAGE_KEY, JSON.stringify(session));
  }

  private clear(): void {
    this.storage?.removeItem(STORAGE_KEY);
    this.storage?.removeItem(STATE_KEY);
  }
}

/** Map the login-service token response onto a Session. */
function normalizeSession(body: Record<string, unknown>, tenantId: string): Session {
  const accessToken = String(body.access_token ?? '');
  const expiresIn = Number(body.expires_in ?? 300);
  return {
    accessToken,
    refreshToken: body.refresh_token ? String(body.refresh_token) : undefined,
    expiresAt: Math.floor(Date.now() / 1000) + expiresIn,
    userId: String(body.user_id ?? ''),
    tenantId,
    roles: Array.isArray(body.roles) ? (body.roles as string[]) : [],
  };
}

export function createClient(options: SesameClientOptions): SesameClient {
  return new SesameClient(options);
}
