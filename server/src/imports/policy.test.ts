import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveImportGate } from './policy.js';

const hash = 'scrypt$16384$8$1$c2FsdA==$aGFzaA==';

test('imports stay dark under AUTH_MODE=off even with a configured curator hash', () => {
  assert.equal(resolveImportGate({ requested: 'true', passwordHash: hash, authMode: 'off' }), false);
});

test('imports require an explicit request flag, a real hash and AUTH_MODE=on', () => {
  assert.equal(resolveImportGate({ requested: 'true', passwordHash: hash, authMode: 'on' }), true);
  assert.equal(resolveImportGate({ requested: 'on', passwordHash: hash, authMode: 'on' }), true);
  assert.equal(resolveImportGate({ requested: 'false', passwordHash: hash, authMode: 'on' }), false);
  assert.equal(resolveImportGate({ requested: undefined, passwordHash: hash, authMode: 'on' }), false);
  assert.equal(resolveImportGate({ requested: 'true', passwordHash: '', authMode: 'on' }), false);
  assert.equal(resolveImportGate({ requested: 'true', passwordHash: 'not-a-hash', authMode: 'on' }), false);
});
