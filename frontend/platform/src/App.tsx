import { createSignal, onMount, Show } from 'solid-js';
import { Button, Card, ConsoleShell, StatusPill, bootstrapSession, createConsoleClient, missingSettings, runtimeConfig } from '@sesame/shared';
import type { Session } from '@sesame/idam-client';

/**
 * Sesame PLATFORM console (ADR-010) — the operator's view.
 *
 * Scope (ADR-004/009 platform surfaces): tenants, environments, platform SMS
 * sender + spend, signing keys/JWKS health, audit stream.
 *
 * Design language: Flux-Operator-inspired — stripped-down, status-first,
 * light/dark first-class, colour reserved for state semantics.
 *
 * Auth: this console signs in THROUGH the hosted auth surface via
 * @sesame/idam-client — dogfooding the exact path external tenants use.
 */
function Console() {
  const config = runtimeConfig();
  const nav = [
    { label: 'Overview', href: '#', current: true },
    { label: 'Tenants', href: '#tenants' },
    { label: 'Environments', href: '#environments' },
    { label: 'Signing keys', href: '#keys' },
    { label: 'SMS & spend', href: '#sms' },
    { label: 'Audit', href: '#audit' },
  ];

  // Dogfood: this console signs in through the hosted auth surface with the
  // very SDK external tenants use.
  const client = createConsoleClient({
    authBaseUrl: config.authBaseUrl ?? '',
    tenantId: config.tenantId ?? '',
  });
  const [session, setSession] = createSignal<Session | null>(null);
  onMount(async () => setSession(await bootstrapSession(client)));

  const actions = (
    <Show
      when={session()}
      fallback={
        <Button onClick={() => client.login()} variant="primary">
          Sign in
        </Button>
      }
    >
      <span class="text-theme-sm text-gray-500">{session()?.userId}</span>
      <Button variant="ghost" onClick={() => client.logout()}>
        Sign out
      </Button>
    </Show>
  );

  return (
    <ConsoleShell product="Sesame Platform" nav={nav} actions={actions}>
      <h1 class="mb-1 text-title-sm font-semibold text-gray-900 dark:text-white">Overview</h1>
      <p class="mb-6 text-theme-sm text-gray-500">Platform health at a glance.</p>

      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <Card title="Identity session" subtitle="JWKS issuer">
          <div class="flex items-center justify-between">
            <span class="text-theme-sm text-gray-500">2 replicas · shared keyset</span>
            <StatusPill status="ready" />
          </div>
        </Card>
        <Card title="Tenants" subtitle="Registered">
          <div class="flex items-center justify-between">
            <span class="text-title-md font-semibold text-gray-900 dark:text-white">—</span>
            <StatusPill status="unknown" label="not wired" />
          </div>
        </Card>
        <Card title="SMS spend" subtitle="Platform, today">
          <div class="flex items-center justify-between">
            <span class="text-title-md font-semibold text-gray-900 dark:text-white">—</span>
            <StatusPill status="unknown" label="not wired" />
          </div>
        </Card>
        <Card title="Auth surface" subtitle="sesame-auth · own origin">
          <div class="flex items-center justify-between">
            <span class="text-theme-sm text-gray-500">sign-in · OTP · magic link · reset</span>
            <StatusPill status="ready" />
          </div>
        </Card>
        <Card title="Signing keyset" subtitle="ADR-006 step 1">
          <div class="flex items-center justify-between">
            <span class="text-theme-sm text-gray-500">SOPS-delivered · 2 keys</span>
            <StatusPill status="ready" />
          </div>
        </Card>
        <Card title="SMS custody" subtitle="ADR-009">
          <div class="flex items-center justify-between">
            <span class="text-theme-sm text-gray-500">Connect (ext) · envelope (dogfood)</span>
            <StatusPill status="pending" label="design" />
          </div>
        </Card>
      </div>
    </ConsoleShell>
  );
}

/** The console, or the settings this deployment has not provided (`/config.json`). */
export function App() {
  const missing = missingSettings(runtimeConfig(), ['authBaseUrl', 'tenantId']);
  if (missing.length > 0) {
    return (
      <main class="flex min-h-screen items-center justify-center px-4">
        <p class="max-w-md text-theme-sm text-gray-600 dark:text-gray-300">
          This console is not configured: {missing.join(', ')} missing from /config.json.
        </p>
      </main>
    );
  }
  return <Console />;
}
