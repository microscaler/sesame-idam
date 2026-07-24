import { ConsoleShell, StatusPill, Card } from '@sesame/shared';

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
export function App() {
  const nav = [
    { label: 'Overview', href: '#', current: true },
    { label: 'Tenants', href: '#tenants' },
    { label: 'Environments', href: '#environments' },
    { label: 'Signing keys', href: '#keys' },
    { label: 'SMS & spend', href: '#sms' },
    { label: 'Audit', href: '#audit' },
  ];

  return (
    <ConsoleShell product="Sesame Platform" nav={nav}>
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
      </div>
    </ConsoleShell>
  );
}
