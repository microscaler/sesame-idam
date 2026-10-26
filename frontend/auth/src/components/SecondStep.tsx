import { createSignal, Match, Show, Switch } from 'solid-js';
import { Button, Card, Field } from '@sesame/shared';
import { AuthError, isChallenge, mfaVerify, passkeyChallenge, passkeyVerify } from '../lib/api';
import type { Enrolled, MfaChallenge, SignInResult, TokenResponse } from '../lib/api';
import { assertionJSON, cancelled, passkeysSupported, requestOptions } from '../lib/webauthn';
import { Enroll } from './Enroll';
import { RecoveryCodes } from './RecoveryCodes';

export interface SecondStepProps {
  tenantId: string;
  challenge: MfaChallenge;
  onAuthenticated: (tokens: TokenResponse) => void;
  /** The sign-in step expired or was used up: back to the first step. */
  onStartOver: (message: string) => void;
}

/**
 * The second step of a sign-in (ADR-008): a passkey, an authenticator-app code or a recovery code; or, when the
 * account's organisation requires a method the account lacks, enrolling one (then the account's first recovery codes,
 * then in). No token exists until this completes.
 */
export function SecondStep(props: SecondStepProps) {
  const [challenge, setChallenge] = createSignal<MfaChallenge>(props.challenge);
  const [useRecovery, setUseRecovery] = createSignal(
    !props.challenge.methods.includes('totp') && !props.challenge.methods.includes('passkey'),
  );
  const [code, setCode] = createSignal('');
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal('');
  const [codes, setCodes] = createSignal<{ codes: string[]; tokens: TokenResponse } | null>(null);

  const has = (m: 'totp' | 'passkey' | 'recovery_code') => challenge().methods.includes(m);

  const next = (r: SignInResult) => {
    if (isChallenge(r)) {
      setChallenge(r);
      setCode('');
      return;
    }
    props.onAuthenticated(r);
  };
  const fail = (e: unknown) => {
    if (e instanceof AuthError && e.code === 'invalid_token') {
      props.onStartOver('That sign-in step expired. Sign in again.');
      return;
    }
    setError(e instanceof AuthError ? e.message : 'Something went wrong. Please try again.');
  };

  const verifyCode = async () => {
    setBusy(true);
    setError('');
    try {
      next(await mfaVerify(props.tenantId, challenge().mfa_token, useRecovery() ? 'recovery_code' : 'totp', code()));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const usePasskey = async () => {
    setBusy(true);
    setError('');
    try {
      const token = challenge().mfa_token;
      const { challenge_id, options } = await passkeyChallenge(props.tenantId, { mfaToken: token });
      const cred = await navigator.credentials.get(requestOptions(options, false));
      if (!cred) throw new Error('no credential');
      next(
        await passkeyVerify(
          props.tenantId,
          challenge_id,
          assertionJSON(cred as unknown as Parameters<typeof assertionJSON>[0]),
          token,
        ),
      );
    } catch (e) {
      if (cancelled(e)) setError('The passkey prompt was closed. Try again, or use another method.');
      else fail(e);
    } finally {
      setBusy(false);
    }
  };

  const enrolled = (r: Enrolled) => {
    if (!r.tokens) {
      props.onStartOver('Your sign-in method was added. Sign in again.');
      return;
    }
    if (r.recovery_codes?.length) {
      setCodes({ codes: r.recovery_codes, tokens: r.tokens });
      return;
    }
    props.onAuthenticated(r.tokens);
  };

  const enrolTitle = () =>
    challenge().enrollment_required === 'passkey'
      ? 'Your organisation requires a passkey'
      : 'Your organisation requires two-step sign-in';

  return (
    <Switch>
      <Match when={codes()}>
        <Card title="Save your recovery codes">
          <RecoveryCodes codes={codes()!.codes} onDone={() => props.onAuthenticated(codes()!.tokens)} />
        </Card>
      </Match>
      <Match when={challenge().enrollment_required}>
        <Card
          title={enrolTitle()}
          subtitle="Add a second way to prove it's you. You'll use it every time you sign in."
        >
          <Enroll
            tenantId={props.tenantId}
            mfaToken={challenge().mfa_token}
            allowTotp={challenge().enrollment_required !== 'passkey'}
            onEnrolled={enrolled}
            onExpired={() => props.onStartOver('That sign-in step expired. Sign in again.')}
          />
        </Card>
      </Match>
      <Match when={true}>
        <Card title="Two-step sign-in" subtitle="Confirm it's you to finish signing in.">
          <Show when={error()}>
            <div class="mb-4 rounded-lg bg-error-50 px-4 py-3 text-theme-sm text-error-700" role="alert">
              {error()}
            </div>
          </Show>
          <Show when={has('passkey') && passkeysSupported()}>
            <Button full loading={busy()} onClick={() => void usePasskey()}>
              Use your passkey
            </Button>
            <Show when={has('totp') || has('recovery_code')}>
              <p class="my-4 text-center text-theme-xs text-gray-400">or</p>
            </Show>
          </Show>
          <Show when={has('totp') || (useRecovery() && has('recovery_code'))}>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void verifyCode();
              }}
            >
              <Show
                when={!useRecovery()}
                fallback={
                  <Field
                    label="Recovery code"
                    autocomplete="off"
                    spellcheck={false}
                    required
                    value={code()}
                    onInput={(e) => setCode(e.currentTarget.value)}
                  />
                }
              >
                <Field
                  label="Code from your authenticator app"
                  inputmode="numeric"
                  autocomplete="one-time-code"
                  maxlength={6}
                  required
                  value={code()}
                  onInput={(e) => setCode(e.currentTarget.value.replace(/\D/g, ''))}
                />
              </Show>
              <Button
                type="submit"
                full
                variant={has('passkey') ? 'secondary' : 'primary'}
                loading={busy()}
                disabled={useRecovery() ? code().trim().length < 16 : code().length !== 6}
              >
                Verify
              </Button>
            </form>
          </Show>
          <Show when={has('recovery_code') && (has('totp') || !useRecovery() || has('passkey'))}>
            <Button
              variant="ghost"
              full
              class="mt-2"
              onClick={() => {
                setUseRecovery(!useRecovery());
                setCode('');
                setError('');
              }}
            >
              {useRecovery()
                ? has('totp')
                  ? 'Use your authenticator app instead'
                  : 'Back'
                : 'Lost your device? Use a recovery code'}
            </Button>
          </Show>
          <Button variant="ghost" full class="mt-2" onClick={() => props.onStartOver('')}>
            Start over
          </Button>
        </Card>
      </Match>
    </Switch>
  );
}
