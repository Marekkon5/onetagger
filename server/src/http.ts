import type { SearchQuery } from './domain.js';
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
  console.error('Unhandled request failure');
  return fallback;
}
export function validateSearchQuery(input: unknown): SearchQuery {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new HttpInputError('A discovery query object is required',400);
  const query=input as Record<string,unknown>,keys=['q','url','artist','crew','minDurationMs','durationExpectedMs','maxDepth','maxItems','limit'];
  if (Object.keys(query).some(key=>!keys.includes(key))) throw new HttpInputError('Discovery query contains unsupported controls',400);
  for(const field of ['q','url','artist','crew']) if(query[field]!==undefined && (typeof query[field]!=='string' || !(query[field] as string).trim() || (query[field] as string).length>(field==='url'?2048:500))) throw new HttpInputError(`Discovery ${field} must be bounded text`,400);
  if(query.url!==undefined){let url:URL;try{url=new URL(query.url as string);}catch{throw new HttpInputError('Discovery source URL is invalid',400);}if(!['https:','http:'].includes(url.protocol)||url.username||url.password||url.port)throw new HttpInputError('Discovery source URL is invalid',400);}
  for(const [field,max,min] of [['minDurationMs',86400000,1],['durationExpectedMs',86400000,1],['maxDepth',6,0],['maxItems',2500,1],['limit',150,1]] as const) {
    if(query[field]!==undefined&&(!Number.isInteger(query[field])||(query[field] as number)<min||(query[field] as number)>max))throw new HttpInputError(`Discovery ${field} is outside supported bounds`,400);
  }
  if(!query.q&&!query.url&&!query.artist&&!query.crew)throw new HttpInputError('Enter a discovery query or public source URL',400);
  return query as SearchQuery;
}
