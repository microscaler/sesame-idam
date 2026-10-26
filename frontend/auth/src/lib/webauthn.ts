/**
 * WebAuthn options and responses as JSON (ADR-008 passkeys).
 *
 * Sesame (webauthn-rs) sends options with every binary member base64url-encoded and expects the browser's response
 * in the same encoding. The browser API works in ArrayBuffers. These helpers convert both ways, and are kept free of
 * DOM-only types where possible so the encoding runs under `node --test` (src/lib/webauthn.test.mjs).
 */

/** base64url (no padding) to bytes. */
export function fromB64url(value: string): Uint8Array {
  const b64 = value.replace(/-/g, '+').replace(/_/g, '/');
  const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4);
  const bin = atob(padded);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** bytes to base64url (no padding). */
export function toB64url(value: ArrayBuffer | Uint8Array): string {
  const bytes = value instanceof Uint8Array ? value : new Uint8Array(value);
  let bin = '';
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

type Json = Record<string, unknown>;

const withId = (c: Json) => ({ ...c, id: fromB64url(String(c.id)) });

/** `{ publicKey: … }` from Sesame to the argument of `navigator.credentials.create()`. */
export function creationOptions(options: Json): CredentialCreationOptions {
  const pk = (options.publicKey ?? options) as Json;
  const user = pk.user as Json;
  return {
    publicKey: {
      ...(pk as object),
      challenge: fromB64url(String(pk.challenge)),
      user: { ...user, id: fromB64url(String(user.id)) },
      excludeCredentials: ((pk.excludeCredentials as Json[] | undefined) ?? []).map(withId),
    } as unknown as PublicKeyCredentialCreationOptions,
  };
}

/**
 * `{ publicKey: …, mediation? }` from Sesame to the argument of `navigator.credentials.get()`. `conditional`: keep
 * Sesame's `mediation: conditional` (autofill on the email field); otherwise a modal prompt.
 */
export function requestOptions(
  options: Json,
  conditional: boolean,
  signal?: AbortSignal,
): CredentialRequestOptions {
  const pk = (options.publicKey ?? options) as Json;
  const out: CredentialRequestOptions = {
    publicKey: {
      ...(pk as object),
      challenge: fromB64url(String(pk.challenge)),
      allowCredentials: ((pk.allowCredentials as Json[] | undefined) ?? []).map(withId),
    } as unknown as PublicKeyCredentialRequestOptions,
  };
  if (conditional) (out as Json).mediation = 'conditional';
  if (signal) out.signal = signal;
  return out;
}

interface CredentialLike {
  id: string;
  rawId: ArrayBuffer;
  type: string;
  response: unknown;
  getClientExtensionResults?: () => unknown;
}

/** The browser's registration result as Sesame expects it. */
export function registrationJSON(cred: CredentialLike): Json {
  const r = cred.response as {
    attestationObject: ArrayBuffer;
    clientDataJSON: ArrayBuffer;
    getTransports?: () => string[];
  };
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response: {
      attestationObject: toB64url(r.attestationObject),
      clientDataJSON: toB64url(r.clientDataJSON),
      transports: r.getTransports?.() ?? undefined,
    },
    extensions: cred.getClientExtensionResults?.() ?? {},
  };
}

/** The browser's assertion as Sesame expects it. */
export function assertionJSON(cred: CredentialLike): Json {
  const r = cred.response as {
    authenticatorData: ArrayBuffer;
    clientDataJSON: ArrayBuffer;
    signature: ArrayBuffer;
    userHandle: ArrayBuffer | null;
  };
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response: {
      authenticatorData: toB64url(r.authenticatorData),
      clientDataJSON: toB64url(r.clientDataJSON),
      signature: toB64url(r.signature),
      userHandle: r.userHandle ? toB64url(r.userHandle) : null,
    },
    extensions: cred.getClientExtensionResults?.() ?? {},
  };
}

/** Whether this browser can use passkeys at all. */
export function passkeysSupported(): boolean {
  return typeof window !== 'undefined' && typeof window.PublicKeyCredential === 'function';
}

/** Whether this browser offers passkeys as autofill (conditional mediation). */
export async function conditionalAvailable(): Promise<boolean> {
  if (!passkeysSupported()) return false;
  const pkc = window.PublicKeyCredential as unknown as {
    isConditionalMediationAvailable?: () => Promise<boolean>;
  };
  try {
    return (await pkc.isConditionalMediationAvailable?.()) ?? false;
  } catch {
    return false;
  }
}

/** A user who dismissed the browser's passkey prompt is not an error worth showing. */
export function cancelled(e: unknown): boolean {
  const name = (e as { name?: string })?.name ?? '';
  return name === 'NotAllowedError' || name === 'AbortError';
}
