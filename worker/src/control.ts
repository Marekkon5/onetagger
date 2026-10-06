import { createHmac } from 'node:crypto';
import type { WorkerConfig } from './config.js';

export interface ImportJobSpec {
  id: string;
  requestedBy: string;
  provider: string;
  sourceUrl: string;
  sourceExternalId?: string;
  mode: 'metadata' | 'audio';
  state: string;
  idempotencyKey: string;
  attempt: number;
  createdAt: string;
  updatedAt: string;
}

export interface ImportLimits {
  maxInputBytes: number;
  maxOutputBytes: number;
  maxDurationMs: number;
  maxAttempts: number;
  autoApplyThreshold: number;
}

export interface WorkSpec {
  job: ImportJobSpec;
  limits: ImportLimits;
  capabilities: Record<string, { metadata: boolean; audio: boolean }>;
  /** Private original object key for user uploads. Never a filesystem path. */
  originalObjectKey?: string;
}

export class ControlPlaneError extends Error {
  constructor(message: string, readonly status: number) {
    super(message);
  }
}

/** Same canonical HMAC the API verifies: timestamp \n method \n path \n body. */
export function signRequest(secret: string, timestamp: string, method: string, path: string, body: string): string {
  return createHmac('sha256', secret).update(`${timestamp}\n${method}\n${path}\n${body}`, 'utf8').digest('hex');
}

export class ControlClient {
  constructor(private config: WorkerConfig, private fetchImpl: typeof fetch = fetch) {}

  private async call<T>(method: string, path: string, body?: unknown): Promise<{ status: number; payload: T }> {
    const raw = body === undefined ? '' : JSON.stringify(body);
    const timestamp = String(Math.floor(Date.now() / 1000));
    const response = await this.fetchImpl(`${this.config.apiBase}${path}`, {
      method,
      headers: {
        'content-type': 'application/json',
        'x-import-worker-timestamp': timestamp,
        'x-import-worker-signature': signRequest(this.config.secret, timestamp, method, path, raw),
        'x-import-worker-scope': 'import_worker',
        'x-import-worker-id': this.config.workerId,
      },
      body: raw || undefined,
    });
    const payload = response.status === 204 ? undefined : await response.json().catch(() => undefined) as T;
    return { status: response.status, payload: payload as T };
  }

  /** Lease the next due job. Returns null when the queue is empty. */
  async claimNext(): Promise<WorkSpec | null> {
    const { status, payload } = await this.call<WorkSpec>('GET', '/internal/imports/next');
    if (status === 204) return null;
    if (status !== 200) throw new ControlPlaneError(`claim failed with ${status}`, status);
    return payload;
  }

  async heartbeat(jobId: string): Promise<boolean> {
    const { status } = await this.call('POST', `/internal/imports/${jobId}/heartbeat`, {});
    if (status === 409) return false;
    if (status !== 204 && status !== 200) throw new ControlPlaneError(`heartbeat failed with ${status}`, status);
    return true;
  }

  async event(jobId: string, type: string, extra: Record<string, unknown> = {}): Promise<void> {
    const { status } = await this.call('POST', `/internal/imports/${jobId}/events`, { type, payload: extra, ...extra });
    if (status !== 204 && status !== 200) throw new ControlPlaneError(`event failed with ${status}`, status);
  }

  async registerArtifact(jobId: string, artifact: Record<string, unknown>): Promise<string | undefined> {
    const { status, payload } = await this.call<{ id?: string }>('POST', `/internal/imports/${jobId}/artifacts`, artifact);
    if (status !== 201) throw new ControlPlaneError(`artifact registration failed with ${status}`, status);
    return payload?.id;
  }

  async submitEvidence(jobId: string, scores: unknown[]): Promise<void> {
    const { status } = await this.call('POST', `/internal/imports/${jobId}/evidence`, { scores });
    if (status !== 201) throw new ControlPlaneError(`evidence submit failed with ${status}`, status);
  }

  async complete(jobId: string, outcome: {
    state: 'completed' | 'failed' | 'review';
    attempt: number;
    mixId?: string;
    error?: string;
    errorCode?: string;
    retryable?: boolean;
  }): Promise<void> {
    const { status } = await this.call('POST', `/internal/imports/${jobId}/complete`, outcome);
    if (status !== 200) throw new ControlPlaneError(`complete failed with ${status}`, status);
  }
}
