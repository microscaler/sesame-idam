import { Button } from '@sesame/shared';

/**
 * Sesame brochure site (ADR-010) — the public marketing surface.
 *
 * Leads with the launch wedge (ROADMAP-launch-1.0 §1): open-source +
 * self-hosted, standards-based asymmetric JWT/JWKS, and a database-native RLS
 * bridge — the triple no competitor has.
 */
export function App() {
  const pillars = [
    {
      title: 'Open source, self-hosted',
      body: 'Run the whole identity platform yourself. No per-MAU pricing, no opaque tokens, no lock-in.',
    },
    {
      title: 'Standards-based JWT/JWKS',
      body: 'Asymmetric EdDSA access tokens with rotation, grace windows and revocation — verifiable by anyone, anywhere.',
    },
    {
      title: 'Database-native RLS',
      body: 'Tenant isolation proven at the data layer, not just in application code.',
    },
  ];

  return (
    <div class="min-h-screen bg-white dark:bg-gray-950">
      <header class="mx-auto flex max-w-6xl items-center justify-between px-6 py-6">
        <span class="text-theme-sm font-semibold tracking-tight text-gray-900 dark:text-white">Sesame</span>
        <nav class="flex items-center gap-6 text-theme-sm text-gray-600 dark:text-gray-400">
          <a href="#docs" class="hover:text-gray-900 dark:hover:text-white">Docs</a>
          <a href="#pricing" class="hover:text-gray-900 dark:hover:text-white">Pricing</a>
          <Button variant="primary">Get started</Button>
        </nav>
      </header>

      <section class="mx-auto max-w-4xl px-6 py-24 text-center">
        <h1 class="text-title-lg font-semibold tracking-tight text-gray-900 dark:text-white">
          Identity your platform engineers will envy
        </h1>
        <p class="mx-auto mt-6 max-w-2xl text-theme-xl text-gray-600 dark:text-gray-400">
          Auth with zero logic in your app. Standards-based asymmetric tokens. Row-level security nobody else offers.
        </p>
        <div class="mt-10 flex justify-center gap-3">
          <Button variant="primary">Start self-hosting</Button>
          <Button variant="secondary">Read the docs</Button>
        </div>
      </section>

      <section class="mx-auto grid max-w-6xl gap-8 px-6 pb-24 md:grid-cols-3">
        {pillars.map((p) => (
          <div class="rounded-2xl border border-gray-200 p-6 dark:border-gray-800">
            <h2 class="text-theme-xl font-medium text-gray-900 dark:text-white">{p.title}</h2>
            <p class="mt-2 text-theme-sm text-gray-600 dark:text-gray-400">{p.body}</p>
          </div>
        ))}
      </section>
    </div>
  );
}
