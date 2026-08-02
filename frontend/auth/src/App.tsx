import { createSignal, onMount, Show } from 'solid-js';
import { applyTenantTheme } from '@sesame/shared';
import { SignIn } from './pages/SignIn';
import { ResetPassword } from './pages/ResetPassword';
import { completeOidcAuthorize, mintSessionCode, verifyMagicLink } from './lib/api';
import type { TokenResponse } from './lib/api';

/**
 * Hosted auth surface shell (ADR-010 + OIDC authorize/complete).
 *
 * Routes (query-driven):
 *   /authorize?request_id=&tenant=&client_id=     → OIDC hosted login
 *   /authorize?tenant=&redirect_uri=&state=       → ADR-010 session-code handoff
 *   /verify-magic?tenant=&token=&state=           → magic-link "click"
 *
 * OIDC path: after password/OTP, POST /oauth/authorize/complete and follow the
 * 302 to the registered RP redirect with `code` + `state`.
 * ADR-010 path: mint session code and return to `redirect_uri`.
 */
export function App() {
  const params = new URLSearchParams(window.location.search);
  const requestId = params.get('request_id') ?? '';
  const tenantId =
    params.get('tenant') ?? import.meta.env.VITE_DEFAULT_TENANT ?? 'hauliage';
  const clientId =
    params.get('client_id') ?? import.meta.env.VITE_DEFAULT_CLIENT_ID ?? 'hauliage-web';
  const redirectUri = params.get('redirect_uri') ?? '';
  const state = params.get('state') ?? '';
  const magicToken = params.get('token');
  const path = window.location.pathname;
  const isReset = path.includes('reset-password') || path.includes('forgot-password');
  const isOidc = requestId.length > 0;

  const [status, setStatus] = createSignal<'ready' | 'verifying' | 'error'>(
    path.includes('verify-magic') && magicToken ? 'verifying' : 'ready',
  );
  const [error, setError] = createSignal('');

  onMount(async () => {
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
    try {
      if (isOidc) {
        const { redirect } = await completeOidcAuthorize(
          tenantId,
          tokens.access_token,
          requestId,
        );
        if (!redirect) {
          throw new Error('missing redirect');
        }
        window.location.assign(redirect);
        return;
      }

      if (!redirectUri) {
        setError('Signed in, but no return destination was supplied.');
        setStatus('error');
        return;
      }
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
        <ResetPassword
          tenantId={tenantId}
          token={path.includes('reset-password') ? (magicToken ?? undefined) : undefined}
        />
      </Show>
      <Show when={status() === 'ready' && !isReset}>
        <SignIn
          tenantId={tenantId}
          clientId={clientId}
          tenantName={tenantId}
          onAuthenticated={(t) => void complete(t)}
        />
      </Show>
      <Show when={status() === 'verifying'}>
        <p class="text-theme-sm text-gray-600 dark:text-gray-300">Signing you in…</p>
      </Show>
      <Show when={status() === 'error'}>
        <div class="max-w-md rounded-2xl border border-gray-200 bg-white p-8 text-center shadow-theme-lg dark:border-gray-700 dark:bg-gray-900">
          <p class="text-theme-sm text-error-600">{error()}</p>
          <a
            href={
              isOidc
                ? `/authorize?request_id=${encodeURIComponent(requestId)}&tenant=${encodeURIComponent(tenantId)}&client_id=${encodeURIComponent(clientId)}`
                : `/authorize?tenant=${tenantId}`
            }
            class="mt-4 inline-block text-theme-sm text-brand-primary underline"
          >
            Back to sign in
          </a>
        </div>
      </Show>
    </main>
  );
}
