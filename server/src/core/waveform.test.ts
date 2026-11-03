import assert from 'node:assert/strict';
import test from 'node:test';
import {mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import type {MixRecord} from '@syco23/catalog-domain';
import {openCatalog} from '../catalog/repository.js';
import {WaveformQueue} from './waveform.js';
test('failed catalog analyses remain visible and retryable after worker recreation',async(context)=>{
 const dir=mkdtempSync(join(tmpdir(),'syco23-analysis-'));const path=join(dir,'catalog.sqlite');const repo=openCatalog(path);
 const stamp='2026-10-03T12:00:00.000Z';const mix:MixRecord={kind:'mix',id:'mix_analysis_durable_00001',title:'Analysis',createdAt:stamp,updatedAt:stamp,revision:1,verification:'source-confirmed',reviewState:'ready',people:[],eventIds:[],genres:[],styles:[],assets:[],sources:[]};repo.transaction(tx=>tx.saveRecord(mix));
 context.mock.method(globalThis,'fetch',async()=>new Response('missing',{status:404}));
 try{const queue=new WaveformQueue(repo);const job=queue.create('https://archive.org/download/example/audio.mp3',mix.id);for(let i=0;i<20&&job.state==='running';i++)await new Promise(resolve=>setTimeout(resolve,5));assert.equal(job.state,'error');repo.close();const reopened=openCatalog(path);try{const restored=new WaveformQueue(reopened);assert.equal(restored.get(job.id)?.state,'error');assert.ok(restored.create(job.sourceUrl,mix.id).id);}finally{await new Promise(resolve=>setTimeout(resolve,20));reopened.close();}}finally{rmSync(dir,{recursive:true,force:true});}
});

test('audio analysis rejects advertised audio larger than 250 MiB before decoding',async(context)=>{
 context.mock.method(globalThis,'fetch',async()=>new Response('audio',{headers:{'content-length':String(251*1024*1024)}}));
 const job=new WaveformQueue().create('https://archive.org/download/example/audio.mp3');for(let i=0;i<20&&job.state==='running';i++)await new Promise(resolve=>setTimeout(resolve,5));assert.equal(job.state,'error');assert.match(job.error??'',/250 MiB/);
});
test('audio analysis rejects redirects beyond supported provider hosts',async(context)=>{
 context.mock.method(globalThis,'fetch',async()=>new Response(null,{status:302,headers:{location:'https://private.example.org/audio.mp3'}}));
 const job=new WaveformQueue().create('https://archive.org/download/example/audio.mp3');for(let i=0;i<20&&job.state==='running';i++)await new Promise(resolve=>setTimeout(resolve,5));assert.equal(job.state,'error');assert.match(job.error??'',/public audio/);
});
