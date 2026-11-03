import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable } from 'node:stream';
import type { IncomingMessage, ServerResponse } from 'node:http';
import type { MixRecord } from '@syco23/catalog-domain';
import { openCatalog } from './repository.js';
import { createCuratorAuth, hashCuratorPassword } from '../auth/curator.js';
import { handleCatalogRoute } from './routes.js';
const stamp='2026-10-03T12:00:00.000Z';
const mix:MixRecord={kind:'mix',id:'mix_api_gaps_00000001',title:'Original',createdAt:stamp,updatedAt:stamp,revision:1,verification:'source-confirmed',reviewState:'ready',people:[],eventIds:[],genres:[],styles:[],assets:[],sources:[]};
async function fixture(fn:(repo:ReturnType<typeof openCatalog>,call:(method:string,url:string,input?:unknown)=>Promise<{status:number;body:any}>)=>Promise<void>){
 const dir=mkdtempSync(join(tmpdir(),'syco23-api-gaps-')); const repo=openCatalog(join(dir,'catalog.sqlite'));
 const old=process.env.CORS_ORIGIN;process.env.CORS_ORIGIN='http://localhost:5173';
 const auth=createCuratorAuth(repo,hashCuratorPassword('local-api-test-password'));const cookie=auth.login('local-api-test-password').split(';')[0];
 repo.transaction(tx=>tx.saveRecord(mix));
 const call=async(method:string,url:string,input?:unknown)=>{const req=Readable.from(input===undefined?[]:[JSON.stringify(input)]) as IncomingMessage;req.method=method;req.url=url;req.headers={origin:'http://localhost:5173',cookie};let status=0;let body:any;const res={setHeader(){},writeHead(code:number){status=code;},end(value:string){body=JSON.parse(value||'null');}} as unknown as ServerResponse; await handleCatalogRoute({repository:repo,auth},req,res);return{status,body};};
 try{await fn(repo,call);}finally{repo.close();rmSync(dir,{recursive:true,force:true});if(old===undefined)delete process.env.CORS_ORIGIN;else process.env.CORS_ORIGIN=old;}
}
test('null JSON mutation inputs are validation errors',()=>fixture(async(_repo,call)=>{for(const [method,path] of [['PATCH',`/api/catalog/records/${mix.id}`],['POST','/api/catalog/migrate'],['POST','/api/catalog/review/claim_invalid/decision']])assert.equal((await call(method,path,null)).status,400);}));
test('invalid percent encoding returns a validation response without rejecting the handler',()=>fixture(async(_repo,call)=>{assert.equal((await call('GET','/api/catalog/records/%ZZ')).status,400);}));
test('curator patches replace selected evidence with an attributable curated claim',()=>fixture(async(repo,call)=>{const result=await call('PATCH',`/api/catalog/records/${mix.id}`,{expectedRevision:1,patch:{title:'Curated'}});assert.equal(result.status,200);assert.ok(result.body.selectedEvidence.title); const claims=repo.listClaims(mix.id);assert.equal(claims.length,1);assert.equal(claims[0].claim.evidence,'curated');assert.equal(claims[0].claim.value,'Curated');}));
test('explicit entity creation returns a shared confirmed identity and related mixes',()=>fixture(async(repo,call)=>{const created=await call('POST','/api/catalog/records',{kind:'entity',displayName:'DJ Live',roles:['artist'],aliases:['DJ L']});assert.equal(created.status,201);assert.equal(created.body.verification,'curator-confirmed');const entity=created.body;repo.transaction(tx=>tx.saveRecord({...mix,revision:2,people:[{entityId:entity.id,role:'artist'}]},1));const related=await call('GET',`/api/catalog/records/${entity.id}/related`);assert.equal(related.status,200);assert.equal(related.body[0].id,mix.id);}));

test('unexpected repository errors cannot leak messages even when they contain not found',()=>fixture(async(repo,call)=>{
 const get=repo.getRecord;repo.getRecord=()=>{throw new Error('Private credential at /worker/private.env not found');};
 try{const result=await call('GET',`/api/catalog/records/${mix.id}`);assert.equal(result.status,500);assert.equal(result.body.error,'Request failed');}finally{repo.getRecord=get;}
}));

test('removing an entity role and its detail is one atomic curator correction',()=>fixture(async(_repo,call)=>{
 const created=await call('POST','/api/catalog/records',{kind:'entity',displayName:'DJ Crew',roles:['artist','crew'],artist:{realName:'Person'},crew:{websiteUrls:['https://crew.example.org']}});assert.equal(created.status,201);
 const changed=await call('PATCH',`/api/catalog/records/${created.body.id}`,{expectedRevision:created.body.revision,patch:{roles:['crew'],artist:null}});assert.equal(changed.status,200);assert.deepEqual(changed.body.roles,['crew']);assert.equal(changed.body.artist,undefined);assert.ok(changed.body.selectedEvidence.artist);
}));
