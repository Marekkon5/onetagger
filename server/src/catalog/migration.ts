import { createHash, randomUUID } from 'node:crypto';
import {
  LegacyMixSchema,
  MigrationResultSchema,
  ProviderIdSchema,
  RecordingDateSchema,
  normalizeName,
  normalizeProviderRef,
  getFieldValue,
  validateField,
  FieldDefinitions,
  type FieldClaim,
  type CatalogRecord,
  type EntityRecord,
  type LegacyMix,
  type MigrationResult,
  type MixRecord,
  type ProviderRef,
  type RecordId
} from '@syco23/catalog-domain';
import { persistWaveform } from './media.js';
import { applyOne, claimFingerprint } from './merge.js';
import type { CatalogRepository } from './repository.js';
import type { CuratorActor } from '../auth/curator.js';

const demoIds = new Set(['spiral-warehouse-2001', 'metek-aniane-98', 'novabase-live-tech', 'gotek-roma-2007']);

function stableId(prefix: string, value: string): RecordId {
  return `${prefix}_${createHash('sha256').update(value).digest('hex').slice(0, 32)}`;
}

function parsedRecordingDate(value: string | undefined) {
  if (!value) return undefined;
  const normalized = value.trim();
  const precision = normalized.length === 4 ? 'year' : normalized.length === 7 ? 'month' : normalized.length === 10 ? 'day' : undefined;
  if (!precision) return undefined;
  return RecordingDateSchema.safeParse({ value: normalized, precision }).data;
}

function sourceRefs(record: LegacyMix): ProviderRef[] {
  const refs: ProviderRef[] = [];
  for (const source of record.sources ?? []) {
    if (!source || typeof source !== 'object') continue;
    const value = source as Record<string, unknown>;
    const provider = value.provider;
    const urlValue = value.url;
    if (typeof provider !== 'string' || typeof urlValue !== 'string') continue;
    const parsedProvider = ProviderIdSchema.safeParse(provider);
    if (!parsedProvider.success) continue;
    try {
      const url = new URL(urlValue);
      if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) continue;
      refs.push(normalizeProviderRef({
        provider: parsedProvider.data,
        resourceType: typeof value.resourceType === 'string' ? value.resourceType : 'recording',
        externalId: typeof value.externalId === 'string' && value.externalId ? value.externalId : createHash('sha256').update(url.toString()).digest('hex'),
        url: url.toString()
      }));
    } catch {
      continue;
    }
  }
  return refs;
}

function linkedEntity(record: LegacyMix, role: 'artist' | 'crew' | 'label', name: string, entityId: RecordId, timestamp: string): EntityRecord {
  const legacyEntities = Array.isArray(record.entities) ? record.entities.filter((entity): entity is Record<string, unknown> => Boolean(entity && typeof entity === 'object')) : [];
  const explicit = legacyEntities.find((entity) => entity.kind === role && typeof entity.name === 'string' && normalizeName(entity.name) === normalizeName(name));
  const provider = typeof explicit?.provider === 'string' ? explicit.provider as ProviderRef['provider'] : undefined;
  const externalId = typeof explicit?.externalId === 'string' ? explicit.externalId : undefined;
  const url = typeof explicit?.url === 'string' ? explicit.url : undefined;
  const parsedProvider = provider ? ProviderIdSchema.safeParse(provider) : undefined;
  const source = parsedProvider?.success && externalId
    ? [{ provider: parsedProvider.data, resourceType: role === 'label' ? 'label' : 'artist', externalId, ...(url ? { url } : {}) }]
    : [];
  const imageUrl = typeof explicit?.imageUrl === 'string' ? explicit.imageUrl : undefined;
  const assetRole = role === 'artist' ? 'artist-portrait' : role === 'crew' ? 'crew-logo' : 'label-logo';
  const asset = imageUrl ? (() => {
    try {
      const parsed = new URL(imageUrl);
      const imageSource: ProviderRef['provider'] | 'local' = parsedProvider?.success ? parsedProvider.data : 'local';
      return ['http:', 'https:'].includes(parsed.protocol) ? [{ role: assetRole as 'artist-portrait' | 'crew-logo' | 'label-logo', url: parsed.toString(), source: imageSource }] : [];
    } catch { return []; }
  })() : [];
  return {
    kind: 'entity', id: entityId, createdAt: timestamp, updatedAt: timestamp, revision: 1,
    verification: source.length ? 'source-confirmed' : 'proposed', reviewState: 'ready', displayName: name,
    roles: [role], aliases: [], assets: asset,
    providerRefs: source,
    ...(typeof explicit?.profile === 'string' ? { profile: explicit.profile.slice(0, 20000) } : {}),
    ...(role === 'artist' ? { artist: {} } : {}),
    ...(role === 'crew' ? { crew: {} } : {}),
    ...(role === 'label' ? { label: {} } : {})
  };
}

