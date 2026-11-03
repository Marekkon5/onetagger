import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
  FieldClaimSchema,
  ImportJobSchema,
  ImportMetadataFactsSchema,
  decideImportPolicy,
  durationComponent,
  scoreEvidence,
} from './index.js';


const timestamp = '2026-10-06T12:00:00.000Z';

test('soundcloud audio requires an audio rights basis, metadata is allowed', () => {
  const denied = decideImportPolicy({
    source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
    mode: 'audio',
    conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
    autoApplyThreshold: 80,
    rights: { basis: 'provider_metadata_only', attestationVersion: '2026-10' },
  });
  assert.equal(denied.allowed, false);
  assert.equal(denied.blockedPolicy, true);

  const allowed = decideImportPolicy({
    source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
    mode: 'audio',
    conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
    autoApplyThreshold: 80,
    rights: { basis: 'global_rights_confirmation', attestationVersion: 'global-2026-11' },
  });
  assert.equal(allowed.allowed, true);

  const missing = decideImportPolicy({
    source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
    mode: 'audio',
    conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
    autoApplyThreshold: 80,
  });
  assert.equal(missing.allowed, false, 'audio without a rights basis fails closed');

  const metadata = decideImportPolicy({
    source: { provider: 'soundcloud', url: 'https://soundcloud.com/example/example' },
    mode: 'metadata',
    autoApplyThreshold: 80,
  });
  assert.equal(metadata.allowed, true);
  assert.equal(metadata.blockedPolicy, false);
});

test('user-upload jobs persist a private URN rather than an http URL', () => {
  const job = ImportJobSchema.parse({
    id: 'imp_abcdef0123456789abcdef01',
    requestedBy: 'curator-test',
    provider: 'user_upload',
    sourceUrl: 'urn:syco23:upload:upl_123',
    mode: 'audio',
    state: 'queued',
    idempotencyKey: 'idem-upload-urn-01',
    attempt: 0,
    createdAt: timestamp,
    updatedAt: timestamp,
  });
  assert.equal(job.sourceUrl, 'urn:syco23:upload:upl_123');
  assert.throws(() => ImportJobSchema.parse({
    ...job,
    sourceUrl: '/tmp/mix.mp3',
  }));
});

test('user upload audio requires an explicit audio rights basis', () => {
  const denied = decideImportPolicy({
    source: { provider: 'user_upload', uploadId: 'upl_123' },
    mode: 'audio',
    conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
    autoApplyThreshold: 80,
    rights: { basis: 'provider_metadata_only', attestationVersion: '2026-10' },
  });
  assert.equal(denied.blockedPolicy, true);

  const allowed = decideImportPolicy({
    source: { provider: 'user_upload', uploadId: 'upl_123' },
    mode: 'audio',
    conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
    autoApplyThreshold: 80,
    rights: { basis: 'user_authorized_copy', attestationVersion: '2026-10' },
  });
  assert.equal(allowed.allowed, true);
});

test('mixcloud, discogs and musicbrainz metadata are importable and every provider has an explicit capability', () => {
  const mixcloud = decideImportPolicy({
    source: { provider: 'mixcloud', url: 'https://www.mixcloud.com/example/show/' },
    mode: 'metadata',
    autoApplyThreshold: 80,
    rights: { basis: 'provider_metadata_only', attestationVersion: '2026-10' },
  });
  assert.equal(mixcloud.allowed, true);
  assert.equal(mixcloud.blockedPolicy, false);
  assert.deepEqual(mixcloud.capabilities, {
    metadata: true,
    serverSideAudioAcquisition: true,
    requiresUserConsent: false,
    requiresExplicitRightsBasis: true,
  });

  const discogs = decideImportPolicy({
    source: { provider: 'discogs', url: 'https://www.discogs.com/release/249504' },
    mode: 'metadata',
    autoApplyThreshold: 80,
  });
  assert.equal(discogs.allowed, true, 'Discogs release pages supply metadata import');

  const musicbrainz = decideImportPolicy({
    source: { provider: 'musicbrainz', url: 'https://musicbrainz.org/recording/b0261007-c152-424e-8c17-339ba94c0f3d' },
    mode: 'metadata',
    autoApplyThreshold: 80,
  });
  assert.equal(musicbrainz.allowed, true, 'MusicBrainz recording pages supply metadata import');

  const musicbrainzAudio = decideImportPolicy({
    source: { provider: 'musicbrainz', url: 'https://musicbrainz.org/recording/b0261007-c152-424e-8c17-339ba94c0f3d' },
    mode: 'audio',
    conversion: { format: 'mp3', bitrateKbps: 320, id3Version: '2.3' },
    autoApplyThreshold: 80,
    rights: { basis: 'global_rights_confirmation', attestationVersion: 'global-2026-11' },
  });
  assert.equal(musicbrainzAudio.allowed, false, 'MusicBrainz never supplies mix audio');
});

