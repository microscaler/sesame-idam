import { createSignal, onMount, Show } from 'solid-js';
import { applyTenantTheme } from '@sesame/shared';
import { SignIn } from './pages/SignIn';
import { ResetPassword } from './pages/ResetPassword';
import { mintSessionCode, verifyMagicLink } from './lib/api';
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
 * On success the surface mints a ONE-TIME CODE (POST /auth/session/code) and
 * returns the caller to `redirect_uri` with that code plus the original
 * `state`. @sesame/idam-client redeems it at /auth/token
 * (grant_type=authorization_code). Tokens are never placed in a URL.
 */
export function App() {
  const params = new URLSearchParams(window.location.search);
  const tenantId = params.get('tenant') ?? import.meta.env.VITE_DEFAULT_TENANT ?? 'hauliage';
  const redirectUri = params.get('redirect_uri') ?? '';
  const state = params.get('state') ?? '';
  const magicToken = params.get('token');
  const path = window.location.pathname;
  const isReset = path.includes('reset-password') || path.includes('forgot-password');

  const [status, setStatus] = createSignal<'ready' | 'verifying' | 'error'>(
    path.includes('verify-magic') && magicToken ? 'verifying' : 'ready',
  );
  const [error, setError] = createSignal('');

  onMount(async () => {
    // Per-tenant branding (ADR-009 config → runtime CSS vars; ADR-007 domain).
    applyTenantTheme({ displayName: tenantId });

    if (status() === 'verifying' && magicToken) {
      try {
        const tokens = await verifyMagicLink(tenantId, magicToken);
        void complete(tokens);
      } catch {
        setError('This sign-in link is invalid, expired, or already used.');
        setStatus('error');
      }
    }
  });

  const complete = async (tokens: TokenResponse) => {
    if (!redirectUri) {
      // No caller (direct visit) — nothing to redirect to.
      setError('Signed in, but no return destination was supplied.');
      setStatus('error');
      return;
    }
    try {
      const { code } = await mintSessionCode(
        tenantId,
        tokens.access_token,
        tokens.refresh_token,
        redirectUri,
      );
      const url = new URL(redirectUri);
      url.searchParams.set('code', code);
      if (state) url.searchParams.set('state', state);
      window.location.assign(url.toString());
    } catch {
      setError('Signed in, but the handoff to the application failed.');
      setStatus('error');
    }
  };

  return (
    <main class="flex min-h-screen items-center justify-center bg-gray-50 px-4 dark:bg-gray-950">
      <Show when={status() === 'ready' && isReset}>
        <ResetPassword tenantId={tenantId} token={path.includes('reset-password') ? (magicToken ?? undefined) : undefined} />
      </Show>
      <Show when={status() === 'ready' && !isReset}>
        <SignIn tenantId={tenantId} tenantName={tenantId} onAuthenticated={(t) => void complete(t)} />
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
