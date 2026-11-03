import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import type { FieldClaim, MixRecord, RecordId } from '@syco23/catalog-domain';
import { openCatalog } from './repository.js';
import { applyClaims, decideReview, mergeRecords, refreshReview } from './merge.js';

const timestamp = '2026-10-03T12:00:00.000Z';

function claim(value: unknown, overrides: Partial<FieldClaim> = {}): FieldClaim {
  return {
    targetRecordId: 'mix_01J9CATALOGUE00000000000010',
    field: 'title',
    value,
    provider: { provider: 'youtube', resourceType: 'video', externalId: 'video-1' },
    sourceUrl: 'https://www.youtube.com/watch?v=video-1',
    observedAt: timestamp,
    evidence: 'direct',
    matchExplanation: 'Exact source identity and compatible duration',
    ...overrides
  };
}

function mix(overrides: Partial<MixRecord> = {}): MixRecord {
  return {
    kind: 'mix', id: 'mix_01J9CATALOGUE00000000000010', createdAt: timestamp, updatedAt: timestamp,
    revision: 1, verification: 'source-confirmed', reviewState: 'ready', title: 'Original title',
    people: [], eventIds: [], genres: [], styles: [], assets: [], sources: [], ...overrides
  };
}

function withCatalog(run: (repo: ReturnType<typeof openCatalog>) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-merge-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  try {
    repo.transaction((tx) => {
      tx.saveRecord(mix());
      tx.addProviderSource(mix().id, { provider: 'youtube', resourceType: 'video', externalId: 'video-1', url: 'https://www.youtube.com/watch?v=video-1' });
    });
    run(repo);
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
}

test('direct claims fill gaps, equivalent evidence is idempotent, and conflicts preserve the selection', () => {
  withCatalog((repo) => {
    const missingDescription = claim('Verified recording description', { field: 'description' });
    const first = applyClaims(repo, [missingDescription]);
    assert.equal(first.applied, 1);
    const selected = repo.getRecord(missingDescription.targetRecordId);
    assert.equal(selected?.kind === 'mix' ? selected.description : undefined, 'Verified recording description');
    assert.equal(applyClaims(repo, [missingDescription]).reviewed, 0);

    const conflict = claim('Provider title');
    const second = applyClaims(repo, [conflict]);
    assert.equal(second.reviewed, 1);
    const afterConflict = repo.getRecord(missingDescription.targetRecordId);
    assert.equal(afterConflict?.kind === 'mix' ? afterConflict.title : undefined, 'Original title');
    assert.equal(repo.listReview().length, 1);
    assert.equal(applyClaims(repo, [conflict]).reviewed, 0);
    assert.equal(repo.listReview().length, 1);
  });
});

test('parsed claims wait for curation and accepted decisions enforce the current revision', () => {
  withCatalog((repo) => {
    const pending = claim('Parsed title', { evidence: 'parsed', matchExplanation: 'Parsed from a filename excerpt' });
    const report = applyClaims(repo, [pending]);
    assert.equal(report.applied, 0);
    assert.equal(report.reviewed, 1);
    const item = repo.listReview()[0];
    const accepted = decideReview(repo, item.id, 'accept', item.recordRevision, 'curator-session-1');
    assert.equal(repo.getReview(item.id)?.state, 'accepted');
    assert.equal(accepted.id,item.targetRecordId);
    const stored = repo.getRecord(pending.targetRecordId);
    assert.equal(stored?.kind === 'mix' ? stored.title : undefined, 'Parsed title');
  });
});

test('stale review decisions fail and rejected claims do not reappear', () => {
  withCatalog((repo) => {
    const proposal = claim('Rejected proposal', { evidence: 'parsed' });
    applyClaims(repo, [proposal]);
    const item = repo.listReview()[0];
    repo.transaction((tx) => {
      const current = repo.getRecord(proposal.targetRecordId)!;
      tx.saveRecord({ ...current, revision: current.revision + 1, updatedAt: '2026-10-03T13:00:00.000Z' }, current.revision);
    });
    assert.throws(() => decideReview(repo, item.id, 'accept', item.recordRevision, 'curator-session-1'), /revision/i);
    const currentItem = refreshReview(repo, item.id);
    assert.equal(currentItem.recordRevision, repo.getRecord(proposal.targetRecordId)!.revision);
    decideReview(repo, currentItem.id, 'reject', currentItem.recordRevision, 'curator-session-1');
    assert.equal(applyClaims(repo, [proposal]).reviewed, 0);
    assert.equal(repo.listReview().length, 0);
  });
});

test('direct claims from an unlinked provider stay in review and equal values corroborate', () => {
  withCatalog((repo) => {
    const unlinked = claim('Unverified provider description', {
      field: 'description',
      provider: { provider: 'youtube', resourceType: 'video', externalId: 'other-video' },
      sourceUrl: 'https://www.youtube.com/watch?v=other-video'
    });
    const report = applyClaims(repo, [unlinked]);
    assert.equal(report.applied, 0);
    assert.equal(report.reviewed, 1);

    const equivalent = claim(' original TITLE ');
    const corroboration = applyClaims(repo, [equivalent]);
    assert.equal(corroboration.corroborated, 1);
    assert.equal(corroboration.reviewed, 0);
  });
});

function duplicateMix(overrides: Partial<MixRecord> = {}): MixRecord {
  return mix({
    id: 'mix_01J9CATALOGUE00000000000099',
    title: 'Different provider title',
    genres: ['techno'],
    styles: ['hard groove'],
    ...overrides
  });
}

function withTwoRecords(run: (repo: ReturnType<typeof openCatalog>) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-merge-dup-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  try {
    repo.transaction((tx) => {
      tx.saveRecord(mix());
      tx.addProviderSource(mix().id, { provider: 'youtube', resourceType: 'video', externalId: 'video-1', url: 'https://www.youtube.com/watch?v=video-1' });
      tx.saveRecord(duplicateMix());
      tx.addProviderSource(duplicateMix().id, { provider: 'hearthis', resourceType: 'track', externalId: 'track-9', url: 'https://hearthis.at/track-9/' });
    });
    run(repo);
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
}

test('a curator merge keeps evidence, unions missing fields and leaves disagreements in review', () => {
  withTwoRecords((repo) => {
    const survivor = repo.getRecord(mix().id)!;
    const duplicate = repo.getRecord(duplicateMix().id)!;
    const merged = mergeRecords(repo, survivor.id as RecordId, duplicate.id as RecordId, [survivor.revision, duplicate.revision], 'curator-session-1');

    // Source identities from both records follow the survivor.
    assert.equal(repo.findByProvider({ provider: 'hearthis', resourceType: 'track', externalId: 'track-9' }), survivor.id);
    assert.equal(repo.findByProvider({ provider: 'youtube', resourceType: 'video', externalId: 'video-1' }), survivor.id);

    // Values the survivor lacks are filled from the duplicate.
    assert.deepEqual(merged.kind === 'mix' ? merged.styles : undefined, ['hard groove']);
    assert.equal(merged.kind === 'mix' ? merged.revision : 0, survivor.revision + 1);

    // A disagreement must not be resolved by write order: the survivor's selected
    // title stays, and the duplicate's value becomes a review item.
    assert.equal(merged.kind === 'mix' ? merged.title : undefined, 'Original title');
    const pending = repo.listReview().filter((item) => item.targetRecordId === survivor.id);
    assert.equal(pending.length, 1);
    assert.equal(pending[0].field, 'title');
    assert.equal(pending[0].state, 'pending');
  });
});

test('a conflicting merge still succeeds when the duplicate carries a dated provider source', () => {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-merge-src-'));
  const repo = openCatalog(join(directory, 'catalog.sqlite'));
  try {
    // Imported records keep `addedAt` inside `sources`; a conflict must not pass that
    // dated source straight through as a provider reference.
    const sourced = duplicateMix({ sources: [{ provider: 'hearthis', resourceType: 'track', externalId: 'track-9', url: 'https://hearthis.at/track-9/', addedAt: timestamp }] });
    repo.transaction((tx) => {
      tx.saveRecord(mix());
      tx.addProviderSource(mix().id, { provider: 'youtube', resourceType: 'video', externalId: 'video-1', url: 'https://www.youtube.com/watch?v=video-1' });
      tx.saveRecord(sourced);
    });
    const survivor = repo.getRecord(mix().id)!;
    const duplicate = repo.getRecord(sourced.id)!;
    const merged = mergeRecords(repo, survivor.id as RecordId, duplicate.id as RecordId, [survivor.revision, duplicate.revision], 'curator-session-1');
    assert.equal(merged.kind === 'mix' ? merged.title : undefined, 'Original title');
    const pending = repo.listReview().filter((item) => item.targetRecordId === survivor.id && item.field === 'title');
    assert.equal(pending.length, 1);
    assert.equal(pending[0].state, 'pending');
  } finally {
    repo.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

test('a merged duplicate keeps its old id resolving to the survivor and cannot be merged over newer curation', () => {
  withTwoRecords((repo) => {
    const survivor = repo.getRecord(mix().id)!;
    const duplicate = repo.getRecord(duplicateMix().id)!;
    mergeRecords(repo, survivor.id as RecordId, duplicate.id as RecordId, [survivor.revision, duplicate.revision], 'curator-session-1');

    // A detail link that was already shared for the duplicate must still resolve.
    assert.equal(repo.getRecord(duplicate.id as RecordId)?.id, survivor.id);

    // A stale second curator session must be refused rather than overwriting newer
    // curation with the revisions it happened to load.
    assert.throws(
      () => mergeRecords(repo, survivor.id as RecordId, duplicate.id as RecordId, [survivor.revision, duplicate.revision], 'curator-session-2'),
      /revision conflict/i
    );
  });
});