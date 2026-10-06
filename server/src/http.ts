import type { IncomingMessage, ServerResponse } from 'node:http';

export class HttpInputError extends Error {
  constructor(message: string, readonly statusCode: number) {
    super(message);
  }
}

export function isAllowedOrigin(request: IncomingMessage): boolean {
  const origin = request.headers.origin;
  if (!origin) return false;
  const configured = (process.env.CORS_ORIGIN ?? '')
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean);
  if (process.env.NODE_ENV !== 'production') {
    configured.push('http://localhost:5173', 'http://127.0.0.1:5173');
  }
  return configured.includes(origin);
}

export function applyCorsHeaders(request: IncomingMessage, response: ServerResponse): void {
  const origin = request.headers.origin;
  if (!origin || !isAllowedOrigin(request)) return;
  response.setHeader('access-control-allow-origin', origin);
  response.setHeader('access-control-allow-credentials', 'true');
  response.setHeader('access-control-allow-headers', 'content-type');
  response.setHeader('access-control-allow-methods', 'GET,POST,PATCH,PUT,OPTIONS');
  response.setHeader('vary', 'Origin');
}

export async function readJsonBody(request: IncomingMessage, maxBytes = 2 * 1024 * 1024): Promise<unknown> {
  const bytes = await readRequestBytes(request, maxBytes);
  if (!bytes.length) return {};
  try {
    return JSON.parse(bytes.toString('utf8')) as unknown;
  } catch {
    throw new HttpInputError('Request body must be valid JSON', 400);
  }
}

export async function readRequestBytes(request: IncomingMessage, maxBytes: number): Promise<Buffer> {
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of request) {
    const buffer = Buffer.from(chunk);
    size += buffer.length;
    if (size > maxBytes) throw new HttpInputError('Request body exceeds the configured size limit', 413);
    chunks.push(buffer);
  }
  return Buffer.concat(chunks);
}

export function sendJson(response: ServerResponse, status: number, body: unknown, headers: Record<string, string> = {}): void {
  const payload = status === 204 ? '' : JSON.stringify(body);
  response.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    ...(status === 204 ? {} : { 'content-length': Buffer.byteLength(payload) }),
    ...headers
  });
  response.end(payload);
}

/**
 * Errors that are safe to describe to a client. Anything else is logged
 * server-side and reported as a generic failure, so internal details such as
 * file paths, SQL fragments or dependency messages never reach a response body.
 */
export function publicErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof HttpInputError) return error.message;
  if (error instanceof Error && error.name === 'ZodError') return 'Input validation failed';
  if (error && typeof error === 'object' && 'statusCode' in error) {
    const statusCode = (error as { statusCode: unknown }).statusCode;
    if (typeof statusCode === 'number' && statusCode >= 400 && statusCode < 500) {
      return error instanceof Error ? error.message : fallback;
    }
  }
  console.error('Unhandled request failure:', error);
  return fallback;
}