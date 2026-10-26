import { createSignal, For } from 'solid-js';
import { Button } from '@sesame/shared';

export interface RecoveryCodesProps {
  codes: string[];
  onDone: () => void;
  doneLabel?: string;
}

/**
 * One-time recovery codes, shown once (ADR-008 §6). The user must confirm they kept them before moving on: these are
 * the way back in when the authenticator app or passkey is lost, and Sesame stores only their hashes.
 */
export function RecoveryCodes(props: RecoveryCodesProps) {
  const [saved, setSaved] = createSignal(false);
  const [copied, setCopied] = createSignal(false);
  const text = () => props.codes.join('\n');

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text());
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };
  const download = () => {
    const blob = new Blob(
      [`Recovery codes (each works once). Generated ${new Date().toISOString()}\n\n${text()}\n`],
      { type: 'text/plain' },
    );
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'recovery-codes.txt';
    a.click();
    URL.revokeObjectURL(a.href);
  };

  return (
    <div>
      <p class="mb-3 text-theme-sm text-gray-600 dark:text-gray-300">
        Save these recovery codes somewhere safe, away from your phone. If you lose your authenticator or passkey, each
        code signs you in once. They will not be shown again.
      </p>
      <ol class="mb-4 grid grid-cols-2 gap-2 rounded-lg bg-gray-50 p-4 font-mono text-theme-sm text-gray-900 dark:bg-gray-800 dark:text-gray-100">
        <For each={props.codes}>{(code) => <li>{code}</li>}</For>
      </ol>
      <div class="mb-4 flex gap-2">
        <Button variant="secondary" onClick={() => void copy()}>
          {copied() ? 'Copied' : 'Copy'}
        </Button>
        <Button variant="secondary" onClick={download}>
          Download
        </Button>
      </div>
      <label class="mb-4 flex items-center gap-2 text-theme-sm text-gray-700 dark:text-gray-300">
        <input type="checkbox" checked={saved()} onChange={(e) => setSaved(e.currentTarget.checked)} />
        I have saved my recovery codes
      </label>
      <Button full disabled={!saved()} onClick={() => props.onDone()}>
        {props.doneLabel ?? 'Continue'}
      </Button>
    </div>
  );
}
