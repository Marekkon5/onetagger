import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { openCatalog } from './repository.js';
import { importCandidate } from './identity.js';
import { applyClaims, decideReview } from './merge.js';
import type { MixRecord, ImportCandidate } from '@syco23/catalog-domain';
const now='2026-10-03T12:00:00.000Z';
const mix=(id:string):MixRecord=>({kind:'mix',id,createdAt:now,updatedAt:now,revision:1,verification:'curator-confirmed',reviewState:'ready',title:id,people:[],eventIds:[],genres:[],styles:[],assets:[],sources:[]});
function run(fn:(repo:ReturnType<typeof openCatalog>)=>void) {const dir=mkdtempSync(join(tmpdir(),'catalog-gaps-'));const repo=openCatalog(join(dir,'catalog.sqlite'));try{fn(repo);}finally{repo.close();rmSync(dir,{recursive:true,force:true});}}

test('record writes reject dangling typed references and wrong membership roles',()=>run(repo=>{
  assert.throws(()=>repo.transaction(tx=>tx.saveRecord({...mix('mix_dangling'),people:[{entityId:'entity_missing',role:'artist'}]})),/relationship|unknown|reference/i);
}));
test('filename performer proposals have durable Review and confirm only after acceptance',()=>run(repo=>{
  const candidate:ImportCandidate={provider:'freeteknomusic',title:'DJ Live session',artists:['DJ Live'],crews:['Live Crew'],genres:[],artwork:[],source:{provider:'freeteknomusic',resourceType:'recording',externalId:'performer-1',url:'https://archive.freeteknomusic.org/performer-1.mp3'},confidence:.9,reasons:['filename']};
  const {record}=importCandidate(repo,candidate,{sessionId:'curator'});
  assert.equal(record.kind==='mix'?record.people.length:0,0);
  assert.ok(repo.listReview().some(item=>item.field==='people'));
  assert.equal(repo.listIndex('artist',{page:1,pageSize:25}).total,0);
  const item=repo.listReview().find(item=>item.field==='displayName'&&repo.getRecord(item.targetRecordId)?.kind==='entity'&&(repo.getRecord(item.targetRecordId) as any).roles.includes('artist'));assert.ok(item);
  const accepted=decideReview(repo,item.id,'accept',item.recordRevision,'curator');
  assert.equal(accepted.verification,'curator-confirmed');
  assert.equal(repo.listIndex('artist',{page:1,pageSize:25}).total,1);
}));
test('duplicate merge preserves sources/evidence/review/aliases and refuses stale revisions',async()=>{
  const {mergeRecords}=await import('./merge.js');
  run(repo=>{
    const first=mix('mix_survivor');const second={...mix('mix_duplicate'),description:'Useful missing description'};
    repo.transaction(tx=>{tx.saveRecord(first);tx.saveRecord(second);tx.addProviderSource(second.id,{provider:'youtube',resourceType:'video',externalId:'merged-source'});tx.addLegacyAlias('old-duplicate',second.id);});
    applyClaims(repo,[{targetRecordId:second.id,field:'title',value:'Other title',provider:{provider:'youtube',resourceType:'video',externalId:'merged-source'},sourceUrl:'https://youtube.com/watch?v=merged-source',observedAt:now,evidence:'parsed',matchExplanation:'Uncertain title'}]);
    assert.throws(()=>mergeRecords(repo,first.id,second.id,[2,1],{sessionId:'curator'}),/revision/i);
    const result=mergeRecords(repo,first.id,second.id,[repo.getRecord(first.id)!.revision,repo.getRecord(second.id)!.revision],{sessionId:'curator'});
    assert.equal(result.kind==='mix'?result.description:undefined,'Useful missing description');
    assert.equal(repo.findByProvider({provider:'youtube',resourceType:'video',externalId:'merged-source'}),first.id);
    assert.equal(repo.getRecord(second.id)?.id,first.id);assert.equal(repo.getRecord('old-duplicate')?.id,first.id);
    assert.ok(repo.listReview().every(item=>item.targetRecordId===first.id));
    assert.ok(repo.listReview().some(item=>item.currentValue===first.title));
  });
});

test('similar recordings create possible-duplicate Review without silently linking source identities',()=>run(repo=>{
  const candidate:ImportCandidate={provider:'archiveorg',title:'Identical recording title',artists:[],crews:[],genres:[],artwork:[],durationMs:100000,source:{provider:'archiveorg',resourceType:'recording',externalId:'possible-first',url:'https://archive.org/details/possible-first'},confidence:1,reasons:[]};
  const first=importCandidate(repo,candidate,{sessionId:'curator'});const second=importCandidate(repo,{...candidate,source:{...candidate.source,externalId:'possible-second',url:'https://archive.org/details/possible-second'}},{sessionId:'curator'});
  assert.notEqual(first.record.id,second.record.id);
  const item=repo.listReview().find(item=>item.field==='possibleDuplicate');assert.ok(item);
  assert.throws(()=>decideReview(repo,item.id,'accept',item.recordRevision,'curator'),/merge/i);
}));
