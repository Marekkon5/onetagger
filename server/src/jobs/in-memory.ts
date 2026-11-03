import { providerFailureMessage } from '../core/utils.js';
import type { MixCandidate, SearchQuery } from '../domain.js';
import type { ProviderRegistry } from '../core/registry.js';

export type JobState = 'queued' | 'running' | 'review' | 'done' | 'error' | 'cancelled';

export interface DiscoveryJob {
  id: string;
  provider: string;
  query: SearchQuery;
  state: JobState;
  progress: number;
  scanned: number;
  found: number;
  createdAt: string;
  updatedAt: string;
  candidates: MixCandidate[];
  error?: string;
}

export class JobQueueFullError extends Error {
  readonly statusCode = 429;
  constructor(message: string) {
    super(message);
    this.name = 'JobQueueFullError';
  }
}

export class InMemoryJobQueue {
  private jobs = new Map<string, DiscoveryJob>();
  private controllers = new Map<string, AbortController>();
  private waiting: DiscoveryJob[] = [];
  private active = 0;
  private readonly maxConcurrent: number;
  private readonly maxQueued: number;
  private readonly maxRetained: number;
  private readonly terminalTtlMs: number;

  constructor(
    private registry: ProviderRegistry,
    options: { maxConcurrent?: number; maxQueued?: number; maxRetained?: number; terminalTtlMs?: number } = {},
  ) {
    this.maxConcurrent = Math.max(1, options.maxConcurrent ?? 3);
    this.maxQueued = Math.max(this.maxConcurrent, options.maxQueued ?? 25);
    this.maxRetained = Math.max(1, options.maxRetained ?? 200);
    this.terminalTtlMs = Math.max(0, options.terminalTtlMs ?? 30 * 60 * 1000);
  }

  list(): DiscoveryJob[] {
    this.evictTerminal();
    return [...this.jobs.values()].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  }

  get(id: string): DiscoveryJob | undefined {
    this.evictTerminal();
    return this.jobs.get(id);
  }

  /**
   * Discovery jobs are process-local. Terminal jobs are evicted after a TTL and
   * capped by count so a long-running process does not leak finished results.
   * Queued/running/review jobs stay bounded by maxQueued/maxConcurrent instead.
   */
  private evictTerminal(): void {
    const now = Date.now();
    const terminal: DiscoveryJob[] = [];
    for (const job of this.jobs.values()) {
      if (job.state === 'done' || job.state === 'error' || job.state === 'cancelled') terminal.push(job);
    }
    for (const job of terminal) {
      if (now - Date.parse(job.updatedAt) >= this.terminalTtlMs) this.jobs.delete(job.id);
    }
    const retained = [...this.jobs.values()]
      .filter((job) => job.state === 'done' || job.state === 'error' || job.state === 'cancelled')
      .sort((a, b) => a.updatedAt.localeCompare(b.updatedAt));
    while (retained.length > this.maxRetained) {
      const job = retained.shift()!;
      this.jobs.delete(job.id);
    }
  }

  /** Number of jobs currently executing against a provider. */
  running(): number {
    return this.active;
  }

  /** Number of jobs accepted but not yet started. */
  pending(): number {
    return this.waiting.length;
  }

  create(provider: string, query: SearchQuery): DiscoveryJob {
    this.evictTerminal();
    if (this.waiting.length >= this.maxQueued) {
      throw new JobQueueFullError(`Job queue is full (${this.maxQueued} waiting); wait for a discovery run to finish`);
    }
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    const job: DiscoveryJob = { id, provider, query, state: 'queued', progress: 0, scanned: 0, found: 0, candidates: [], createdAt: now, updatedAt: now };
    this.jobs.set(id, job);
    this.waiting.push(job);
    queueMicrotask(() => this.pump());
    return job;
  }

  /**
   * Starts queued jobs while a worker slot is free. Provider fan-out is bounded so a
   * single curator cannot saturate the worker's outbound connections or memory.
   */
  private pump() {
    while (this.active < this.maxConcurrent && this.waiting.length) {
      const job = this.waiting.shift()!;
      if (job.state !== 'queued') continue;
      this.active += 1;
      void this.run(job).finally(() => {
        this.active -= 1;
        this.pump();
      });
    }
  }

  cancel(id: string): DiscoveryJob | undefined {
    const job = this.jobs.get(id);
    if (!job) return;
    this.controllers.get(id)?.abort(new Error('cancelled'));
    if (job.state === 'queued') {
      this.waiting = this.waiting.filter((item) => item.id !== id);
    }
    job.state = 'cancelled';
    job.updatedAt = new Date().toISOString();
    return job;
  }

  private async run(job: DiscoveryJob) {
    const controller = new AbortController();
    this.controllers.set(job.id, controller);
    job.state = 'running';
    job.progress = 8;
    job.updatedAt = new Date().toISOString();
    try {
      const candidates = await this.registry.search(job.provider, job.query, controller.signal);
      if (this.jobs.get(job.id)?.state === 'cancelled') return;
      job.candidates = candidates;
      job.found = candidates.length;
      job.scanned = Math.max(candidates.length, candidates.length * 3);
      job.progress = 100;
      job.state = candidates.length ? 'review' : 'done';
    } catch (error) {
      if (this.jobs.get(job.id)?.state !== 'cancelled') {
        job.state = 'error';
        job.error = providerFailureMessage(error);
      }
    } finally {
      job.updatedAt = new Date().toISOString();
      this.controllers.delete(job.id);
    }
  }
}
