import { createSignal, Show } from 'solid-js';
import qrcode from 'qrcode-generator';
import { Button, Field } from '@sesame/shared';
import {
  AuthError,
  passkeyRegister,
  passkeyRegisterOptions,
  totpConfirm,
  totpEnroll,
} from '../lib/api';
import type { Enrolled } from '../lib/api';
import { cancelled, creationOptions, passkeysSupported, registrationJSON } from '../lib/webauthn';

export interface EnrollProps {
  tenantId: string;
  /** A management session, or a sign-in that must enrol. */
  mfaToken: string;
  /** Offer an authenticator app (false when policy requires a passkey). */
  allowTotp: boolean;
  onEnrolled: (result: Enrolled) => void;
  onCancel?: () => void;
  /** The session ended (expired or used): the caller starts again. */
  onExpired: () => void;
}

type Step = 'choose' | 'totp';

/**
 * Add a passkey (preferred: phishing-resistant, one gesture) or an authenticator app (TOTP: a QR code, then a first
 * code to confirm it). Shared by the security page and the sign-in step an organisation's policy forces.
 */
export function Enroll(props: EnrollProps) {
  const [step, setStep] = createSignal<Step>('choose');
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal('');
  const [secret, setSecret] = createSignal<{ factor_id: string; secret: string; otpauth_uri: string } | null>(null);
  const [code, setCode] = createSignal('');

  const fail = (e: unknown) => {
    if (e instanceof AuthError && e.code === 'invalid_token') {
      props.onExpired();
      return;
    }
    setError(e instanceof AuthError ? e.message : 'Something went wrong. Please try again.');
  };

  const addPasskey = async () => {
    setBusy(true);
    setError('');
    try {
      const { challenge_id, options } = await passkeyRegisterOptions(props.tenantId, props.mfaToken);
      const cred = await navigator.credentials.create(creationOptions(options));
      if (!cred) throw new Error('no credential');
      const result = await passkeyRegister(
        props.tenantId,
        props.mfaToken,
        challenge_id,
        registrationJSON(cred as unknown as Parameters<typeof registrationJSON>[0]),
      );
      props.onEnrolled(result);
    } catch (e) {
      if (cancelled(e)) {
        setError('The passkey was not created. Try again when you are ready.');
      } else {
        fail(e);
      }
    } finally {
      setBusy(false);
    }
  };

  const startTotp = async () => {
    setBusy(true);
    setError('');
    try {
      setSecret(await totpEnroll(props.tenantId, props.mfaToken));
      setStep('totp');
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const confirmTotp = async () => {
    const s = secret();
    if (!s) return;
    setBusy(true);
    setError('');
    try {
      props.onEnrolled(await totpConfirm(props.tenantId, props.mfaToken, s.factor_id, code()));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const qrSvg = () => {
    const s = secret();
    if (!s) return '';
    const qr = qrcode(0, 'M');
    qr.addData(s.otpauth_uri);
    qr.make();
    return qr.createSvgTag({ cellSize: 4, margin: 2, scalable: true });
  };

  return (
    <div>
      <Show when={error()}>
        <div class="mb-4 rounded-lg bg-error-50 px-4 py-3 text-theme-sm text-error-700" role="alert">
          {error()}
        </div>
      </Show>

      <Show when={step() === 'choose'}>
        <div class="flex flex-col gap-3">
          <Show when={passkeysSupported()}>
            <Button full loading={busy()} onClick={() => void addPasskey()}>
              Add a passkey
            </Button>
            <p class="-mt-1 text-theme-xs text-gray-500">
              Recommended. Sign in with your fingerprint, face or device PIN. It cannot be phished.
            </p>
          </Show>
          <Show when={props.allowTotp}>
            <Button variant="secondary" full loading={busy()} onClick={() => void startTotp()}>
              Use an authenticator app
            </Button>
          </Show>
          <Show when={!props.allowTotp && !passkeysSupported()}>
            <p class="text-theme-sm text-gray-600 dark:text-gray-300">
              Your organisation requires a passkey, and this browser cannot create one. Use a current browser on a
              device with a screen lock.
            </p>
          </Show>
          <Show when={props.onCancel}>
            <Button variant="ghost" full onClick={() => props.onCancel?.()}>
              Cancel
            </Button>
          </Show>
        </div>
      </Show>

      <Show when={step() === 'totp' && secret()}>
        <p class="mb-3 text-theme-sm text-gray-600 dark:text-gray-300">
          Scan this code with your authenticator app (Google Authenticator, 1Password, Authy…), then enter the 6-digit
          code it shows.
        </p>
        {/* eslint-disable-next-line solid/no-innerhtml -- SVG generated locally from our own otpauth URI */}
        <div class="mx-auto mb-3 w-48 bg-white p-2" innerHTML={qrSvg()} />
        <details class="mb-4 text-theme-xs text-gray-500">
          <summary class="cursor-pointer">Can't scan? Enter this key instead</summary>
          <code class="mt-2 block break-all font-mono text-theme-sm text-gray-900 dark:text-gray-100">
            {secret()?.secret.replace(/(.{4})/g, '$1 ').trim()}
          </code>
        </details>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void confirmTotp();
          }}
        >
          <Field
            label="Code from the app"
            inputmode="numeric"
            autocomplete="one-time-code"
            maxlength={6}
            required
            value={code()}
            onInput={(e) => setCode(e.currentTarget.value.replace(/\D/g, ''))}
          />
          <Button type="submit" full loading={busy()} disabled={code().length !== 6}>
            Confirm
          </Button>
        </form>
        <Button variant="ghost" full class="mt-2" onClick={() => setStep('choose')}>
          Back
        </Button>
      </Show>
    </div>
  );
}
