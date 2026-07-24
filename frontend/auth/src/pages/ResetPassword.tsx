import { createSignal, Show } from 'solid-js';
import { Button, Card, Field } from '@sesame/shared';
import { AuthError, forgotPassword, resetPassword } from '../lib/api';

/**
 * Hosted password-reset pages (ADR-010).
 *
 * Two modes on one component:
 *  - REQUEST  (/forgot-password): ask for the email → generic confirmation.
 *  - CONSUME  (/reset-password?token=…): set a new password.
 *
 * Contract notes mirrored from the backend:
 *  - The request response is generic whether or not the account exists, so
 *    this UI always advances to the confirmation step.
 *  - A weak password is rejected WITHOUT burning the token, so the user stays
 *    on this form and can retry with the same link.
 *  - Success does not sign the user in — we send them to sign-in, which
 *    exercises the new password.
 */
export function ResetPassword(props: { tenantId: string; token?: string }) {
  const [email, setEmail] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [confirm, setConfirm] = createSignal('');
  const [sent, setSent] = createSignal(false);
  const [done, setDone] = createSignal(false);
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

  const mismatch = () => confirm().length > 0 && password() !== confirm();

  return (
    <>
      <Show when={error()}>
        <div class="mb-4 w-full max-w-md rounded-lg bg-error-50 px-4 py-3 text-theme-sm text-error-700" role="alert">
          {error()}
        </div>
      </Show>

      {/* ── CONSUME: we have a token ── */}
      <Show when={props.token}>
        <Show
          when={!done()}
          fallback={
            <Card title="Password updated" subtitle="You can now sign in with your new password.">
              <a href={`/authorize?tenant=${props.tenantId}`}>
                <Button full>Go to sign in</Button>
              </a>
            </Card>
          }
        >
          <Card title="Choose a new password" subtitle="Your reset link is valid for 15 minutes.">
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void run(async () => {
                  await resetPassword(props.tenantId, props.token!, password());
                  setDone(true);
                });
              }}
            >
              <Field
                label="New password"
                type="password"
                autocomplete="new-password"
                required
                value={password()}
                onInput={(e) => setPassword(e.currentTarget.value)}
                hint="At least 12 characters, with upper, lower, number and symbol."
              />
              <Field
                label="Confirm new password"
                type="password"
                autocomplete="new-password"
                required
                value={confirm()}
                onInput={(e) => setConfirm(e.currentTarget.value)}
                error={mismatch() ? 'Passwords do not match' : undefined}
              />
              <Button type="submit" full loading={busy()} disabled={!password() || mismatch()}>
                Update password
              </Button>
            </form>
          </Card>
        </Show>
      </Show>

      {/* ── REQUEST: no token ── */}
      <Show when={!props.token}>
        <Show
          when={!sent()}
          fallback={
            <Card title="Check your email" subtitle="If an account exists, we've sent a reset link. It expires in 15 minutes.">
              <a href={`/authorize?tenant=${props.tenantId}`}>
                <Button variant="secondary" full>
                  Back to sign in
                </Button>
              </a>
            </Card>
          }
        >
          <Card title="Reset your password" subtitle="We'll email you a link to set a new one.">
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void run(async () => {
                  await forgotPassword(props.tenantId, email());
                  setSent(true); // unconditional — generic response by design
                });
              }}
            >
              <Field
                label="Email"
                type="email"
                autocomplete="username"
                required
                value={email()}
                onInput={(e) => setEmail(e.currentTarget.value)}
              />
              <Button type="submit" full loading={busy()} disabled={!email()}>
                Send reset link
              </Button>
            </form>
            <a href={`/authorize?tenant=${props.tenantId}`}>
              <Button variant="ghost" full class="mt-2">
                Back to sign in
              </Button>
            </a>
          </Card>
        </Show>
      </Show>
    </>
  );
}
