import { createSignal, For, Match, Show, Switch } from 'solid-js';
import { Button, Card, Field } from '@sesame/shared';
import {
  AuthError,
  mfaFactors,
  mfaManage,
  mfaRecoveryCodes,
  mfaRemoveFactor,
  mfaRenameFactor,
} from '../lib/api';
import type { Enrolled, MfaFactor, MfaFactors, TokenResponse } from '../lib/api';
import { Enroll } from '../components/Enroll';
import { RecoveryCodes } from '../components/RecoveryCodes';
import { SignIn } from './SignIn';

export interface SecurityProps {
  tenantId: string;
  clientId: string;
}

type View = 'list' | 'add' | 'codes';

const when = (t?: number | null) => (t ? new Date(t * 1000).toLocaleDateString() : 'never');

/**
 * Sign-in security (ADR-008): passkeys, the authenticator app and recovery codes of the signed-in user.
 *
 * Always starts with a fresh sign-in on this page (with the second factor when the account has one): changing how an
 * account signs in is what a stolen session would most like to do, so Sesame only opens a security session
 * (`/auth/mfa/manage`) for a sign-in made in the last ten minutes. Nothing is kept in storage; the 15-minute session
 * lives in this page's memory only.
 */
export function Security(props: SecurityProps) {
  const [token, setToken] = createSignal<string | null>(null);
  const [state, setState] = createSignal<MfaFactors | null>(null);
  const [view, setView] = createSignal<View>('list');
  const [codes, setCodes] = createSignal<string[]>([]);
  const [error, setError] = createSignal('');
  const [notice, setNotice] = createSignal('');
  const [busy, setBusy] = createSignal(false);
  const [renaming, setRenaming] = createSignal<string | null>(null);
  const [label, setLabel] = createSignal('');

  const ended = () => {
    setToken(null);
    setState(null);
    setView('list');
    setError('Your security session ended. Sign in again to continue.');
  };
  const fail = (e: unknown) => {
    if (e instanceof AuthError && e.code === 'invalid_token') {
      ended();
      return;
    }
    setError(e instanceof AuthError ? e.message : 'Something went wrong. Please try again.');
  };
  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError('');
    setNotice('');
    try {
      await fn();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const signedIn = (tokens: TokenResponse) =>
    run(async () => {
      const session = await mfaManage(props.tenantId, tokens.access_token);
      setToken(session.mfa_token);
      setState(await mfaFactors(props.tenantId, session.mfa_token));
    });

  const enrolled = (r: Enrolled) => {
    setNotice(
      r.factor.kind === 'passkey' ? `Passkey "${r.factor.label}" added.` : 'Authenticator app added.',
    );
    void run(async () => {
      setState(await mfaFactors(props.tenantId, token()!));
      if (r.recovery_codes?.length) {
        setCodes(r.recovery_codes);
        setView('codes');
      } else {
        setView('list');
      }
    });
  };

  const remove = (f: MfaFactor) => {
    const what = f.kind === 'passkey' ? `the passkey "${f.label}"` : 'your authenticator app';
    if (!window.confirm(`Remove ${what}? You will no longer be able to sign in with it.`)) return;
    void run(async () => {
      setState(await mfaRemoveFactor(props.tenantId, token()!, f.id));
      setNotice('Removed.');
    });
  };

  const rename = (f: MfaFactor) =>
    run(async () => {
      setState(await mfaRenameFactor(props.tenantId, token()!, f.id, label().trim()));
      setRenaming(null);
    });

  const regenerate = () => {
    if (!window.confirm('Generate new recovery codes? Your current codes will stop working.')) return;
    void run(async () => {
      const r = await mfaRecoveryCodes(props.tenantId, token()!);
      setCodes(r.recovery_codes);
      setView('codes');
      setState(await mfaFactors(props.tenantId, token()!));
    });
  };

  const policyNote = () => {
    const s = state();
    if (!s || s.required === 'none') return '';
    return s.required === 'passkey'
      ? 'Your organisation requires a passkey to sign in.'
      : 'Your organisation requires two-step sign-in.';
  };

  return (
    <Switch>
      <Match when={!token()}>
        <div class="w-full max-w-md">
          <Show when={error()}>
            <div class="mb-4 rounded-lg bg-error-50 px-4 py-3 text-theme-sm text-error-700" role="alert">
              {error()}
            </div>
          </Show>
          <SignIn
            tenantId={props.tenantId}
            clientId={props.clientId}
            title="Sign in to manage your sign-in methods"
            onAuthenticated={(t) => void signedIn(t)}
          />
        </div>
      </Match>
      <Match when={view() === 'codes'}>
        <Card title="Your recovery codes">
          <RecoveryCodes codes={codes()} onDone={() => setView('list')} doneLabel="Done" />
        </Card>
      </Match>
      <Match when={view() === 'add'}>
        <Card title="Add a sign-in method">
          <Enroll
            tenantId={props.tenantId}
            mfaToken={token()!}
            allowTotp={!!state()?.totp_available && !state()?.factors.some((f) => f.kind === 'totp')}
            onEnrolled={enrolled}
            onCancel={() => setView('list')}
            onExpired={ended}
          />
        </Card>
      </Match>
      <Match when={true}>
        <Card
          title="Sign-in security"
          subtitle="Passkeys and an authenticator app protect your account even if your password is stolen."
        >
          <Show when={error()}>
            <div class="mb-4 rounded-lg bg-error-50 px-4 py-3 text-theme-sm text-error-700" role="alert">
              {error()}
            </div>
          </Show>
          <Show when={notice()}>
            <div class="mb-4 rounded-lg bg-success-50 px-4 py-3 text-theme-sm text-success-700" role="status">
              {notice()}
            </div>
          </Show>
          <Show when={policyNote()}>
            <p class="mb-4 text-theme-sm font-medium text-gray-700 dark:text-gray-200">{policyNote()}</p>
          </Show>
          <Show
            when={state()?.factors.length}
            fallback={
              <p class="mb-4 text-theme-sm text-gray-600 dark:text-gray-300">
                You sign in with your password or an emailed code only. Add a passkey or an authenticator app.
              </p>
            }
          >
            <ul class="mb-4 divide-y divide-gray-100 dark:divide-gray-800">
              <For each={state()!.factors}>
                {(f) => (
                  <li class="py-3">
                    <Show
                      when={renaming() === f.id}
                      fallback={
                        <div class="flex items-start justify-between gap-3">
                          <div>
                            <p class="text-theme-sm font-medium text-gray-900 dark:text-white">
                              {f.label}
                              <span class="ml-2 text-theme-xs font-normal text-gray-500">
                                {f.kind === 'passkey' ? (f.synced ? 'Synced passkey' : 'Passkey on this key') : 'Authenticator app'}
                              </span>
                            </p>
                            <p class="text-theme-xs text-gray-500">
                              Added {when(f.created_at)} · last used {when(f.last_used_at)}
                            </p>
                          </div>
                          <div class="flex shrink-0 gap-1">
                            <Button
                              variant="ghost"
                              onClick={() => {
                                setLabel(f.label);
                                setRenaming(f.id);
                              }}
                            >
                              Rename
                            </Button>
                            <Button variant="ghost" loading={busy()} onClick={() => remove(f)}>
                              Remove
                            </Button>
                          </div>
                        </div>
                      }
                    >
                      <form
                        class="flex items-end gap-2"
                        onSubmit={(e) => {
                          e.preventDefault();
                          void rename(f);
                        }}
                      >
                        <div class="grow">
                          <Field
                            label="Name"
                            maxlength={64}
                            required
                            value={label()}
                            onInput={(e) => setLabel(e.currentTarget.value)}
                          />
                        </div>
                        <Button type="submit" loading={busy()} class="mb-4">
                          Save
                        </Button>
                      </form>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <Button full onClick={() => setView('add')}>
            Add a passkey or authenticator app
          </Button>
          <Show when={state()?.factors.length}>
            <div class="mt-6 border-t border-gray-100 pt-4 dark:border-gray-800">
              <p class="mb-2 text-theme-sm text-gray-700 dark:text-gray-300">
                Recovery codes: {state()?.recovery_codes_left ?? 0} unused
              </p>
              <Button variant="secondary" full loading={busy()} onClick={regenerate}>
                Generate new recovery codes
              </Button>
            </div>
          </Show>
          <p class="mt-6 text-center text-theme-xs text-gray-500">You can close this tab when you are done.</p>
        </Card>
      </Match>
    </Switch>
  );
}
