import { createSignal, onCleanup, onMount, Show } from 'solid-js';
import { Button, Card, Field } from '@sesame/shared';
import {
  AuthError,
  isChallenge,
  login,
  passkeyChallenge,
  passkeyVerify,
  sendEmailOtp,
  sendMagicLink,
  verifyEmailOtp,
} from '../lib/api';
import type { MfaChallenge, SignInResult, TokenResponse } from '../lib/api';
import { SecondStep } from '../components/SecondStep';
import {
  assertionJSON,
  cancelled,
  conditionalAvailable,
  passkeysSupported,
  requestOptions,
} from '../lib/webauthn';

type Step = 'identify' | 'password' | 'otp-sent' | 'link-sent';

export interface SignInProps {
  tenantId: string;
  /** OIDC / login client_id (required by auth_login client registry). */
  clientId: string;
  tenantName?: string;
  onAuthenticated: (tokens: TokenResponse) => void;
  /** A sign-in that already needs its second step (a magic link whose account has a second factor). */
  initialChallenge?: MfaChallenge;
  /** Heading override (the security page asks the user to sign in again). */
  title?: string;
}

/**
 * Hosted sign-in (ADR-010 §2.2): the canned page tenants redirect TO.
 *
 * Method-agnostic identifier-first flow: collect the identifier, then offer
 * password / email OTP / magic link — or skip the identifier altogether with a
 * passkey (ADR-008): a button, and the browser's passkey autofill on the email
 * field where it supports conditional mediation. Any first factor may answer
 * with a second step (202): the account has a second factor, or its
 * organisation requires one ([`SecondStep`]).
 *
 * A3 contract: OTP and magic-link sends return a generic success whether or
 * not the account exists, so this UI advances to the "check your…" step
 * unconditionally. Never branch on existence — that would rebuild the
 * enumeration oracle the backend deliberately removed.
 */
