import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import type { IncomingMessage } from 'node:http';
import { openCatalog } from '../catalog/repository.js';
import { createCuratorAuth, hashCuratorPassword } from './curator.js';

function withCatalog(run: (path: string, repo: ReturnType<typeof openCatalog>) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-auth-'));
  const path = join(directory, 'catalog.sqlite');
  const repo = openCatalog(path);
  try {
    run(path, repo);
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
}

test('curator sessions use salted password hashes, HTTP-only cookies and hashed session tokens', () => {
  withCatalog((path, repo) => {
    const hash = hashCuratorPassword('local-test-password');
    assert.equal(hash.includes('local-test-password'), false);
    const auth = createCuratorAuth(repo, hash);
    const cookie = auth.login('local-test-password', '127.0.0.1');
    assert.match(cookie, /HttpOnly/i);
    assert.match(cookie, /SameSite=Lax/i);
    assert.match(cookie, /Path=\//i);
    const token = cookie.match(/syco23_curator=([^;]+)/)?.[1];
    assert.ok(token);
    const actor = auth.authenticate({ headers: { cookie: `syco23_curator=${token}` } } as IncomingMessage);
    assert.ok(actor?.sessionId);

    const database = new DatabaseSync(path);
    const session = database.prepare('SELECT token_hash FROM curator_sessions').get() as { token_hash: string };
    assert.notEqual(session.token_hash, token);
    assert.equal(session.token_hash, createHash('sha256').update(token).digest('hex'));
    database.close();
  });
});

test('failed curator logins are rate limited before another password check', () => {
  withCatalog((_path, repo) => {
    const auth = createCuratorAuth(repo, hashCuratorPassword('correct-password'));
    for (let attempt = 0; attempt < 5; attempt += 1) {
      assert.throws(() => auth.login('wrong-password', '192.0.2.10'), /invalid|rate/i);
    }
    assert.throws(() => auth.login('correct-password', '192.0.2.10'), (error: unknown) => {
      return error instanceof Error && 'statusCode' in error && error.statusCode === 429;
    });
  });
});

test('logout revokes the session and clears the cookie', () => {
  withCatalog((_path, repo) => {
    const auth = createCuratorAuth(repo, hashCuratorPassword('local-test-password'));
    const cookie = auth.login('local-test-password', '127.0.0.1');
    const request = { headers: { cookie: cookie.split(';')[0] } } as IncomingMessage;
    assert.ok(auth.authenticate(request));
    const cleared = auth.logout(request);
    assert.match(cleared, /Max-Age=0/i);
    assert.equal(auth.authenticate(request), undefined);
  });
});