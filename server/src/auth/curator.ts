import { createHash, randomBytes, randomUUID, scryptSync, timingSafeEqual } from 'node:crypto';
import type { IncomingMessage } from 'node:http';
import type { CatalogRepository } from '../catalog/repository.js';

const cookieName = 'syco23_curator';
const sessionDurationMs = 12 * 60 * 60 * 1000;
const attemptWindowMs = 15 * 60 * 1000;
const maxAttempts = 5;

export interface CuratorActor {
  sessionId: string;
}

export class AuthError extends Error {
  constructor(message: string, readonly statusCode: number) {
    super(message);
  }
}

export function hashCuratorPassword(password: string): string {
  if (password.length < 12 || password.length > 1024) throw new Error('Curator password must be between 12 and 1024 characters');
  const salt = randomBytes(16);
  const derivedKey = scryptSync(password, salt, 64, { N: 16384, r: 8, p: 1, maxmem: 64 * 1024 * 1024 });
  return `scrypt$${salt.toString('base64url')}$${derivedKey.toString('base64url')}`;
}

function verifyPassword(password: string, passwordHash: string): boolean {
  const [algorithm, encodedSalt, encodedKey] = passwordHash.split('$');
  if (algorithm !== 'scrypt' || !encodedSalt || !encodedKey) return false;
  const salt = Buffer.from(encodedSalt, 'base64url');
  const expected = Buffer.from(encodedKey, 'base64url');
  if (salt.length !== 16 || expected.length !== 64) return false;
  const actual = scryptSync(password, salt, expected.length, { N: 16384, r: 8, p: 1, maxmem: 64 * 1024 * 1024 });
  return timingSafeEqual(actual, expected);
}

function tokenHash(token: string): string {
  return createHash('sha256').update(token).digest('hex');
}

function requestCookie(request: IncomingMessage): string | undefined {
  const cookieHeader = request.headers.cookie;
  const cookies = Array.isArray(cookieHeader) ? cookieHeader.join(';') : cookieHeader;
  const token = cookies?.split(';').map((part) => part.trim()).find((part) => part.startsWith(`${cookieName}=`))?.slice(cookieName.length + 1);
  return token || undefined;
}

function cookie(value: string, maxAge: number): string {
  const production = process.env.NODE_ENV === 'production';
  const sameSite = production && process.env.CURATOR_COOKIE_SAME_SITE === 'none' ? 'None' : 'Lax';
  const secure = production ? '; Secure' : '';
  return `${cookieName}=${value}; Path=/; HttpOnly; SameSite=${sameSite}; Max-Age=${maxAge}${secure}`;
}

/** Local-only compatibility switch; deployed open access requires AUTH_MODE=off. */
export function isCuratorAuthDisabled(): boolean {
  if (process.env.NODE_ENV === 'production') return false;
  return /^(1|true|yes|on)$/i.test((process.env.CURATOR_AUTH_DISABLED ?? '').trim());
}

export type AuthMode = 'on' | 'off';

export function resolveCuratorAuthMode(configuredMode: string | undefined = process.env.AUTH_MODE): AuthMode {
  if (isCuratorAuthDisabled()) return 'off';
  if (configuredMode !== undefined) {
    if (configuredMode !== 'on' && configuredMode !== 'off') throw new Error('AUTH_MODE must be on or off');
    return configuredMode;
  }
  return 'on';
}

export function createCuratorAuth(repository: CatalogRepository, passwordHash: string, configuredMode?: string) {
  const mode = resolveCuratorAuthMode(configuredMode);
  const attempts = new Map<string, { count: number; resetAt: number }>();
  const authDisabled = mode === 'off';
  if (authDisabled) console.warn('Curator authentication is disabled: every request is treated as curator.');

  return {
    mode,
    disabled: authDisabled,

    login(password: string, clientAddress = 'unknown'): string {
      if (authDisabled) return cookie('', 0);
      if (!passwordHash.startsWith('scrypt$')) throw new AuthError('Curator authentication is not configured', 503);
      const now = Date.now();
      const entry = attempts.get(clientAddress);
      if (entry && entry.resetAt > now && entry.count >= maxAttempts) throw new AuthError('Too many login attempts', 429);
      if (entry && entry.resetAt <= now) attempts.delete(clientAddress);
      if (!verifyPassword(password, passwordHash)) {
        const current = attempts.get(clientAddress);
        attempts.set(clientAddress, current && current.resetAt > now
          ? { ...current, count: current.count + 1 }
          : { count: 1, resetAt: now + attemptWindowMs });
        throw new AuthError('Invalid curator password', 401);
      }

      attempts.delete(clientAddress);
      const token = randomBytes(32).toString('base64url');
      const createdAt = new Date(now).toISOString();
      repository.createSession({
        id: randomUUID(),
        tokenHash: tokenHash(token),
        createdAt,
        expiresAt: new Date(now + sessionDurationMs).toISOString()
      });
      return cookie(token, Math.floor(sessionDurationMs / 1000));
    },

    authenticate(request: IncomingMessage): CuratorActor | undefined {
      if (authDisabled) return { sessionId: 'auth-off' };
      const token = requestCookie(request);
      if (!token) return undefined;
      const hash = tokenHash(token);
      const session = repository.getSession(hash);
      if (!session) return undefined;
      if (Date.parse(session.expiresAt) <= Date.now()) {
        repository.deleteSession(hash);
        return undefined;
      }
      return { sessionId: session.id };
    },

    logout(request: IncomingMessage): string {
      if (authDisabled) return cookie('', 0);
      const token = requestCookie(request);
      if (token) repository.deleteSession(tokenHash(token));
      return cookie('', 0);
    }
  };
}

export type CuratorAuth = ReturnType<typeof createCuratorAuth>;
