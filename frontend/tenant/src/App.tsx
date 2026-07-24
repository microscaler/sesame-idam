import { createSignal, onMount, Show } from 'solid-js';
import { Button, Card, ConsoleShell, StatusPill, bootstrapSession, createConsoleClient } from '@sesame/shared';
import type { Session } from '@sesame/idam-client';

/**
 * Sesame TENANT console (ADR-010) — a tenant admin's view of their own
 * partition.
 *
 * Scope: users, organisations, applications, branding (the theme applied to
 * the hosted auth surface), verified domains (ADR-007), tenant SMS config +
 * spend (ADR-009), tenant audit.
 *
 * Same design language and shell as the platform console — one product, two
 * audiences. Auth also runs through the hosted surface via
 * @sesame/idam-client.
 */
export function App() {
  const nav = [
    { label: 'Overview', href: '#', current: true },
    { label: 'Users', href: '#users' },
    { label: 'Organisations', href: '#orgs' },
    { label: 'Applications', href: '#apps' },
    { label: 'Branding', href: '#branding' },
    { label: 'Domains', href: '#domains' },
    { label: 'SMS & spend', href: '#sms' },
  ];

  // Dogfood: same hosted surface + same SDK a tenant's own app would use.
  const client = createConsoleClient({
    authBaseUrl: import.meta.env.VITE_AUTH_BASE_URL ?? 'https://sesame-auth.dev.microscaler.local',
    tenantId: import.meta.env.VITE_TENANT_ID ?? 'hauliage',
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
    <ConsoleShell product="Sesame Tenant" nav={nav} actions={actions}>
      <h1 class="mb-1 text-title-sm font-semibold text-gray-900 dark:text-white">Overview</h1>
      <p class="mb-6 text-theme-sm text-gray-500">Your identity partition at a glance.</p>

      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <Card title="Sign-in methods" subtitle="Hosted surface">
          <ul class="space-y-2 text-theme-sm">
            <li class="flex items-center justify-between">
              <span class="text-gray-600 dark:text-gray-300">Password</span>
              <StatusPill status="ready" label="live" />
            </li>
            <li class="flex items-center justify-between">
              <span class="text-gray-600 dark:text-gray-300">Email OTP</span>
              <StatusPill status="ready" label="live" />
            </li>
            <li class="flex items-center justify-between">
              <span class="text-gray-600 dark:text-gray-300">Magic link</span>
              <StatusPill status="ready" label="live" />
            </li>
            <li class="flex items-center justify-between">
              <span class="text-gray-600 dark:text-gray-300">Password reset</span>
              <StatusPill status="ready" label="live" />
            </li>
            <li class="flex items-center justify-between">
              <span class="text-gray-600 dark:text-gray-300">SMS OTP</span>
              <StatusPill status="suspended" label="cost-gated" />
            </li>
            <li class="flex items-center justify-between">
              <span class="text-gray-600 dark:text-gray-300">Passkeys</span>
              <StatusPill status="pending" label="ADR-008" />
            </li>
          </ul>
        </Card>
        <Card title="Users" subtitle="Active">
          <div class="flex items-center justify-between">
            <span class="text-title-md font-semibold text-gray-900 dark:text-white">—</span>
            <StatusPill status="unknown" label="not wired" />
          </div>
        </Card>
        <Card title="Domain" subtitle="Verification (ADR-007)">
          <div class="flex items-center justify-between">
            <span class="text-theme-sm text-gray-500">not configured</span>
            <StatusPill status="pending" />
          </div>
        </Card>
      </div>
    </ConsoleShell>
  );
}
