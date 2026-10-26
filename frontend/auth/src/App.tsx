import { createSignal, onCleanup, onMount, Show } from 'solid-js';
import { applyTenantTheme, runtimeConfig } from '@sesame/shared';
import { SignIn } from './pages/SignIn';
import { ResetPassword } from './pages/ResetPassword';
import { Security } from './pages/Security';
import { AuthError, completeOidcAuthorize, isChallenge, mintSessionCode, verifyMagicLink } from './lib/api';
import type { MfaChallenge, TokenResponse } from './lib/api';
import { decide, fetchAuthorizeStatus } from './lib/oidcRequest';

/**
 * Hosted auth surface shell (ADR-010 + OIDC authorize/complete).
 *
 * Routes (query-driven):
 *   /authorize?request_id=&tenant=&client_id=     → OIDC hosted login
 *   /authorize?tenant=&redirect_uri=&state=       → ADR-010 session-code handoff
 *   /verify-magic?tenant=&token=&state=           → magic-link "click"
 *   /account/security?tenant=&client_id=          → sign-in security (passkeys, authenticator app, recovery codes)
 *
 * OIDC path: after password/OTP, POST /oauth/authorize/complete and follow the
 * 302 to the registered RP redirect with `code` + `state`.
 * ADR-010 path: mint session code and return to `redirect_uri`.
 */
/**
 * What a failed handoff means for the user. An OIDC sign-in request lives for 30 minutes and is used once. When the
 * application registered a sign-in start (`initiate_login_uri`) the page restarts there instead (lib/oidcRequest.ts);
 * this message is for applications that did not.
 */
function handoffFailure(e: unknown): string {
  const code = e instanceof AuthError ? e.code : '';
  if (code === 'invalid_request') {
    return 'Signed in, but this sign-in request has expired or was already used. Go back to the application and sign in again.';
  }
  if (code === 'temporarily_unavailable') {
    return 'Signed in, but the sign-in service is briefly unavailable. Go back to the application and try again in a moment.';
  }
  return 'Signed in, but the handoff to the application failed.';
}

export function App() {
  const params = new URLSearchParams(window.location.search);
  const requestId = params.get('request_id') ?? '';
  // The request names its tenant and client; a deployment may set defaults in /config.json. Never a built-in one.
  const tenantId = params.get('tenant') ?? runtimeConfig().defaultTenant ?? '';
  const clientId = params.get('client_id') ?? runtimeConfig().defaultClientId ?? '';
  const redirectUri = params.get('redirect_uri') ?? '';
  const state = params.get('state') ?? '';
  const magicToken = params.get('token');
  const path = window.location.pathname;
  const isReset = path.includes('reset-password') || path.includes('forgot-password');
  const isSecurity = path.includes('account/security');
  const isOidc = requestId.length > 0;
  // A magic link whose account has a second factor: the sign-in continues with its second step (ADR-008).
  const [magicChallenge, setMagicChallenge] = createSignal<MfaChallenge | undefined>();

  const [status, setStatus] = createSignal<'ready' | 'verifying' | 'error'>(
    !tenantId || (!isReset && !clientId)
      ? 'error'
      : path.includes('verify-magic') && magicToken
        ? 'verifying'
        : 'ready',
  );
  const [error, setError] = createSignal(
    !tenantId || (!isReset && !clientId)
      ? 'This sign-in link is incomplete: it does not name the application. Start again from the application.'
      : '',
  );

  // OIDC: never leave the user on a request that can no longer complete (lib/oidcRequest.ts). Checked on load, when the
  // tab is shown again, and when the request is due to expire; an expired one goes straight to the application's
  // registered sign-in start, which begins a fresh request and comes back here.
  let leaving = false;
  let recheckTimer: number | undefined;
  let lastCheck = 0;
  const restartAt = (to: string) => {
    leaving = true;
    window.clearTimeout(recheckTimer);
    window.location.replace(to);
  };
  const checkRequest = async (force = false): Promise<void> => {
    if (!isOidc || leaving || status() !== 'ready') return;
    const now = Date.now();
    if (!force && now - lastCheck < 5_000) return;
    lastCheck = now;
    const check = decide(await fetchAuthorizeStatus(requestId, clientId));
    if (leaving || status() !== 'ready') return;
    if (check.kind === 'restart') {
      restartAt(check.to);
    } else if (check.kind === 'expired') {
      setError('This sign-in request has expired. Go back to the application and sign in again.');
      setStatus('error');
    } else if (check.kind === 'ok') {
      window.clearTimeout(recheckTimer);
      recheckTimer = window.setTimeout(() => void checkRequest(true), check.recheckInMs);
    }
  };
  const onShown = () => {
    if (document.visibilityState === 'visible') void checkRequest();
  };

  onMount(async () => {
    applyTenantTheme({ displayName: tenantId });

    if (isOidc) {
      document.addEventListener('visibilitychange', onShown);
      window.addEventListener('focus', onShown);
      window.addEventListener('pageshow', onShown);
      onCleanup(() => {
        document.removeEventListener('visibilitychange', onShown);
        window.removeEventListener('focus', onShown);
        window.removeEventListener('pageshow', onShown);
        window.clearTimeout(recheckTimer);
      });
      void checkRequest(true);
    }

    if (status() === 'verifying' && magicToken) {
      try {
        const result = await verifyMagicLink(tenantId, magicToken);
        if (isChallenge(result)) {
          setMagicChallenge(result);
          setStatus('ready');
          return;
        }
        void complete(result);
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
    } catch (e) {
      // The request expired or was used while the user signed in: restart at the application when it registered a
      // sign-in start, rather than ending on a message.
      if (isOidc && e instanceof AuthError && e.code === 'invalid_request') {
        const check = decide(await fetchAuthorizeStatus(requestId, clientId));
        if (check.kind === 'restart') {
          restartAt(check.to);
          return;
        }
      }
      setError(handoffFailure(e));
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
      <Show when={status() === 'ready' && isSecurity}>
        <Security tenantId={tenantId} clientId={clientId} />
      </Show>
      <Show when={status() === 'ready' && !isReset && !isSecurity}>
        <SignIn
          tenantId={tenantId}
          clientId={clientId}
          tenantName={tenantId}
          initialChallenge={magicChallenge()}
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
