/**
 * @sesame/shared — design system surface (ADR-010).
 *
 * Consumed by brochure / platform / tenant / auth so the product family stays
 * visually coherent, and so the hosted auth surface can be themed per tenant
 * at runtime (styles/theme.css + applyTenantTheme).
 */

export { Button } from './components/Button';
export { Card } from './components/Card';
export { Field } from './components/Field';
export { StatusPill } from './components/StatusPill';
export type { Status } from './components/StatusPill';
export { ConsoleShell } from './components/ConsoleShell';
export type { NavItem } from './components/ConsoleShell';
export { applyTenantTheme } from './theme';
export type { TenantTheme } from './theme';
