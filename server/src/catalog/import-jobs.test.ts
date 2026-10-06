import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { openCatalog } from './repository.js';

const timestamp = '2026-10-06T12:00:00.000Z';

function withCatalog(run: (path: string) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-import-'));
  const path = join(directory, 'catalog.sqlite');
  try {
    run(path);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function job(id: string, key: string) {
  return {
    id,
    requestedBy: 'curator-test',
    provider: 'soundcloud' as const,
    sourceUrl: 'https://soundcloud.com/example/example',
    mode: 'metadata' as const,
    state: 'queued' as const,
    idempotencyKey: key,
    attempt: 0,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}

test('import jobs persist with idempotency, events and outbox lease', () => {
  withCatalog((path) => {
    const repo = openCatalog(path);
    const created = repo.createImportJob(job('imp_testjob01', 'idem-key-01'));
    assert.equal(created.state, 'queued');
    assert.equal(repo.getImportJobByIdempotency('idem-key-01')?.id, 'imp_testjob01');

    repo.enqueueImportJob(created.id);
    const claimed = repo.claimDueImportJob('worker-1');
    assert.equal(claimed?.id, 'imp_testjob01');
    // Second claim while leased returns nothing.
    assert.equal(repo.claimDueImportJob('worker-2'), undefined);

    repo.updateImportJobState(created.id, 'resolving');
    const events = repo.listImportEvents(created.id);
    assert.ok(events.length >= 2);
    assert.deepEqual(events.map((event) => event.sequence), [...events.map((event) => event.sequence)].sort((a, b) => a - b));

    repo.saveImportProvenance(created.id, {
      provider: 'soundcloud',
      sourceUrl: 'https://soundcloud.com/example/example',
      retrievalMethod: 'official-api',
      observedAt: timestamp,
      snapshot: { title: 'Example' },
    });
    repo.saveImportRightsConsent({
      id: 'rights_test01',
      jobId: created.id,
      requestedBy: 'curator-test',
      basis: 'provider_metadata_only',
      provider: 'soundcloud',
      sourceUrl: 'https://soundcloud.com/example/example',
      attestationVersion: '2026-10',
    });
    repo.registerImportArtifact({
      id: 'art_test0001',
      jobId: created.id,
      role: 'metadata',
      objectKey: 'imports/imp_testjob01/metadata/snapshot.json',
      sha256: 'a'.repeat(64),
    });
    assert.equal(repo.listImportArtifacts(created.id).length, 1);

    const scored = repo.saveImportEvidenceScore({
      id: 'evd_test0001',
      jobId: created.id,
      field: 'title',
      scored: {
        score: 95,
        algorithmVersion: 'evidence-v1',
        components: { I: 1, T: 1, D: 1, R: 1, P: 1, C: 0, X: 0 },
        hardGates: { identity: true, rights: true, noConflict: true, policy: true },
        decision: 'auto_apply',
        evaluatedAt: timestamp,
      },
    });
    assert.equal(scored.decision, 'auto_apply');
    assert.equal(repo.listImportEvidenceScores(created.id).length, 1);

    repo.updateImportJobState(created.id, 'completed');
    // Terminal state clears the outbox.
    assert.equal(repo.claimDueImportJob('worker-1'), undefined);
    repo.close();
  });
});

test('duplicate idempotency keys are rejected', () => {
  withCatalog((path) => {
    const repo = openCatalog(path);
    repo.createImportJob(job('imp_testjob02', 'idem-dup-key1'));
    assert.throws(() => repo.createImportJob(job('imp_testjob03', 'idem-dup-key1')));
    repo.close();
  });
});