export function SignIn(props: SignInProps) {
  const [step, setStep] = createSignal<Step>('identify');
  const [email, setEmail] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [code, setCode] = createSignal('');
  const [error, setError] = createSignal('');
  const [busy, setBusy] = createSignal(false);
  const [challenge, setChallenge] = createSignal<MfaChallenge | undefined>(props.initialChallenge);

  /** Tokens, or the second step. */
  const handle = (r: SignInResult) => {
    if (isChallenge(r)) {
      setChallenge(r);
      return;
    }
    props.onAuthenticated(r);
  };

  // Passkeys: one pending navigator.credentials.get() at a time, so the autofill request is aborted before a button
  // press starts a modal one.
  let autofill: AbortController | undefined;
  const stopAutofill = () => {
    autofill?.abort();
    autofill = undefined;
  };
  const passkeySignIn = async (conditional: boolean) => {
    stopAutofill();
    const controller = conditional ? new AbortController() : undefined;
    if (controller) autofill = controller;
    try {
      const { challenge_id, options } = await passkeyChallenge(props.tenantId, { clientId: props.clientId });
      const cred = await navigator.credentials.get(requestOptions(options, conditional, controller?.signal));
      if (!cred) return;
      if (!conditional) setBusy(true);
      handle(
        await passkeyVerify(
          props.tenantId,
          challenge_id,
          assertionJSON(cred as unknown as Parameters<typeof assertionJSON>[0]),
        ),
      );
    } catch (e) {
      if (cancelled(e)) return;
      setError(e instanceof AuthError ? e.message : 'That passkey could not be used. Try again or use your email.');
    } finally {
      if (!conditional) setBusy(false);
    }
  };
  onMount(async () => {
    if (!props.initialChallenge && (await conditionalAvailable())) void passkeySignIn(true);
  });
  onCleanup(stopAutofill);

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError('');
    try {
      await fn();
    } catch (e) {
      setError(e instanceof AuthError ? e.message : 'Something went wrong. Please try again.');
    } finally {
      setBusy(false);
    }
  };

  const heading = () => props.title ?? (props.tenantName ? `Sign in to ${props.tenantName}` : 'Sign in');

  return (
    <Show
      when={!challenge()}
      fallback={
        <SecondStep
          tenantId={props.tenantId}
          challenge={challenge()!}
          onAuthenticated={(t) => props.onAuthenticated(t)}
          onStartOver={(message) => {
            setChallenge(undefined);
            setPassword('');
            setCode('');
            setStep('identify');
            setError(message);
          }}
        />
      }
    >
    <Card title={heading()} subtitle="Use your email to continue.">
      <Show when={error()}>
        <div class="mb-4 rounded-lg bg-error-50 px-4 py-3 text-theme-sm text-error-700" role="alert">
          {error()}
        </div>
      </Show>

      <Show when={step() === 'identify'}>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            setStep('password');
          }}
        >
          <Field
            label="Email"
            type="email"
            autocomplete="username webauthn"
            required
            value={email()}
            onInput={(e) => setEmail(e.currentTarget.value)}
          />
          <Button type="submit" full disabled={!email()}>
            Continue
          </Button>
        </form>
        <Show when={passkeysSupported()}>
          <Button variant="secondary" full class="mt-2" loading={busy()} onClick={() => void passkeySignIn(false)}>
            Sign in with a passkey
          </Button>
        </Show>
        <div class="mt-4 flex flex-col gap-2">
          <Button
            variant="secondary"
            full
            loading={busy()}
            disabled={!email()}
            onClick={() => run(async () => {
              await sendEmailOtp(props.tenantId, email());
              setStep('otp-sent'); // unconditional — generic response by design
            })}
          >
            Email me a code
          </Button>
          <Button
            variant="ghost"
            full
            loading={busy()}
            disabled={!email()}
            onClick={() => run(async () => {
              await sendMagicLink(props.tenantId, email());
              setStep('link-sent');
            })}
          >
            Email me a sign-in link
          </Button>
        </div>
      </Show>

      <Show when={step() === 'password'}>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(async () => {
              stopAutofill();
              handle(await login(props.tenantId, email(), password(), props.clientId));
            });
          }}
        >
          <Field label="Email" type="email" value={email()} disabled />
          <Field
            label="Password"
            type="password"
            autocomplete="current-password"
            required
            value={password()}
            onInput={(e) => setPassword(e.currentTarget.value)}
          />
          <Button type="submit" full loading={busy()}>
            Sign in
          </Button>
        </form>
        <Button variant="ghost" full class="mt-2" onClick={() => setStep('identify')}>
          Use another method
        </Button>
        <a href={`/forgot-password?tenant=${props.tenantId}`} class="mt-3 block text-center text-theme-sm text-gray-500 underline hover:text-gray-700">
          Forgot your password?
        </a>
      </Show>

      <Show when={step() === 'otp-sent'}>
        <p class="mb-4 text-theme-sm text-gray-600 dark:text-gray-300">
          If an account exists for {email()}, we've sent a 6-digit code. It expires in 5 minutes.
        </p>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(async () => {
              stopAutofill();
              handle(await verifyEmailOtp(props.tenantId, email(), code()));
            });
          }}
        >
          <Field
            label="Verification code"
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength={6}
            required
            value={code()}
            onInput={(e) => setCode(e.currentTarget.value)}
          />
          <Button type="submit" full loading={busy()} disabled={code().length !== 6}>
            Verify and continue
          </Button>
        </form>
        <Button variant="ghost" full class="mt-2" onClick={() => setStep('identify')}>
          Start over
        </Button>
      </Show>

      <Show when={step() === 'link-sent'}>
        <p class="text-theme-sm text-gray-600 dark:text-gray-300">
          If an account exists for {email()}, we've sent a sign-in link. It expires in 10 minutes and can be used once.
        </p>
        <Button variant="ghost" full class="mt-4" onClick={() => setStep('identify')}>
          Start over
        </Button>
      </Show>
    </Card>
    </Show>
  );
}
