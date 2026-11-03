import { randomUUID } from 'node:crypto';
import { ImportCandidateSchema, ImportResultSchema, RecordingDateSchema, normalizeName, type ImportCandidate,type ImportResult,type MixRecord,type EntityRecord,type FieldClaim } from '@syco23/catalog-domain';
import { applyOne } from './merge.js';
import type { CatalogRepository } from './repository.js';
import type { CuratorActor } from '../auth/curator.js';

export function importCandidate(repo:CatalogRepository,input:ImportCandidate,actor:CuratorActor):ImportResult {
  const candidate=ImportCandidateSchema.parse(input),ref=candidate.source,now=new Date().toISOString();
  let outcome:ImportResult['outcome']='created',id='';
  repo.transaction(tx=>{
    const existing=tx.findBySource(ref);
    if(existing){id=existing;outcome='existing';return;}
    id=`mix_${randomUUID()}`;
    const people:MixRecord['people']=[];
    // Discovery artist strings on upload platforms can be uploader names. They require explicit adapter facts.
    if(['freeteknomusic','archiveorg'].includes(candidate.provider)) for(const [role,names] of [['artist',candidate.artists],['crew',candidate.crews]] as const) {
      for(const name of [...new Map(names.filter(value=>value.trim()).map(value=>[normalizeName(value),value])).values()]) {
        const entity:EntityRecord={kind:'entity',id:`entity_${randomUUID()}`,createdAt:now,updatedAt:now,revision:1,verification:'proposed',reviewState:'review',displayName:name,roles:[role],aliases:[],assets:[],providerRefs:[],[role]:{}};
        tx.saveRecord(entity);
        people.push({entityId:entity.id,role});
        applyOne(tx,{targetRecordId:entity.id,field:'displayName',value:name,provider:ref,sourceUrl:ref.url!,observedAt:now,evidence:'parsed',matchExplanation:`Discovery performer text requires identity confirmation: ${name}`});
      }
    }
    const record:MixRecord={kind:'mix',id,createdAt:now,updatedAt:now,revision:1,verification:'curator-confirmed',reviewState:'ready',title:candidate.title,people:[],eventIds:[],genres:[],styles:[],assets:[],sources:[{...ref,addedAt:now,...(candidate.uploadedAt&&RecordingDateSchema.safeParse({value:candidate.uploadedAt,precision:candidate.uploadedAt.length===4?'year':candidate.uploadedAt.length===7?'month':'day'}).success?{uploadDate:RecordingDateSchema.parse({value:candidate.uploadedAt,precision:candidate.uploadedAt.length===4?'year':candidate.uploadedAt.length===7?'month':'day'})}:{})}]};
    tx.saveRecord(record);tx.addProviderSource(id,ref);
    if(people.length)applyOne(tx,{targetRecordId:id,field:'people',value:people,provider:ref,sourceUrl:ref.url!,observedAt:now,evidence:'parsed',matchExplanation:'Parsed performer/crew links require confirming each identity and the recording relationship'});
    const claims:Array<[string,unknown,FieldClaim['evidence']]>=[['title',candidate.title,'curated']];
    if(candidate.durationMs)claims.push(['durationMs',candidate.durationMs,'direct']);
    if(candidate.description)claims.push(['description',candidate.description,'direct']);
    if(candidate.genres.length)claims.push(['genres',candidate.genres,'direct']);
    if(candidate.recordedAt){const value=candidate.recordedAt;const date=RecordingDateSchema.safeParse({value,precision:value.length===4?'year':value.length===7?'month':'day'});if(date.success)claims.push(['recordingDate',date.data,candidate.fieldEvidence?.recordingDate??candidate.fieldEvidence?.recordedAt??'parsed']);}
    if(candidate.artwork.length)claims.push(['cover',{role:'mix-cover',url:candidate.artwork[0]},'parsed']);
    for(const duplicate of tx.allRecords())if(duplicate.kind==='mix'&&duplicate.id!==id&&normalizeName(duplicate.title)===normalizeName(candidate.title)&&(!duplicate.durationMs||!candidate.durationMs||Math.abs(duplicate.durationMs-candidate.durationMs)<=Math.max(2000,candidate.durationMs*.02)))applyOne(tx,{targetRecordId:id,field:'possibleDuplicate',value:{recordId:duplicate.id,revision:duplicate.revision},provider:ref,sourceUrl:ref.url!,observedAt:now,evidence:'parsed',matchExplanation:'Similar title and available compatible duration suggest a duplicate; source identity has not been confirmed'});
    for(const [field,value,evidence] of claims) applyOne(tx,{targetRecordId:id,field,value,provider:ref,sourceUrl:ref.url!,observedAt:now,evidence,matchExplanation:evidence==='parsed'?'Parsed discovery fact requires curator confirmation':`Curator ${actor.sessionId} selected this provider resource`});
  });
  return ImportResultSchema.parse({record:repo.getRecord(id),outcome});
}
