// node --experimental-strip-types --test src/lib/webauthn.test.mjs   (pnpm --filter @sesame/auth test)
import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  assertionJSON,
  cancelled,
  creationOptions,
  fromB64url,
  registrationJSON,
  requestOptions,
  toB64url,
} from './webauthn.ts';

const bytes = (...b) => new Uint8Array(b);

test('base64url round-trips every byte value, without padding', () => {
  const all = new Uint8Array(256).map((_, i) => i);
  const enc = toB64url(all);
  assert.doesNotMatch(enc, /[+/=]/);
  assert.deepEqual(fromB64url(enc), all);
  assert.equal(toB64url(bytes(0xfb, 0xff)), '-_8');
  assert.deepEqual(fromB64url('-_8'), bytes(0xfb, 0xff));
});

test('creation options from Sesame become ArrayBuffers where the browser needs them', () => {
  const opts = creationOptions({
    publicKey: {
      rp: { id: 'pp.pricewhisperer.ai', name: 'PriceWhisperer' },
      user: { id: toB64url(bytes(1, 2, 3)), name: 'a@b.co', displayName: 'a@b.co' },
      challenge: toB64url(bytes(9, 9)),
      pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
      excludeCredentials: [{ type: 'public-key', id: toB64url(bytes(7)) }],
      authenticatorSelection: { residentKey: 'preferred', userVerification: 'required' },
    },
  });
  assert.deepEqual(opts.publicKey.challenge, bytes(9, 9));
  assert.deepEqual(opts.publicKey.user.id, bytes(1, 2, 3));
  assert.deepEqual(opts.publicKey.excludeCredentials[0].id, bytes(7));
  assert.equal(opts.publicKey.rp.id, 'pp.pricewhisperer.ai');
  assert.equal(opts.publicKey.authenticatorSelection.userVerification, 'required');
});

test('request options: conditional only when asked, allow-list decoded', () => {
  const sesame = {
    publicKey: { challenge: toB64url(bytes(5)), allowCredentials: [{ type: 'public-key', id: toB64url(bytes(8)) }] },
    mediation: 'conditional',
  };
  const modal = requestOptions(sesame, false);
  assert.equal(modal.mediation, undefined);
  assert.deepEqual(modal.publicKey.allowCredentials[0].id, bytes(8));
  const auto = requestOptions(sesame, true);
  assert.equal(auto.mediation, 'conditional');
});

test('responses go back base64url-encoded', () => {
  const reg = registrationJSON({
    id: 'abc',
    rawId: bytes(1, 2).buffer,
    type: 'public-key',
    response: { attestationObject: bytes(3).buffer, clientDataJSON: bytes(4).buffer, getTransports: () => ['internal'] },
    getClientExtensionResults: () => ({ credProps: { rk: true } }),
  });
  assert.deepEqual(reg, {
    id: 'abc',
    rawId: 'AQI',
    type: 'public-key',
    response: { attestationObject: 'Aw', clientDataJSON: 'BA', transports: ['internal'] },
    extensions: { credProps: { rk: true } },
  });
  const asn = assertionJSON({
    id: 'abc',
    rawId: bytes(1).buffer,
    type: 'public-key',
    response: {
      authenticatorData: bytes(2).buffer,
      clientDataJSON: bytes(3).buffer,
      signature: bytes(4).buffer,
      userHandle: null,
    },
  });
  assert.equal(asn.response.userHandle, null);
  assert.equal(asn.response.signature, 'BA');
  assert.deepEqual(asn.extensions, {});
});

test('a dismissed prompt is a cancellation, not an error', () => {
  assert.equal(cancelled({ name: 'NotAllowedError' }), true);
  assert.equal(cancelled({ name: 'AbortError' }), true);
  assert.equal(cancelled(new Error('boom')), false);
});