function legacyCover(record: LegacyMix): MixRecord['assets'][number] | undefined {
  const artwork = Array.isArray(record.artwork) ? record.artwork : [];
  for (const entry of artwork) {
    if (!entry || typeof entry !== 'object') continue;
    const value = entry as Record<string, unknown>;
    if (value.kind !== 'cover' || typeof value.url !== 'string') continue;
    try {
      const url = new URL(value.url);
      if (!['http:', 'https:'].includes(url.protocol)) continue;
      const provider = value.source;
      const parsedProvider = typeof provider === 'string' ? ProviderIdSchema.safeParse(provider) : undefined;
      const source = parsedProvider?.success ? parsedProvider.data : 'local';
      return { role: 'mix-cover', url: url.toString(), source };
    } catch {
      continue;
    }
  }
  return undefined;
}

function createMix(record: LegacyMix, id: RecordId, timestamp: string, refs: ProviderRef[], entityLinks: Array<{ entityId: RecordId; role: 'artist' | 'crew' | 'label' }>, eventIds: RecordId[]): MixRecord {
  const cover = legacyCover(record);
  const candidates = Array.isArray(record.candidates) ? record.candidates : [];
  const assets = cover ? [cover] : [];
  return {
    kind: 'mix', id,
    createdAt: typeof record.createdAt === 'string' ? record.createdAt : timestamp,
    updatedAt: timestamp,
    revision: 1,
    verification: 'curator-confirmed',
    reviewState: candidates.some((candidate) => candidate && typeof candidate === 'object' && (candidate as Record<string, unknown>).state === 'pending') ? 'review' : 'ready',
    title: record.title,
    ...(parsedRecordingDate(typeof record.recordedAt === 'string' ? record.recordedAt : undefined) ? { recordingDate: parsedRecordingDate(record.recordedAt as string) } : {}),
    ...(typeof record.durationMs === 'number' ? { durationMs: record.durationMs } : {}),
    ...(typeof record.description === 'string' ? { description: record.description.slice(0, 10000) } : {}),
    people: entityLinks,
    eventIds,
    genres: Array.isArray(record.genres) ? record.genres.filter((value): value is string => typeof value === 'string').slice(0, 100) : [],
    styles: Array.isArray(record.styles) ? record.styles.filter((value): value is string => typeof value === 'string').slice(0, 100) : [],
    assets,
    sources: refs.map((ref) => ({ ...ref, addedAt: timestamp }))
  };
}


function restoreLegacyDecisions(tx: import('./repository.js').CatalogTransaction, legacy:LegacyMix, targetId:RecordId, refs:ProviderRef[],actor:CuratorActor) {
  const target=tx.getRecord(targetId)!;
  const fallback=refs[0]??{provider:'archiveorg' as const,resourceType:'legacy-curation',externalId:legacy.id,url:`https://mixsets.syco23.org/mix/${encodeURIComponent(legacy.id)}`};
  const candidates=Array.isArray(legacy.candidates)?legacy.candidates:[];
  for(const raw of candidates) {
    if(!raw||typeof raw!=='object')continue;const candidate=raw as Record<string,unknown>;
    if(!candidate.fields||typeof candidate.fields!=='object')continue;
    for(const [legacyField,rawValue] of Object.entries(candidate.fields as object)) {
      let field=legacyField,value=rawValue;
      if(field==='recordedAt'){field='recordingDate';value=parsedRecordingDate(typeof rawValue==='string'?rawValue:undefined);}
      if(field==='artwork'){field='cover';value=legacyCover({...legacy,artwork:rawValue as unknown[]});if(value)value={role:'mix-cover',url:(value as {url:string}).url};}
      if(!Object.hasOwn(FieldDefinitions,field)||value===undefined)continue;
      try {validateField(target,field,value);}catch{continue;}
      const provider=typeof candidate.provider==='string'?ProviderIdSchema.safeParse(candidate.provider):undefined;
      const ref=provider?.success?refs.find(source=>source.provider===provider.data)??{...fallback,provider:provider.data}:fallback;
      const claim:FieldClaim={targetRecordId:targetId,field,value,provider:ref,sourceUrl:ref.url??fallback.url!,observedAt:new Date().toISOString(),evidence:'parsed',matchExplanation:'Preserved legacy review decision and supporting local source'};
      const fingerprint=claimFingerprint(claim);if(tx.getClaim(fingerprint))continue;
      tx.addClaim(fingerprint,fingerprint,claim,'pending',tx.getRecord(targetId)!.revision,getFieldValue(tx.getRecord(targetId)!,field));
      if(candidate.state==='accepted'||candidate.state==='rejected')tx.decideReview(fingerprint,candidate.state,actor.sessionId);
    }
  }
}

