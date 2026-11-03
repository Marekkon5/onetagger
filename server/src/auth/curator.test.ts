import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import type { IncomingMessage } from 'node:http';
import { openCatalog } from '../catalog/repository.js';
import { createCuratorAuth, hashCuratorPassword, isCuratorAuthDisabled } from './curator.js';

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
test('cross-site approved frontend sessions use secure cookies that browsers can send',()=>{
 const oldEnv=process.env.NODE_ENV;const oldSite=process.env.CURATOR_COOKIE_SAME_SITE;process.env.NODE_ENV='production';process.env.CURATOR_COOKIE_SAME_SITE='none';
 try{withCatalog((_path,repo)=>{const cookie=createCuratorAuth(repo,hashCuratorPassword('local-test-password')).login('local-test-password');assert.match(cookie,/SameSite=None/);assert.match(cookie,/Secure/);});}finally{if(oldEnv===undefined)delete process.env.NODE_ENV;else process.env.NODE_ENV=oldEnv;if(oldSite===undefined)delete process.env.CURATOR_COOKIE_SAME_SITE;else process.env.CURATOR_COOKIE_SAME_SITE=oldSite;}
});

test('local single-user mode authenticates every request without a session cookie',()=>{
  const oldDisabled=process.env.CURATOR_AUTH_DISABLED;const oldEnv=process.env.NODE_ENV;
  process.env.CURATOR_AUTH_DISABLED='true';delete process.env.NODE_ENV;
  try{
    assert.equal(isCuratorAuthDisabled(),true);
    withCatalog((_path,repo)=>{
      // No password hash is configured at all, yet a bare request authenticates.
      const auth=createCuratorAuth(repo,'');
      assert.equal(auth.disabled,true);
      assert.equal(auth.mode,'off');
      assert.equal(createCuratorAuth(repo,'','on').mode,'off');
      const bare={headers:{}} as IncomingMessage;
      assert.deepEqual(auth.authenticate(bare),{sessionId:'auth-off'});
      assert.ok(auth.authenticate({headers:{cookie:'syco23_curator=nonsense'}} as IncomingMessage));
      assert.match(auth.logout(bare),/Max-Age=0/i);
    });
  }finally{
    if(oldDisabled===undefined)delete process.env.CURATOR_AUTH_DISABLED;else process.env.CURATOR_AUTH_DISABLED=oldDisabled;
    if(oldEnv===undefined)delete process.env.NODE_ENV;else process.env.NODE_ENV=oldEnv;
  }
});

test('local single-user mode is ignored in production so a deployed worker stays protected',()=>{
  const oldDisabled=process.env.CURATOR_AUTH_DISABLED;const oldEnv=process.env.NODE_ENV;
  process.env.CURATOR_AUTH_DISABLED='true';process.env.NODE_ENV='production';
  try{
    assert.equal(isCuratorAuthDisabled(),false);
    withCatalog((_path,repo)=>{
      const auth=createCuratorAuth(repo,hashCuratorPassword('local-test-password'));
      assert.equal(auth.disabled,false);
      assert.equal(auth.authenticate({headers:{}} as IncomingMessage),undefined);
      // Real credential checking still applies.
      assert.throws(()=>auth.login('wrong-password','192.0.2.11'),/invalid|rate/i);
    });
  }finally{
    if(oldDisabled===undefined)delete process.env.CURATOR_AUTH_DISABLED;else process.env.CURATOR_AUTH_DISABLED=oldDisabled;
    if(oldEnv===undefined)delete process.env.NODE_ENV;else process.env.NODE_ENV=oldEnv;
  }
});

test('authentication stays enabled when the local single-user switch is unset or unrecognised',()=>{
  const oldDisabled=process.env.CURATOR_AUTH_DISABLED;const oldEnv=process.env.NODE_ENV;
  delete process.env.NODE_ENV;
  try{
    for(const value of [undefined,'','false','0','no','maybe']){
      if(value===undefined)delete process.env.CURATOR_AUTH_DISABLED;else process.env.CURATOR_AUTH_DISABLED=value;
      assert.equal(isCuratorAuthDisabled(),false,`expected auth to stay on for ${String(value)}`);
    }
  }finally{
    if(oldDisabled===undefined)delete process.env.CURATOR_AUTH_DISABLED;else process.env.CURATOR_AUTH_DISABLED=oldDisabled;
    if(oldEnv===undefined)delete process.env.NODE_ENV;else process.env.NODE_ENV=oldEnv;
  }
});

test('explicit off mode permits anonymous curator access and on remains the default', () => {
  withCatalog((_path, repo) => {
    const request = { headers: {} } as IncomingMessage;
    const protectedAuth = createCuratorAuth(repo, '', 'on');
    assert.equal(protectedAuth.mode, 'on');
    assert.equal(protectedAuth.authenticate(request), undefined);
    const open = createCuratorAuth(repo, '', 'off');
    assert.equal(open.disabled, true);
    assert.deepEqual(open.authenticate(request), { sessionId: 'auth-off' });
    assert.match(open.logout(request), /Max-Age=0/);
    assert.deepEqual(open.authenticate(request), { sessionId: 'auth-off' });
    assert.throws(() => createCuratorAuth(repo, '', 'false'), /AUTH_MODE/);
  });
});
