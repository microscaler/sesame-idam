import type { JSX } from 'solid-js';
import { splitProps } from 'solid-js';

export interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost';
  full?: boolean;
  loading?: boolean;
}

/** Brand-themeable button — `primary` uses the runtime tenant colour. */
export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ['variant', 'full', 'loading', 'class', 'children', 'disabled']);
  const variant = () => local.variant ?? 'primary';
  const base =
    'inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-theme-sm font-medium transition disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-brand-300';
  const styles = () =>
    ({
      primary: 'bg-brand-primary text-brand-on-primary hover:opacity-90 shadow-theme-xs',
      secondary: 'bg-gray-100 text-gray-900 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-100',
      ghost: 'text-gray-700 hover:bg-gray-100 dark:text-gray-200 dark:hover:bg-gray-800',
    })[variant()];

  return (
    <button
      class={`${base} ${styles()} ${local.full ? 'w-full' : ''} ${local.class ?? ''}`}
      disabled={local.disabled || local.loading}
      {...rest}
    >
      {local.loading ? 'Please wait…' : local.children}
    </button>
  );
}