test('metadata facts contract keeps sourced values and rejects private paths', () => {
  const facts = ImportMetadataFactsSchema.parse({
    provider: 'mixcloud',
    sourceUrl: 'https://www.mixcloud.com/example/show/',
    title: 'Example long-form show',
    artists: ['Example Artist'],
    description: 'Sourced description',
    durationMs: 3_600_000,
    genres: ['Tekno'],
    artworkUrl: 'https://thumbnailer.mixcloud.com/unsafe/300x300/extaudio/x/y/z',
    observedAt: timestamp,
  });
  assert.equal(facts.title, 'Example long-form show');
  assert.throws(() => ImportMetadataFactsSchema.parse({ ...facts, sourceUrl: '/tmp/mix.mp3' }));
  assert.throws(() => ImportMetadataFactsSchema.parse({ ...facts, title: '' }));
});

test('claims may name the curator as their provenance without borrowing a provider', () => {
  const claim = FieldClaimSchema.parse({
    targetRecordId: 'mix_abcdef0123456789abcdef01',
    field: 'title',
    value: 'Curated title',
    provider: { provider: 'curator', resourceType: 'curation', externalId: 'session-1' },
    sourceUrl: 'https://mixsets.syco23.org/catalog/mix_abcdef0123456789abcdef01',
    observedAt: timestamp,
    evidence: 'curated',
    matchExplanation: 'Explicit curator correction',
  });
  assert.equal(claim.provider.provider, 'curator');
});

test('evidence v1 scores field-level evidence with hard gates', () => {
  const strong = scoreEvidence({
    I: 1, T: 1, D: 1, R: 1, P: 1, C: 1, X: 0,
    identityGate: true, rightsGate: true,
    noDirectConflict: true, providerPolicyAllowsUse: true,
  }, timestamp);
  assert.equal(strong.score, 100);
  assert.equal(strong.decision, 'auto_apply');
  assert.equal(strong.algorithmVersion, 'evidence-v1');

  const weak = scoreEvidence({
    I: 0.5, T: 0.5, D: 0.5, R: 0, P: 0.5, C: 0, X: 0,
    identityGate: true, rightsGate: true,
    noDirectConflict: true, providerPolicyAllowsUse: true,
  }, timestamp);
  assert.ok(weak.score < 80);
  assert.equal(weak.decision, 'review');

  const conflict = scoreEvidence({
    I: 1, T: 1, D: 1, R: 1, P: 1, C: 1, X: 1,
    identityGate: true, rightsGate: true,
    noDirectConflict: false, providerPolicyAllowsUse: true,
  }, timestamp);
  assert.equal(conflict.decision, 'reject');
});

test('duration component falls from 2% to 10% deviation', () => {
  assert.equal(durationComponent(3_600_000, 3_600_000), 1);
  assert.equal(durationComponent(3_600_000, 3_672_000), 1);
  assert.equal(durationComponent(3_600_000, 3_960_000), 0);
  const mid = durationComponent(3_600_000, 3_816_000);
  assert.ok(mid > 0 && mid < 1);
});

export function withTempDir(run: (dir: string) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-import-'));
  try {
    run(directory);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}