export function migrateLegacyLibrary(repo:CatalogRepository,input:unknown[],batchId:string,actor:CuratorActor):MigrationResult {
  if(!batchId||batchId.length>200||input.length>1000)throw new Error('Migration batch is outside supported bounds');
  const prior=repo.getMigrationBatch(batchId);
  if(prior&&(!prior.outcomes||prior.outcomes.every(outcome=>outcome.status!=='partial')))return prior;
  const result:MigrationResult={batchId,imported:0,existing:0,rejected:0,legacyIds:{},outcomes:[]};
  for(const raw of input) {
    const parsed=LegacyMixSchema.safeParse(raw),legacyId=parsed.success?parsed.data.id:String((raw as {id?:unknown})?.id??'');
    const previous=prior?.outcomes?.find(outcome=>outcome.legacyId===legacyId&&['imported','existing'].includes(outcome.status));
    if(previous){result.outcomes!.push(previous);if(previous.recordId)result.legacyIds[legacyId]=previous.recordId;if(previous.status==='imported')result.imported++;else result.existing++;continue;}
    const outcome:NonNullable<MigrationResult['outcomes']>[number]={legacyId,status:'rejected',errors:[]};
    result.outcomes!.push(outcome);
    if(!parsed.success||demoIds.has(legacyId)){outcome.errors.push(demoIds.has(legacyId)?'Known demo fixture excluded':'Legacy record validation failed');result.rejected++;continue;}
    const legacy=parsed.data,refs=sourceRefs(legacy),timestamp=new Date().toISOString();
    try {
      const owners=new Set(refs.map(ref=>repo.findBySource(ref)??repo.findByProvider(ref)).filter((id):id is string=>!!id));
      if(owners.size>1)throw new Error('Legacy source identities refer to different canonical records; review is required');
      const alias=repo.getRecord(legacy.id),existingId=[...owners][0]??alias?.id;
      if(alias&&owners.size&&alias.id!==existingId)throw new Error('Legacy ID collision requires review');
      if(alias&&!owners.size&&refs.length)throw new Error('Legacy ID collision with different source identities requires review');
      let mixId=existingId??stableId('mix_mig',legacy.id);
      const existing=existingId?repo.getRecord(existingId):undefined;
      if(existing&&existing.kind!=='mix')throw new Error('Legacy source belongs to a non-mix identity');
      repo.transaction(tx=>{
        const entityLinks:MixRecord['people']=existing?.kind==='mix'?[...existing.people]:[];
        const source=refs[0]??{provider:'archiveorg' as const,resourceType:'legacy-curation',externalId:legacy.id,url:`https://mixsets.syco23.org/mix/${encodeURIComponent(legacy.id)}`};
        for(const [role,names] of [['artist',legacy.artists??[]],['crew',legacy.crews??[]],['label',Array.isArray(legacy.entities)?legacy.entities.flatMap(raw=>raw&&typeof raw==='object'&&(raw as {kind?:unknown}).kind==='label'&&typeof(raw as {name?:unknown}).name==='string'?[(raw as {name:string}).name]:[]):[]]] as const) {
          for(const [index,name] of names.entries()) {
            if(!name.trim())continue;
            if(entityLinks.some(link=>link.role===role&&tx.getRecord(link.entityId)?.kind==='entity'&&normalizeName((tx.getRecord(link.entityId) as EntityRecord).displayName)===normalizeName(name)))continue;
            const proposed=linkedEntity(legacy,role,name.trim(),stableId('entity_mig',`${legacy.id}:${role}:${index}:${normalizeName(name)}`),timestamp);
            const owner=proposed.providerRefs.map(ref=>tx.findByProvider(ref)).find(Boolean);
            const entity=owner?tx.getRecord(owner):tx.getRecord(proposed.id);
            const entityId=entity?.id??proposed.id;
            if(!entity){tx.saveRecord({...proposed,reviewState:proposed.verification==='proposed'?'review':'ready'});for(const ref of proposed.providerRefs)tx.addProviderSource(entityId,ref);if(proposed.verification==='proposed')applyOne(tx,{targetRecordId:entityId,field:'displayName',value:name.trim(),provider:source,sourceUrl:source.url!,observedAt:timestamp,evidence:'parsed',matchExplanation:'Preserved legacy artist/crew name requires identity confirmation'});}
            if(!entityLinks.some(link=>link.entityId===entityId&&link.role===role))entityLinks.push({entityId,role});
          }
        }
        const eventName=typeof legacy.event==='string'?legacy.event.trim():'';
        const eventIds=existing?.kind==='mix'?[...existing.eventIds]:[];
        if(eventName){const eventId=stableId('event_mig',`${legacy.id}:event:${normalizeName(eventName)}`);if(!tx.getRecord(eventId)){tx.saveRecord({kind:'event',id:eventId,createdAt:timestamp,updatedAt:timestamp,revision:1,verification:'proposed',reviewState:'review',name:eventName,assets:[],sourceUrls:refs.flatMap(ref=>ref.url?[ref.url]:[]),mixIds:[]});applyOne(tx,{targetRecordId:eventId,field:'name',value:eventName,provider:source,sourceUrl:source.url!,observedAt:timestamp,evidence:'parsed',matchExplanation:'Legacy event has no supported date or venue; confirm identity'});}if(!eventIds.includes(eventId))eventIds.push(eventId);}
        if(!existing)tx.saveRecord(createMix(legacy,mixId,timestamp,refs,[],[]));
        else {const current=tx.getRecord(mixId)! as MixRecord;tx.saveRecord({...current,people:current.people,eventIds:current.eventIds,sources:[...current.sources,...refs.filter(ref=>!current.sources.some(source=>source.provider===ref.provider&&source.resourceType===ref.resourceType&&source.externalId===ref.externalId))],revision:current.revision+1,updatedAt:timestamp},current.revision);}
        for(const ref of refs)if(!tx.findByProvider(ref))tx.addProviderSource(mixId,ref);
        tx.addLegacyAlias(legacy.id,mixId);
        const imported=createMix(legacy,mixId,timestamp,refs,entityLinks,eventIds);
        for(const field of ['title','description','durationMs','recordingDate','genres','styles','cover'] as const) {
          const value=getFieldValue(imported,field);if(value===undefined)continue;
          applyOne(tx,{targetRecordId:mixId,field,value,provider:source,sourceUrl:source.url!,observedAt:timestamp,evidence:field==='cover'?'curated':'parsed',matchExplanation:'Preserved browser selection; original field provenance remains attributable'});
        }
        if(entityLinks.length)applyOne(tx,{targetRecordId:mixId,field:'people',value:entityLinks,provider:source,sourceUrl:source.url!,observedAt:timestamp,evidence:'parsed',matchExplanation:'Legacy performer identities and recording links require independent confirmation'});
        if(eventIds.length)applyOne(tx,{targetRecordId:mixId,field:'eventIds',value:eventIds,provider:source,sourceUrl:source.url!,observedAt:timestamp,evidence:'parsed',matchExplanation:'Legacy event identity and recording relationship require confirmation'});
        restoreLegacyDecisions(tx,legacy,mixId,refs,actor);
      });
      outcome.recordId=mixId;result.legacyIds[legacy.id]=mixId;outcome.status=existing?'existing':'imported';
      if(existing)result.existing++;else result.imported++;
      if(legacy.waveform&&typeof legacy.waveform==='object') {
        const waveform=legacy.waveform as Record<string,unknown>,dataUrl=typeof waveform.imageDataUrl==='string'?waveform.imageDataUrl:'';
        const match=/^data:image\/png;base64,([A-Za-z0-9+/=]+)$/.exec(dataUrl);
        if(!match||typeof waveform.sourceUrl!=='string')throw new Error('Waveform PNG payload or source URL is invalid');
        persistWaveform(repo,mixId,Buffer.from(match[1],'base64'),waveform.sourceUrl);
      }
    } catch(error) {
      outcome.errors.push(error instanceof Error?error.message:'Legacy migration failed');
      if(outcome.recordId)outcome.status='partial';else {result.rejected++;outcome.status='rejected';}
    }
  }
  return repo.saveMigrationBatch(MigrationResultSchema.parse(result));
}
