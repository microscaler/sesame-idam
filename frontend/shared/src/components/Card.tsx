import type { JSX } from 'solid-js';

export interface CardProps {
  title?: string;
  subtitle?: string;
  children: JSX.Element;
  class?: string;
}

/** Centered content card — the auth surface's primary container. */
export function Card(props: CardProps) {
  return (
    <div
      class={`w-full max-w-md rounded-2xl border border-gray-200 bg-white p-8 shadow-theme-lg dark:border-gray-700 dark:bg-gray-900 ${props.class ?? ''}`}
    >
      {props.title && <h1 class="text-title-sm font-semibold text-gray-900 dark:text-white">{props.title}</h1>}
      {props.subtitle && <p class="mt-2 text-theme-sm text-gray-500 dark:text-gray-400">{props.subtitle}</p>}
      <div class="mt-6">{props.children}</div>
    </div>
  );
}
