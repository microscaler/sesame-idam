import { createSignal, Show } from 'solid-js';
import { Button, Card, Field } from '@sesame/shared';
import { AuthError, login, sendEmailOtp, sendMagicLink, verifyEmailOtp } from '../lib/api';
import type { TokenResponse } from '../lib/api';

type Step = 'identify' | 'password' | 'otp-sent' | 'link-sent';

export interface SignInProps {
  tenantId: string;
  tenantName?: string;
  onAuthenticated: (tokens: TokenResponse) => void;
}

/**
 * Hosted sign-in (ADR-010 §2.2): the canned page tenants redirect TO.
 *
 * Method-agnostic identifier-first flow: collect the identifier, then offer
 * password / email OTP / magic link. Passkeys slot in here as the preferred
 * method when ADR-008 lands.
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

  const heading = () => (props.tenantName ? `Sign in to ${props.tenantName}` : 'Sign in');

  return (
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
              const tokens = await login(props.tenantId, email(), password());
              props.onAuthenticated(tokens);
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
      </Show>

      <Show when={step() === 'otp-sent'}>
        <p class="mb-4 text-theme-sm text-gray-600 dark:text-gray-300">
          If an account exists for {email()}, we've sent a 6-digit code. It expires in 5 minutes.
        </p>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void run(async () => {
              const tokens = await verifyEmailOtp(props.tenantId, email(), code());
              props.onAuthenticated(tokens);
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
  );
}
