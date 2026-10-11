/**
 * Per-deployment settings, read at start-up from `/config.json` (served by the frontend image from a ConfigMap the
 * chart mounts; see helm/sesame-idam-frontend `runtimeConfig`). One image runs in every environment, so nothing
 * deployment-specific is compiled in. In `vite dev`, without the file, the `VITE_*` variables the app passes in fill
 * the gaps. A value neither provides stays undefined: the app shows what is missing rather than guessing a tenant.
 */
export interface RuntimeConfig {
  /** Hosted sign-in origin, e.g. https://auth.example.com (consoles). */
  authBaseUrl?: string;
  /** The tenant this console signs in to (consoles). */
  tenantId?: string;
  /** Environment shown and managed by the tenant console. */
  environment?: string;
  /** Hosted sign-in: tenant when the request names none. */
  defaultTenant?: string;
  /** Hosted sign-in: client when the request names none. */
  defaultClientId?: string;
}

const KEYS: (keyof RuntimeConfig)[] = [
  'authBaseUrl',
  'tenantId',
  'environment',
  'defaultTenant',
  'defaultClientId',
];

let loaded: RuntimeConfig | undefined;

/** The settings in force (after {@link loadRuntimeConfig}); empty before. */
export function runtimeConfig(): RuntimeConfig {
  return loaded ?? {};
}

/** Load `/config.json` once; `fallback` (the app's `VITE_*` values) fills fields the file does not set. */
export async function loadRuntimeConfig(fallback: RuntimeConfig = {}): Promise<RuntimeConfig> {
  let file: Record<string, unknown> = {};
  try {
    const res = await fetch('/config.json', { cache: 'no-store' });
    if (res.ok) file = (await res.json()) as Record<string, unknown>;
  } catch {
    // No file (vite dev): the fallback alone.
  }
  const out: RuntimeConfig = {};
  for (const key of KEYS) {
    const v = file[key];
    const value = typeof v === 'string' && v.trim() !== '' ? v.trim() : fallback[key]?.trim() || undefined;
    if (value) out[key] = value;
  }
  loaded = out;
  return out;
}

/** The names of required settings that are missing, for a "not configured" screen. */
export function missingSettings(config: RuntimeConfig, required: (keyof RuntimeConfig)[]): string[] {
  return required.filter((k) => !config[k]);
}
