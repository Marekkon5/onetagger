import assert from 'node:assert/strict';
import test from 'node:test';
import { InMemoryJobQueue, JobQueueFullError } from './in-memory.js';
import type { ProviderRegistry } from '../core/registry.js';
import type { DiscoveryProvider } from '../domain.js';

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

/**
 * A registry whose provider blocks until released, so observable concurrency can be
 * asserted. Mirrors ProviderRegistry.search(): resolve the provider, call its search().
 */
function blockingRegistry() {
  const gates: Array<() => void> = [];
  const started: string[] = [];
  let failAtCall = -1;
  let failureMessage = 'provider unavailable';

  const provider: DiscoveryProvider = {
    id: 'youtube',
    async health() {
      return { id: 'youtube', state: 'ready', detail: 'ok', checkedAt: '2026-10-03T12:00:00.000Z' };
    },
    async search() {
      const call = started.length;
      started.push('youtube');
      // A configured failure rejects immediately, before any gate is consumed.
      if (call === failAtCall) throw new Error(failureMessage);
      await new Promise<void>((resolve) => gates.push(resolve));
      return [];
    }
  } as unknown as DiscoveryProvider;

  const discovery = new Map<string, DiscoveryProvider>([['youtube', provider]]);
  const registry = {
    discovery,
    search(providerId: string, query: never, signal?: AbortSignal) {
      const found = discovery.get(providerId);
      if (!found) throw new Error(`Unknown discovery provider: ${providerId}`);
      return found.search(query, signal);
    }
  } as unknown as ProviderRegistry;

  return {
    registry,
    gates,
    started,
    failOn(call: number, message = 'provider unavailable') {
      failAtCall = call;
      failureMessage = message;
    }
  };
}

test('discovery jobs run at most the configured number concurrently', async () => {
  const { registry, gates, started } = blockingRegistry();
  const queue = new InMemoryJobQueue(registry, { maxConcurrent: 2 });
  for (let i = 0; i < 5; i += 1) queue.create('youtube', { q: `query ${i}` });

  await tick();
  // Only the first two may reach the provider; the rest stay queued.
  assert.equal(started.length, 2, 'concurrency must be bounded');
  assert.equal(queue.running(), 2);
  assert.equal(queue.pending(), 3);

  // Releasing a slot admits exactly one more job.
  gates.shift()!();
  await tick();
  await tick();
  assert.equal(started.length, 3);
  assert.equal(queue.running(), 2);
});

test('a full queue is rejected instead of growing without bound', () => {
  const { registry } = blockingRegistry();
  const queue = new InMemoryJobQueue(registry, { maxConcurrent: 1, maxQueued: 3 });
  queue.create('youtube', { q: 'a' });
  queue.create('youtube', { q: 'b' });
  queue.create('youtube', { q: 'c' });
  assert.throws(() => queue.create('youtube', { q: 'd' }), (error: unknown) => {
    assert.ok(error instanceof JobQueueFullError);
    assert.equal(error.statusCode, 429);
    return true;
  });
});

test('cancelling a queued job removes it from the pending queue', async () => {
  const { registry, started } = blockingRegistry();
  const queue = new InMemoryJobQueue(registry, { maxConcurrent: 1 });
  const running = queue.create('youtube', { q: 'running' });
  const waiting = queue.create('youtube', { q: 'waiting' });
  await tick();

  const cancelled = queue.cancel(waiting.id);
  assert.equal(cancelled?.state, 'cancelled');
  assert.equal(queue.pending(), 0);

  // The cancelled job must never reach the provider.
  await tick();
  assert.equal(started.length, 1);
  assert.equal(running.state, 'running');
});

test('a failing job frees its slot and does not block the queue', async () => {
  const { registry, gates, started, failOn } = blockingRegistry();
  failOn(0);
  const queue = new InMemoryJobQueue(registry, { maxConcurrent: 1 });
  const first = queue.create('youtube', { q: 'first' });
  const second = queue.create('youtube', { q: 'second' });
  for (let i = 0; i < 4; i += 1) await tick();
  // Release the second job so it can complete after the first one failed.
  gates.shift()?.();
  for (let i = 0; i < 4; i += 1) await tick();

  assert.equal(started.length, 2);
  assert.equal(first.state, 'error');
  assert.match(first.error ?? '', /provider unavailable/);
  assert.equal(second.state, 'done');
  assert.equal(queue.running(), 0);
});
