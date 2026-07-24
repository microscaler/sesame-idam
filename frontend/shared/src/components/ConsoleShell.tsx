import type { JSX } from 'solid-js';
import { For } from 'solid-js';

export interface NavItem {
  label: string;
  href: string;
  current?: boolean;
}

/**
 * Console layout shell (ADR-010): stripped-down sidebar + content, light/dark
 * first-class, minimal chrome. Shared by the platform and tenant consoles so
 * they are visibly one product.
 */
export function ConsoleShell(props: {
  product: string;
  nav: NavItem[];
  children: JSX.Element;
  actions?: JSX.Element;
}): JSX.Element {
  return (
    <div class="flex min-h-screen bg-gray-25 dark:bg-gray-950">
      <aside class="hidden w-60 shrink-0 border-r border-gray-200 bg-white px-4 py-6 dark:border-gray-800 dark:bg-gray-900 md:block">
        <div class="px-2 pb-6 text-theme-sm font-semibold tracking-tight text-gray-900 dark:text-white">
          {props.product}
        </div>
        <nav class="flex flex-col gap-0.5">
          <For each={props.nav}>
            {(item) => (
              <a
                href={item.href}
                aria-current={item.current ? 'page' : undefined}
                class={`rounded-lg px-3 py-2 text-theme-sm transition ${
                  item.current
                    ? 'bg-gray-100 font-medium text-gray-900 dark:bg-gray-800 dark:text-white'
                    : 'text-gray-600 hover:bg-gray-50 dark:text-gray-400 dark:hover:bg-gray-800/50'
                }`}
              >
                {item.label}
              </a>
            )}
          </For>
        </nav>
      </aside>
      <div class="flex min-w-0 flex-1 flex-col">
        <header class="flex h-14 items-center justify-between border-b border-gray-200 bg-white px-6 dark:border-gray-800 dark:bg-gray-900">
          <span class="text-theme-sm text-gray-500 md:hidden">{props.product}</span>
          <div class="ml-auto flex items-center gap-2">{props.actions}</div>
        </header>
        <main class="flex-1 overflow-auto p-6">{props.children}</main>
      </div>
    </div>
  );
}
