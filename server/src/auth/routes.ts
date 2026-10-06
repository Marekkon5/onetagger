import type { IncomingMessage, ServerResponse } from 'node:http';
import { AuthError, type CuratorActor } from './curator.js';
import type { createCuratorAuth } from './curator.js';
import { applyCorsHeaders, HttpInputError, isAllowedOrigin, publicErrorMessage, readJsonBody, sendJson } from '../http.js';

export interface AuthRouteContext {
  auth: ReturnType<typeof createCuratorAuth>;
}

export async function handleAuthRoute(context: AuthRouteContext, request: IncomingMessage, response: ServerResponse): Promise<boolean> {
  applyCorsHeaders(request, response);
  const path = request.url?.split('?')[0];
  if (request.method === 'GET' && path === '/api/auth/session') {
    const actor: CuratorActor | undefined = context.auth.authenticate(request);
    sendJson(response, 200, { authenticated: Boolean(actor) });
    return true;
  }

  if (request.method === 'POST' && path === '/api/auth/login') {
    if (!isAllowedOrigin(request)) {
      sendJson(response, 403, { error: 'Origin is not allowed' });
      return true;
    }
    try {
      const input = await readJsonBody(request);
      if (!input || typeof input !== 'object' || typeof (input as { password?: unknown }).password !== 'string') {
        sendJson(response, 400, { error: 'Password is required' });
        return true;
      }
      const setCookie = context.auth.login((input as { password: string }).password, request.socket.remoteAddress ?? 'unknown');
      sendJson(response, 200, { authenticated: true }, { 'set-cookie': setCookie });
    } catch (error) {
      const status = error instanceof HttpInputError || error instanceof AuthError ? error.statusCode : 500;
      sendJson(response, status, { error: publicErrorMessage(error, 'Login failed') });
    }
    return true;
  }

  if (request.method === 'POST' && path === '/api/auth/logout') {
    if (!isAllowedOrigin(request)) {
      sendJson(response, 403, { error: 'Origin is not allowed' });
      return true;
    }
    sendJson(response, 200, { authenticated: false }, { 'set-cookie': context.auth.logout(request) });
    return true;
  }

  return false;
}