// node --experimental-strip-types --test src/lib/oidcRequest.test.mjs   (pnpm --filter @sesame/auth test)
import assert from 'node:assert/strict';
import { test } from 'node:test';

import { decide, fetchAuthorizeStatus, recheckDelayMs, restartTarget } from './oidcRequest.ts';

const START = 'https://trader.pp.pricewhisperer.ai/auth/start';

const reply = (status, body) => async () =>
  new Response(typeof body === 'string' ? body : JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });

test('an active request is kept and re-checked when it is due to expire', () => {
  assert.deepEqual(decide({ active: true, expires_in: 1799, initiate_login_uri: START }), {
    kind: 'ok',
    recheckInMs: 1_800_000,
  });
});

test('an expired request restarts at the registered sign-in start', () => {
  assert.deepEqual(decide({ active: false, expires_in: 0, initiate_login_uri: START }), {
    kind: 'restart',
    to: START,
  });
});

test('without a registered sign-in start the page keeps its message', () => {
  assert.deepEqual(decide({ active: false, expires_in: 0, initiate_login_uri: null }), { kind: 'expired' });
});

test('an unanswered check changes nothing', () => {
  assert.deepEqual(decide(null), { kind: 'unknown' });
});

test('only absolute https (or loopback http) restart targets are followed', () => {
  assert.equal(restartTarget(START), START);
  assert.equal(restartTarget('http://localhost:3000/auth/start'), 'http://localhost:3000/auth/start');
  for (const bad of [
    'javascript:alert(1)',
    '/auth/start',
    '//evil.example/auth/start',
    'http://trader.example/auth/start',
    'data:text/html,hi',
    '',
    null,
    undefined,
  ]) {
    assert.equal(restartTarget(bad), null, String(bad));
    assert.equal(decide({ active: false, expires_in: 0, initiate_login_uri: bad ?? null }).kind, 'expired');
  }
});

test('re-check delay is bounded', () => {
  assert.equal(recheckDelayMs(0), 1000);
  assert.equal(recheckDelayMs(-5), 1000);
  assert.equal(recheckDelayMs(99_999), 1_801_000);
});

test('status is read from the same-origin endpoint with both ids', async () => {
  let seen;
  const status = await fetchAuthorizeStatus('req id', 'ses_abc', async (url, init) => {
    seen = { url, init };
    return reply(200, { active: false, expires_in: 0, initiate_login_uri: START })();
  });
  assert.equal(seen.url, '/oauth/authorize/status?request_id=req+id&client_id=ses_abc');
  assert.equal(seen.init.cache, 'no-store');
  assert.deepEqual(status, { active: false, expires_in: 0, initiate_login_uri: START });
});

test('errors and malformed answers read as unknown', async () => {
  assert.equal(await fetchAuthorizeStatus('r', 'c', reply(503, { error: 'temporarily_unavailable' })), null);
  assert.equal(await fetchAuthorizeStatus('r', 'c', reply(200, 'not json')), null);
  assert.equal(await fetchAuthorizeStatus('r', 'c', reply(200, { expires_in: 3 })), null);
  assert.equal(
    await fetchAuthorizeStatus('r', 'c', async () => {
      throw new TypeError('network');
    }),
    null,
  );
});
