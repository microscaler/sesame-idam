/**
 * Runtime tenant theming (ADR-010 §2.2 — "hosted ≠ unbranded").
 *
 * The hosted auth surface fetches the tenant's branding from the tenant
 * config (ADR-009) and applies it by setting CSS variables — the same bundle
 * serves every tenant under their own ADR-007 verified domain.
 */

export interface TenantTheme {
  /** Primary brand colour (any CSS colour). */
  primary?: string;
  /** Foreground on primary (contrast-safe). */
  onPrimary?: string;
  /** Logo URL rendered above the auth card. */
  logoUrl?: string;
  /** Display name used in copy ("Sign in to Acme"). */
  displayName?: string;
  /** Corner radius token. */
  radius?: string;
}

/** Apply a tenant theme to the document root. Safe to call repeatedly. */
export function applyTenantTheme(theme: TenantTheme, root: HTMLElement = document.documentElement): void {
  if (theme.primary) root.style.setProperty('--sesame-brand-primary', theme.primary);
  if (theme.onPrimary) root.style.setProperty('--sesame-brand-on-primary', theme.onPrimary);
  if (theme.radius) root.style.setProperty('--sesame-radius', theme.radius);
}
