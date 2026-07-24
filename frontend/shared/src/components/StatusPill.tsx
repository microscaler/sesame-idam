import type { JSX } from 'solid-js';

export type Status = 'ready' | 'pending' | 'failed' | 'suspended' | 'unknown';

const STYLES: Record<Status, string> = {
  ready: 'bg-success-50 text-success-700 ring-success-500/20',
  pending: 'bg-warning-50 text-warning-700 ring-warning-500/20',
  failed: 'bg-error-50 text-error-700 ring-error-500/20',
  suspended: 'bg-gray-100 text-gray-600 ring-gray-500/20',
  unknown: 'bg-gray-100 text-gray-500 ring-gray-400/20',
};

/**
 * Status-first console primitive (ADR-010 console design language).
 * Colour is reserved for state semantics — never decoration.
 */
export function StatusPill(props: { status: Status; label?: string }): JSX.Element {
  return (
    <span
      class={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-theme-xs font-medium ring-1 ring-inset ${STYLES[props.status]}`}
    >
      <span class="h-1.5 w-1.5 rounded-full bg-current opacity-70" />
      {props.label ?? props.status}
    </span>
  );
}
