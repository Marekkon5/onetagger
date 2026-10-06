import { createHmac, timingSafeEqual } from 'node:crypto';
import type { IncomingMessage } from 'node:http';

export const WORKER_SCOPE = 'import_worker';
export const WORKER_TIMESTAMP_TOLERANCE_MS = 15 * 60 * 1000;

export class WorkerAuthError extends Error {
  constructor(message: string, readonly statusCode = 401) {
    super(message);
  }
}

function secret(): string {
  return process.env.IMPORT_WORKER_SECRET ?? '';
}

function safeEqualHex(a: string, b: string): boolean {
  const aBuf = Buffer.from(a, 'utf8');
  const bBuf = Buffer.from(b, 'utf8');
  if (aBuf.length !== bBuf.length) return false;
  return timingSafeEqual(aBuf, bBuf);
}

/** Canonical HMAC-SHA256 over timestamp + method + path + body. */
export function signWorkerRequest(secretValue: string, timestamp: string, method: string, path: string, body: string): string {
  return createHmac('sha256', secretValue).update(`${timestamp}\n${method}\n${path}\n${body}`, 'utf8').digest('hex');
}

async function readBodyText(request: IncomingMessage): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of request) chunks.push(Buffer.from(chunk as Uint8Array));
  return Buffer.concat(chunks).toString('utf8');
}

function header(request: IncomingMessage, name: string): string | undefined {
  const value = request.headers[name.toLowerCase()];
  if (Array.isArray(value)) return value[0];
  return value ?? undefined;
}

/**
 * HMAC-SHA256 worker auth. Headers:
 * x-import-worker-timestamp (unix seconds or ISO), x-import-worker-signature (hex),
 * x-import-worker-scope (must be import_worker).
 */
export async function authenticateWorker(request: IncomingMessage): Promise<{ timestamp: string }> {
  const configured = secret();
  if (!configured) throw new WorkerAuthError('Worker authentication is not configured', 503);
  const timestampHeader = header(request, 'x-import-worker-timestamp');
  const signature = header(request, 'x-import-worker-signature');
  const scope = header(request, 'x-import-worker-scope') ?? WORKER_SCOPE;
  if (!timestampHeader || !signature) throw new WorkerAuthError('Worker authentication is required', 401);
  if (scope !== WORKER_SCOPE) throw new WorkerAuthError('Worker scope is invalid', 401);

  const timestampMs = /^\d+$/.test(timestampHeader) ? Number(timestampHeader) * 1000 : Date.parse(timestampHeader);
  if (!Number.isFinite(timestampMs)) throw new WorkerAuthError('Worker timestamp is invalid', 401);
  if (Math.abs(Date.now() - timestampMs) > WORKER_TIMESTAMP_TOLERANCE_MS) {
    throw new WorkerAuthError('Worker timestamp is outside tolerance', 401);
  }

  const method = (request.method ?? 'GET').toUpperCase();
  const path = (request.url ?? '/').split('?')[0];
  const bodyText = await readBodyText(request);
  // Stash raw body so JSON route handlers can reuse it without re-reading.
  (request as IncomingMessage & { __workerBodyText?: string }).__workerBodyText = bodyText;
  const expected = signWorkerRequest(configured, timestampHeader, method, path, bodyText);
  if (!safeEqualHex(signature, expected)) throw new WorkerAuthError('Worker signature is invalid', 401);
  return { timestamp: timestampHeader };
}

export function workerBodyText(request: IncomingMessage): string | undefined {
  return (request as IncomingMessage & { __workerBodyText?: string }).__workerBodyText;
}
