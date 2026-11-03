import assert from 'node:assert/strict';import test from 'node:test';
import {YouTubeProvider} from './youtube.js';import {ArchiveOrgProvider} from './archiveorg.js';import {SoundCloudProvider} from './soundcloud.js';
test('known YouTube IDs hydrate metadata directly with configured API capability',async(context)=>{
 const old=process.env.YOUTUBE_API_KEY;process.env.YOUTUBE_API_KEY='test-worker-key';
 context.mock.method(globalThis,'fetch',async(input:string|URL|Request)=>String(input).includes('/videos?')?Response.json({items:[{id:'vi5miMVpmuI',snippet:{title:'Known mix',channelTitle:'Archive Uploader',description:'Source biography',thumbnails:{high:{url:'https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg'}}},contentDetails:{duration:'PT44M'}}]}):Response.json({title:'Known mix',author_name:'Archive Uploader'}));
 try{const [mix]=await new YouTubeProvider().search({url:'https://www.youtube.com/watch?v=vi5miMVpmuI'});assert.equal(mix.durationMs,2640000);assert.equal(mix.description,'Source biography');assert.deepEqual(mix.artists,[]);assert.equal(mix.uploader,'Archive Uploader');}finally{if(old===undefined)delete process.env.YOUTUBE_API_KEY;else process.env.YOUTUBE_API_KEY=old;}
});
test('known Archive.org item IDs fetch metadata without relying on name search',async(context)=>{
 context.mock.method(globalThis,'fetch',async(input:string|URL|Request)=>String(input).includes('/metadata/known-id')?Response.json({metadata:{title:'Known mix',creator:'DJ Live',description:'Explicit source description'},files:[{name:'audio.mp3',length:2640}]}):Response.json({response:{docs:[]}}));
 const [mix]=await new ArchiveOrgProvider().search({url:'https://archive.org/details/known-id'});assert.equal(mix.description,'Explicit source description');assert.equal(mix.durationMs,2640000);
});
test('known SoundCloud links resolve metadata without treating uploader as performer',async(context)=>{
 const old=process.env.SOUNDCLOUD_ACCESS_TOKEN;process.env.SOUNDCLOUD_ACCESS_TOKEN='test-worker-token';context.mock.method(globalThis,'fetch',async(input:string|URL|Request)=>String(input).includes('/resolve?')?Response.json({id:42,title:'Known mix',metadata_artist:'DJ Live',user:{username:'Archive Uploader'},duration:2640000,description:'Source description',permalink_url:'https://soundcloud.com/archive/known-mix'}):Response.json({collection:[]}));
 try{const [mix]=await new SoundCloudProvider().search({url:'https://soundcloud.com/archive/known-mix'});assert.equal(mix.description,'Source description');assert.deepEqual(mix.artists,['DJ Live']);assert.equal(mix.uploader,'Archive Uploader');}finally{if(old===undefined)delete process.env.SOUNDCLOUD_ACCESS_TOKEN;else process.env.SOUNDCLOUD_ACCESS_TOKEN=old;}
});
