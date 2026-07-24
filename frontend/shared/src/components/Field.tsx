import type { JSX } from 'solid-js';
import { splitProps } from 'solid-js';

export interface FieldProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
  label: string;
  error?: string;
  hint?: string;
}

/** Labelled input with error/hint slots. */
export function Field(props: FieldProps) {
  const [local, rest] = splitProps(props, ['label', 'error', 'hint', 'class', 'id']);
  const id = () => local.id ?? local.label.toLowerCase().replace(/\s+/g, '-');

  return (
    <div class="mb-4">
      <label for={id()} class="mb-1.5 block text-theme-sm font-medium text-gray-700 dark:text-gray-300">
        {local.label}
      </label>
      <input
        id={id()}
        class={`w-full rounded-lg border px-4 py-2.5 text-theme-sm outline-none transition placeholder:text-gray-400 focus:ring-2 focus:ring-brand-300 dark:bg-gray-900 dark:text-white ${
          local.error ? 'border-error-500' : 'border-gray-300 dark:border-gray-700'
        } ${local.class ?? ''}`}
        {...rest}
      />
      {local.error && <p class="mt-1.5 text-theme-xs text-error-600">{local.error}</p>}
      {!local.error && local.hint && <p class="mt-1.5 text-theme-xs text-gray-500">{local.hint}</p>}
    </div>
  );
}
