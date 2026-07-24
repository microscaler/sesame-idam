import { ConsoleShell, StatusPill, Card } from '@sesame/shared';

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

  return (
    <ConsoleShell product="Sesame Tenant" nav={nav}>
      <h1 class="mb-1 text-title-sm font-semibold text-gray-900 dark:text-white">Overview</h1>
      <p class="mb-6 text-theme-sm text-gray-500">Your identity partition at a glance.</p>

      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <Card title="Sign-in" subtitle="Hosted surface">
          <div class="flex items-center justify-between">
            <span class="text-theme-sm text-gray-500">password · email OTP · magic link</span>
            <StatusPill status="ready" />
          </div>
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
