import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable } from 'node:stream';
import type { IncomingMessage, ServerResponse } from 'node:http';
import test from 'node:test';
import { openCatalog } from '../catalog/repository.js';
import { createCuratorAuth, hashCuratorPassword } from './curator.js';
import { handleAuthRoute } from './routes.js';
import { isAllowedOrigin } from '../http.js';

function mockRequest(method: string, url: string, value?: unknown, headers: Record<string, string> = {}) {
  const input = value === undefined ? [] : [JSON.stringify(value)];
  const request = Readable.from(input) as IncomingMessage;
  request.method = method;
  request.url = url;
  request.headers = headers;
  Object.defineProperty(request, 'socket', { value: { remoteAddress: '127.0.0.1' } });
  return request;
}

function mockResponse() {
  const captured: { status?: number; headers?: Record<string, string>; body?: string } = {};
  const response = {
    setHeader(name: string, value: string) { captured.headers = { ...captured.headers, [name]: value }; },
    writeHead(status: number, headers: Record<string, string>) { captured.status = status; captured.headers = headers; return this; },
    end(body = '') { captured.body = body; }
  } as unknown as ServerResponse;
  return { response, captured };
}

test('auth routes reject unapproved origins and return a cookie, never a session token body', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-auth-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const priorOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'https://mixsets.example.org';
  try {
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const denied = mockResponse();
    await handleAuthRoute({ auth }, mockRequest('POST', '/api/auth/login', { password: 'private-local-password' }, { origin: 'https://attacker.example' }), denied.response);
    assert.equal(denied.captured.status, 403);

    const accepted = mockResponse();
    await handleAuthRoute({ auth }, mockRequest('POST', '/api/auth/login', { password: 'private-local-password' }, { origin: 'https://mixsets.example.org' }), accepted.response);
    assert.equal(accepted.captured.status, 200);
    assert.match(accepted.captured.headers?.['set-cookie'] ?? '', /HttpOnly/i);
    assert.equal(JSON.parse(accepted.captured.body ?? '{}').token, undefined);
  } finally {
    if (priorOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = priorOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('auth route input has a hard byte limit', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-auth-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const priorOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'http://localhost:5173';
  try {
    const auth = createCuratorAuth(repo, hashCuratorPassword('private-local-password'));
    const response = mockResponse();
    await handleAuthRoute({ auth }, mockRequest('POST', '/api/auth/login', { password: 'x'.repeat(2 * 1024 * 1024 + 1) }, { origin: 'http://localhost:5173' }), response.response);
    assert.equal(response.captured.status, 413);
  } finally {
    if (priorOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = priorOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('login failure responses never expose internal error detail', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-auth-routes-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  const priorOrigin = process.env.CORS_ORIGIN;
  process.env.CORS_ORIGIN = 'https://mixsets.example.org';
  try {
    // An authentication backend that throws must not leak its message or stack.
    const auth = {
      login(): string { throw new Error('Internal server error'); },
      authenticate() { return undefined; },
      logout() { return ''; }
    } as unknown as ReturnType<typeof createCuratorAuth>;
    const response = mockResponse();
    await handleAuthRoute({ auth }, mockRequest('POST', '/api/auth/login', { password: 'private-local-password' }, { origin: 'https://mixsets.example.org' }), response.response);
    assert.equal(response.captured.status, 500);
    const body = JSON.parse(response.captured.body ?? '{}') as { error?: string };
    assert.equal(body.error, 'Login failed');
    assert.doesNotMatch(response.captured.body ?? '', /ENOENT|db\.internal|\/srv\//);
  } finally {
    if (priorOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = priorOrigin;
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('development defaults allow only the two local Vite origins', () => {
  const previousOrigin = process.env.CORS_ORIGIN;
  const previousNodeEnv = process.env.NODE_ENV;
  delete process.env.CORS_ORIGIN;
  process.env.NODE_ENV = 'development';
  try {
    expectAllowed('http://localhost:5173', true);
    expectAllowed('http://127.0.0.1:5173', true);
    expectAllowed('https://attacker.example', false);
    process.env.NODE_ENV = 'production';
    expectAllowed('http://localhost:5173', false);
  } finally {
    if (previousOrigin === undefined) delete process.env.CORS_ORIGIN;
    else process.env.CORS_ORIGIN = previousOrigin;
    if (previousNodeEnv === undefined) delete process.env.NODE_ENV;
    else process.env.NODE_ENV = previousNodeEnv;
  }
});

function expectAllowed(origin: string, expected: boolean) {
  const request = { headers: { origin } } as IncomingMessage;
  assert.equal(isAllowedOrigin(request), expected);
}