import { reactive, readonly } from 'vue';
import type {
  CatalogRecord,
  EnrichmentReport,
  FieldEvidence,
  ImportCandidate,
  ImportResult,
  IndexKind,
  MigrationResult,
  MixRecord,
  RecordId,
  ReviewItem
} from '@syco23/catalog-domain';
import { catalogApi, type IndexOptions } from './api';

export type CatalogGateway = typeof catalogApi;

export interface CatalogStoreState {
  indexKind: IndexKind;
  records: CatalogRecord[];
  detail?: CatalogRecord;
  evidence: FieldEvidence[];
  relatedMixes: MixRecord[];
  review: ReviewItem[];
  loading: boolean;
  authenticated: boolean;
  error?: string;
  page: number;
  pageSize: number;
  total: number;
}

export function createCatalogStore(gateway: CatalogGateway = catalogApi) {
  const state = reactive<CatalogStoreState>({
    indexKind: 'mix', records: [], evidence: [], relatedMixes: [], review: [], loading: false, authenticated: false,
    page: 1, pageSize: 25, total: 0
  });

  function remember(record: CatalogRecord) {
    state.detail = record;
    const index = state.records.findIndex((item) => item.id === record.id);
    if (index >= 0) state.records[index] = record;
    return record;
  }

  async function loadIndex(kind: IndexKind, options: IndexOptions = {}) {
    state.loading = true;
    state.error = undefined;
    state.indexKind = kind;
    state.page = options.page ?? 1;
    state.pageSize = options.pageSize ?? 25;
    try {
      const result = await gateway.getIndex(kind, { ...options, page: state.page, pageSize: state.pageSize });
      state.records = result.items;
      state.page = result.page;
      state.pageSize = result.pageSize;
      state.total = result.total;
      return result;
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Catalog index could not be loaded';
      throw error;
    } finally {
      state.loading = false;
    }
  }

  async function loadDetail(id: RecordId) {
    state.loading = true;
    state.error = undefined;
    try {
      return remember(await gateway.getRecord(id));
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Catalog record could not be loaded';
      throw error;
    } finally {
      state.loading = false;
    }
  }

  async function loadReview() {
    state.error = undefined;
    try {
      state.review = await gateway.getReview();
      return state.review;
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Review queue could not be loaded';
      throw error;
    }
  }

  /**
   * Field provenance for the open record. A failure here is not fatal: the record is
   * still worth showing, it just cannot explain why a field holds its value, so the
   * detail view renders without evidence rather than refusing to load.
   */
  async function loadEvidence(id: RecordId) {
    try {
      const result = await gateway.getEvidence(id);
      state.evidence = result.evidence;
      return state.evidence;
    } catch {
      state.evidence = [];
      return state.evidence;
    }
  }

  /**
   * Mixes that reference this entity. Resets on every call so switching from an
   * artist with sets to one without cannot leave the previous list on screen.
   */
  async function loadRelatedMixes(id: RecordId) {
    try {
      const result = await gateway.getRelatedMixes(id);
      state.relatedMixes = result.mixes.filter((mix): mix is MixRecord => mix.kind === 'mix');
      return state.relatedMixes;
    } catch {
      state.relatedMixes = [];
      return state.relatedMixes;
    }
  }

  async function checkSession() {
    state.authenticated = await gateway.checkSession();
    return state.authenticated;
  }

  async function login(password: string) {
    state.error = undefined;
    try {
      state.authenticated = await gateway.login(password);
      return state.authenticated;
    } catch (error) {
      state.authenticated = false;
      state.error = error instanceof Error ? error.message : 'Login failed';
      throw error;
    }
  }

  async function logout() {
    await gateway.logout();
    state.authenticated = false;
    state.review = [];
  }

  async function importCandidate(candidate: ImportCandidate): Promise<ImportResult> {
    state.error = undefined;
    try {
      const result = await gateway.importCandidate(candidate);
      remember(result.record);
      return result;
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Import was not saved';
      throw error;
    }
  }

  async function enrichRecord(id: RecordId): Promise<EnrichmentReport> {
    state.error = undefined;
    try {
      const report = await gateway.enrichRecord(id);
      await loadDetail(id);
      await loadReview().catch(() => undefined);
      return report;
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Enrichment failed';
      throw error;
    }
  }

  async function persistWaveform(id: RecordId, imageDataUrl: string, sourceUrl: string) {
    const record = remember(await gateway.persistWaveform(id, imageDataUrl, sourceUrl));
    return record;
  }

  async function updateRecord(id: RecordId, patch: unknown, revision: number) {
    state.error = undefined;
    try {
      return remember(await gateway.updateRecord(id, patch, revision));
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Changes were not saved';
      throw error;
    }
  }

  async function decideReview(id: RecordId, decision: 'accept' | 'reject', revision: number) {
    state.error = undefined;
    try {
      const item = await gateway.decideReview(id, decision, revision);
      state.review = state.review.filter((review) => review.id !== id);
      await loadDetail(item.targetRecordId);
      return item;
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Review decision was not saved';
      throw error;
    }
  }

  async function refreshReview(id: RecordId) {
    const item = await gateway.refreshReview(id);
    const index = state.review.findIndex((review) => review.id === id);
    if (index >= 0) state.review[index] = item;
    return item;
  }

  /**
   * Merge a confirmed duplicate into its survivor. The retired record leaves the
   * index: a failed merge must not look like it succeeded, so the local list is only
   * updated from the record the server actually committed.
   */
  async function mergeRecords(survivorId: RecordId, duplicateId: RecordId, expectedRevisions: [number, number]) {
    state.error = undefined;
    try {
      const merged = remember(await gateway.mergeRecords(survivorId, duplicateId, expectedRevisions));
      state.records = state.records.filter((record) => record.id !== duplicateId);
      state.total = Math.max(0, state.total - 1);
      // The merge can raise new review items for fields the two records disagreed on.
      await loadReview().catch(() => undefined);
      return merged;
    } catch (error) {
      state.error = error instanceof Error ? error.message : 'Records were not merged';
      throw error;
    }
  }

  async function migrateLocalLibrary(): Promise<MigrationResult | undefined> {
    if (typeof window === 'undefined') return undefined;
    const raw = window.localStorage.getItem('syco23.mixsets.library');
    if (!raw) return undefined;
    const records = JSON.parse(raw) as unknown[];
    const batchStorageKey = 'syco23.mixsets.migrationBatchId';
    const batchId = window.localStorage.getItem(batchStorageKey) || crypto.randomUUID();
    window.localStorage.setItem(batchStorageKey, batchId);
    const media = records.flatMap((record) => {
      if (!record || typeof record !== 'object') return [];
      const legacy = record as Record<string, unknown>;
      const waveform = legacy.waveform;
      if (!waveform || typeof waveform !== 'object') return [];
      const asset = waveform as Record<string, unknown>;
      return typeof legacy.id === 'string' && typeof asset.imageDataUrl === 'string' && typeof asset.sourceUrl === 'string'
        ? [{ legacyId: legacy.id, imageDataUrl: asset.imageDataUrl, sourceUrl: asset.sourceUrl }]
        : [];
    });
    const recordsWithoutMedia = records.map((record) => {
      if (!record || typeof record !== 'object' || !('waveform' in record)) return record;
      const withoutWaveform = { ...record };
      delete withoutWaveform.waveform;
      return withoutWaveform;
    });
    const result = await gateway.migrate(batchId, recordsWithoutMedia);
    for (const asset of media) {
      const recordId = result.legacyIds[asset.legacyId];
      if (recordId) await gateway.persistWaveform(recordId, asset.imageDataUrl, asset.sourceUrl);
    }
    return result;
  }

  return {
    state: readonly(state),
    loadIndex,
    loadDetail,
    loadReview,
    loadEvidence,
    loadRelatedMixes,
    checkSession,
    login,
    logout,
    importCandidate,
    enrichRecord,
    persistWaveform,
    updateRecord,
    decideReview,
    refreshReview,
    mergeRecords,
    migrateLocalLibrary
  };
}

export const useCatalogStore = createCatalogStore();