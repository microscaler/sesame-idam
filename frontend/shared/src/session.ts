/**
 * Console session bootstrap — the dogfood seam (ADR-010).
 *
 * Both consoles authenticate THROUGH the hosted auth surface using the same
 * `@sesame/idam-client` an external tenant would use. If this path is awkward
 * for us, it is awkward for them — which is exactly the point of dogfooding it
 * rather than giving the consoles a private login.
 */

import { createClient, type SesameClient, type Session } from '@sesame/idam-client';

export interface ConsoleAuthConfig {
  /** Hosted auth surface origin (VITE_AUTH_BASE_URL). */
  authBaseUrl: string;
  /** Tenant slug this console operates as. */
  tenantId: string;
}

export function createConsoleClient(config: ConsoleAuthConfig): SesameClient {
  return createClient({
    authBaseUrl: config.authBaseUrl,
    tenantId: config.tenantId,
    redirectUri: `${window.location.origin}/callback`,
  });
}

/**
 * Resolve the current session, completing a redirect callback when we've just
 * come back from the hosted surface. Returns `null` when signed out — callers
 * then invoke `client.login()`.
 */
export async function bootstrapSession(client: SesameClient): Promise<Session | null> {
  if (window.location.pathname === '/callback' && window.location.search.includes('code=')) {
    const session = await client.handleCallback();
    // Strip the code/state from the address bar.
    window.history.replaceState({}, '', '/');
    return session;
  }
  return client.getSession();
}
