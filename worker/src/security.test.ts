import assert from 'node:assert/strict';
import test from 'node:test';
import { assertSafeUrl, ipBlocked } from './security.js';

test('loopback, RFC1918, link-local and metadata addresses are blocked', () => {
  for (const ip of ['127.0.0.1', '10.0.0.1', '172.16.5.4', '192.168.1.1', '169.254.169.254', '::1', 'fe80::1', 'fc00::1']) {
    assert.equal(ipBlocked(ip), true, ip);
  }
  for (const ip of ['8.8.8.8', '1.1.1.1', '93.184.216.34', '2001:4860:4860::8888']) {
    assert.equal(ipBlocked(ip), false, ip);
  }
});

test('URLs with credentials, non-default ports or localhost are rejected', () => {
  assert.throws(() => assertSafeUrl('http://localhost/track.mp3'), /blocked/);
  assert.throws(() => assertSafeUrl('https://127.0.0.1/track.mp3'), /blocked/);
  assert.throws(() => assertSafeUrl('https://user:pass@example.com/a.mp3'), /Credentials/);
  assert.throws(() => assertSafeUrl('https://example.com:8080/a.mp3'), /ports/);
  assert.throws(() => assertSafeUrl('ftp://example.com/a.mp3'), /http/);
  assert.throws(() => assertSafeUrl('file:///etc/passwd'), /http/);
  const ok = assertSafeUrl('https://archive.example.org/mixes/set.mp3');
  assert.equal(ok.hostname, 'archive.example.org');
});
