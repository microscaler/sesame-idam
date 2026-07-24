import { createSignal, onMount, Show } from 'solid-js';
import { applyTenantTheme } from '@sesame/shared';
import { SignIn } from './pages/SignIn';
import { verifyMagicLink } from './lib/api';
import type { TokenResponse } from './lib/api';

/**
 * Hosted auth surface shell (ADR-010).
 *
 * Routes (query-driven, no router dependency yet):
 *   /authorize?tenant=&redirect_uri=&state=&method=   → sign-in
 *   /verify-magic?tenant=&token=&state=               → magic-link "click"
 *
 * On success the surface returns the caller to `redirect_uri` with a
 * one-time `code` + the original `state`, which @sesame/idam-client exchanges
 * for a session. Tokens are never handed to the tenant app via the URL.
 *
 * NOTE (scaffold): the code-exchange endpoint (/session/exchange) is the
 * remaining backend piece — until it exists, DEV mode round-trips the tokens
 * through sessionStorage so the flow is exercisable end to end.
 */
export function App() {
  const params = new URLSearchParams(window.location.search);
  const tenantId = params.get('tenant') ?? import.meta.env.VITE_DEFAULT_TENANT ?? 'hauliage';
  const redirectUri = params.get('redirect_uri') ?? '';
  const state = params.get('state') ?? '';
  const magicToken = params.get('token');

  const [status, setStatus] = createSignal<'ready' | 'verifying' | 'error'>(
    window.location.pathname.includes('verify-magic') && magicToken ? 'verifying' : 'ready',
  );
  const [error, setError] = createSignal('');

  onMount(async () => {
    // Per-tenant branding (ADR-009 config → runtime CSS vars; ADR-007 domain).
    applyTenantTheme({ displayName: tenantId });

    if (status() === 'verifying' && magicToken) {
      try {
        const tokens = await verifyMagicLink(tenantId, magicToken);
        complete(tokens);
      } catch {
        setError('This sign-in link is invalid, expired, or already used.');
        setStatus('error');
      }
    }
  });

  const complete = (tokens: TokenResponse) => {
    if (!redirectUri) {
      // No caller (direct visit) — nothing to redirect to.
      setError('Signed in, but no return destination was supplied.');
      setStatus('error');
      return;
    }
    // DEV round-trip until /session/exchange lands (see module note).
    sessionStorage.setItem('sesame.dev.tokens', JSON.stringify(tokens));
    const url = new URL(redirectUri);
    url.searchParams.set('code', 'dev-' + tokens.user_id);
    if (state) url.searchParams.set('state', state);
    window.location.assign(url.toString());
  };

  return (
    <main class="flex min-h-screen items-center justify-center bg-gray-50 px-4 dark:bg-gray-950">
      <Show when={status() === 'ready'}>
        <SignIn tenantId={tenantId} tenantName={tenantId} onAuthenticated={complete} />
      </Show>
      <Show when={status() === 'verifying'}>
        <p class="text-theme-sm text-gray-600 dark:text-gray-300">Signing you in…</p>
      </Show>
      <Show when={status() === 'error'}>
        <div class="max-w-md rounded-2xl border border-gray-200 bg-white p-8 text-center shadow-theme-lg dark:border-gray-700 dark:bg-gray-900">
          <p class="text-theme-sm text-error-600">{error()}</p>
          <a href={`/authorize?tenant=${tenantId}`} class="mt-4 inline-block text-theme-sm text-brand-primary underline">
            Back to sign in
          </a>
        </div>
      </Show>
    </main>
  );
}
