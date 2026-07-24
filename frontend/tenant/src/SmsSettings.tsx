import { createResource, createSignal, Show } from 'solid-js';
import { Button, Card, Field, StatusPill } from '@sesame/shared';
import { ApiError, smsApi, type SmsConfig, type SmsConfigInput } from './api';

/**
 * Tenant SMS sender configuration (ADR-009 §7).
 *
 * # Two things this screen is careful about
 *
 * **The credential is write-only.** The input starts empty even when a
 * credential is stored, and the stored value is never fetched. "Configured ✓"
 * plus a last-validated timestamp is all the confirmation an admin needs, and
 * it is all an attacker with the admin's session would get.
 *
 * **Connect is the recommended path.** It is presented first and explained in
 * terms of what the tenant gets from it — Twilio bills them directly and
 * Sesame holds nothing to lose. Handing us a raw token is the fallback, and
 * the server will refuse it unless the tenant is on the dogfood allow-list.
 */
export function SmsSettings(props: { tenant: string; environment: string }) {
  const [config, { refetch }] = createResource(
    () => [props.tenant, props.environment] as const,
    ([t, e]) => smsApi.get(t, e),
  );

  const [custody, setCustody] = createSignal<'connect' | 'envelope'>('connect');
  const [connectedSid, setConnectedSid] = createSignal('');
  const [accountSid, setAccountSid] = createSignal('');
  const [authToken, setAuthToken] = createSignal('');
  const [fromNumber, setFromNumber] = createSignal('');
  const [campaignRef, setCampaignRef] = createSignal('');
  const [ceiling, setCeiling] = createSignal('');
  const [error, setError] = createSignal('');
  const [notice, setNotice] = createSignal('');
  const [busy, setBusy] = createSignal(false);

  const mode = () => custody();
  const stored = () => config()?.credential_configured ?? false;

  async function save(e: Event) {
    e.preventDefault();
    setError('');
    setNotice('');
    setBusy(true);
    try {
      const body: SmsConfigInput = { custody_mode: mode() };
      if (mode() === 'connect') {
        body.connected_account_sid = connectedSid().trim();
      } else {
        body.account_sid = accountSid().trim();
        // Omitted when blank: editing a from number must not require
        // re-entering the secret.
        if (authToken().trim()) body.auth_token = authToken().trim();
      }
      if (fromNumber().trim()) body.from_number = fromNumber().trim();
      if (campaignRef().trim()) body.campaign_ref = campaignRef().trim();
      if (ceiling().trim()) body.daily_spend_ceiling_cents = Number(ceiling().trim());

      await smsApi.save(props.tenant, props.environment, body);
      // Clear the secret from memory the moment it has been sent.
      setAuthToken('');
      setNotice('Saved. Sending stays disabled until the credential validates.');
      void refetch();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Could not save the configuration.');
    } finally {
      setBusy(false);
    }
  }

  async function revoke() {
    setError('');
    setNotice('');
    setBusy(true);
    try {
      await smsApi.revoke(props.tenant, props.environment);
      setNotice('Revoked. SMS sending has stopped; flows fall back to email.');
      void refetch();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Could not revoke the configuration.');
    } finally {
      setBusy(false);
    }
  }

  const statusPill = (c: SmsConfig) => {
    if (c.status === 'active') return <StatusPill status="ready" label="active" />;
    if (c.status === 'revoked') return <StatusPill status="suspended" label="revoked" />;
    return <StatusPill status="pending" label="awaiting validation" />;
  };

  return (
    <div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
      <Card title="SMS sender" subtitle={`${props.tenant} · ${props.environment}`}>
        <form onSubmit={save}>
          <fieldset class="mb-5">
            <legend class="mb-2 text-theme-sm font-medium text-gray-700 dark:text-gray-300">
              Custody
            </legend>

            <label class="mb-3 flex cursor-pointer gap-3 rounded-lg border border-gray-200 p-3 dark:border-gray-700">
              <input
                type="radio"
                name="custody"
                class="mt-1"
                checked={mode() === 'connect'}
                onChange={() => setCustody('connect')}
              />
              <span>
                <span class="block text-theme-sm font-medium text-gray-900 dark:text-white">
                  Twilio Connect <span class="text-brand-500">· recommended</span>
                </span>
                <span class="block text-theme-xs text-gray-500">
                  You authorise Sesame on your own Twilio account. Twilio bills you directly, and
                  Sesame stores no credential — you can revoke access from Twilio at any time.
                </span>
              </span>
            </label>

            <label class="flex cursor-pointer gap-3 rounded-lg border border-gray-200 p-3 dark:border-gray-700">
              <input
                type="radio"
                name="custody"
                class="mt-1"
                checked={mode() === 'envelope'}
                onChange={() => setCustody('envelope')}
              />
              <span>
                <span class="block text-theme-sm font-medium text-gray-900 dark:text-white">
                  Stored credentials
                </span>
                <span class="block text-theme-xs text-gray-500">
                  Sesame holds your auth token, encrypted per-credential at rest. Available to
                  approved tenants only.
                </span>
              </span>
            </label>
          </fieldset>

          <Show when={mode() === 'connect'}>
            <Field
              label="Connected account SID"
              placeholder="AC…"
              value={connectedSid()}
              onInput={(e) => setConnectedSid(e.currentTarget.value)}
              hint="From the Twilio Connect authorisation. Not a secret."
            />
          </Show>

          <Show when={mode() === 'envelope'}>
            <Field
              label="Account SID"
              placeholder="AC…"
              value={accountSid()}
              onInput={(e) => setAccountSid(e.currentTarget.value)}
            />
            <Field
              label="Auth token"
              type="password"
              autocomplete="off"
              placeholder={stored() ? 'Leave blank to keep the stored token' : 'Twilio auth token'}
              value={authToken()}
              onInput={(e) => setAuthToken(e.currentTarget.value)}
              hint={
                stored()
                  ? 'A token is stored. It is never displayed — enter a new one only to replace it.'
                  : 'Sent once, encrypted on receipt, and never shown again.'
              }
            />
          </Show>

          <Field
            label="From number"
            placeholder="+441234567890"
            value={fromNumber()}
            onInput={(e) => setFromNumber(e.currentTarget.value)}
          />
          <Field
            label="A2P / 10DLC campaign reference"
            placeholder="Optional"
            value={campaignRef()}
            onInput={(e) => setCampaignRef(e.currentTarget.value)}
            hint="Required by carriers for application-to-person traffic in some regions."
          />
          <Field
            label="Daily spend ceiling (cents)"
            type="number"
            min="0"
            placeholder={String(config()?.daily_spend_ceiling_cents ?? 500)}
            value={ceiling()}
            onInput={(e) => setCeiling(e.currentTarget.value)}
            hint="Sends stop for the rest of the day once this is reached."
          />

          <Show when={error()}>
            <p class="mb-3 text-theme-sm text-error-600">{error()}</p>
          </Show>
          <Show when={notice()}>
            <p class="mb-3 text-theme-sm text-success-600">{notice()}</p>
          </Show>

          <div class="flex gap-3">
            <Button type="submit" variant="primary" disabled={busy()}>
              {busy() ? 'Saving…' : 'Save configuration'}
            </Button>
            <Show when={config()}>
              <Button type="button" variant="ghost" disabled={busy()} onClick={revoke}>
                Revoke
              </Button>
            </Show>
          </div>
        </form>
      </Card>

      <Card title="Current state" subtitle="Server-side truth">
        <Show
          when={config()}
          fallback={<p class="text-theme-sm text-gray-500">No SMS sender configured — flows use email.</p>}
        >
          {(c) => (
            <ul class="space-y-3 text-theme-sm">
              <li class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-300">Status</span>
                {statusPill(c())}
              </li>
              <li class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-300">Custody</span>
                <span class="text-gray-900 dark:text-white">{c().custody_mode}</span>
              </li>
              <li class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-300">Credential</span>
                <span class="text-gray-900 dark:text-white">
                  {c().credential_configured ? 'configured ✓' : 'not set'}
                </span>
              </li>
              <li class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-300">Last validated</span>
                <span class="text-gray-900 dark:text-white">
                  {c().last_validated_at ? new Date(c().last_validated_at!).toLocaleString() : 'never'}
                </span>
              </li>
              <li class="flex items-center justify-between">
                <span class="text-gray-600 dark:text-gray-300">Daily ceiling</span>
                <span class="text-gray-900 dark:text-white">
                  {(c().daily_spend_ceiling_cents / 100).toFixed(2)}
                </span>
              </li>
            </ul>
          )}
        </Show>
        <p class="mt-4 border-t border-gray-200 pt-4 text-theme-xs text-gray-500 dark:border-gray-700">
          SMS is reserved for registration and password reset. Everyday sign-in OTP goes by email —
          it costs nothing per message and is not carrier-rate-limited.
        </p>
      </Card>
    </div>
  );
}
