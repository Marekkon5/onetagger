import assert from 'node:assert/strict';
import test from 'node:test';
import { classifyError } from './queue.js';
import { FFmpegTimeoutError, MediaRejectedError } from './ffmpeg.js';
import { NonRetryableError } from './resolvers.js';
import { RetryableError } from './process.js';
import { UpstreamHttpError, UrlBlockedError } from './security.js';
import { signRequest } from './control.js';

test('errors are classified for at-least-once retry', () => {
  assert.equal(classifyError(new NonRetryableError('blocked')).retryable, false);
  assert.equal(classifyError(new UrlBlockedError('private ip')).retryable, false);
  assert.equal(classifyError(new MediaRejectedError('ffmpeg failed')).retryable, false);
  assert.equal(classifyError(new RetryableError('lease lost')).retryable, true);
  assert.equal(classifyError(new FFmpegTimeoutError('ffmpeg exceeded 1000ms')).retryable, true);
  assert.equal(classifyError(new Error('upstream 503')).retryable, true);
  assert.equal(classifyError(new Error('connect ETIMEDOUT')).retryable, true);
});

test('upstream throttling and server errors retry, auth and missing items do not', () => {
  for (const status of [408, 425, 429, 500, 502, 503]) {
    assert.equal(classifyError(new UpstreamHttpError(status)).retryable, true, `status ${status} retries`);
  }
  for (const status of [400, 401, 403, 404, 410]) {
    assert.equal(classifyError(new UpstreamHttpError(status)).retryable, false, `status ${status} fails permanently`);
  }
});

test('worker signatures verify against the API contract', () => {
  const timestamp = '1700000000';
  const signature = signRequest('secret', timestamp, 'GET', '/internal/imports/next', '');
  assert.match(signature, /^[0-9a-f]{64}$/);
  assert.notEqual(signRequest('secret', timestamp, 'GET', '/internal/imports/next', ''), signRequest('other', timestamp, 'GET', '/internal/imports/next', ''));
});
