import assert from 'node:assert/strict';import test from 'node:test';import {validateSearchQuery} from './http.js';
test('discovery query validation rejects malformed and excessive provider controls',()=>{
 for(const query of [null,{q:42},{q:'x',limit:50000},{url:'file:///private'},{q:'x',maxDepth:-1},{q:'x',unknown:'control'}])assert.throws(()=>validateSearchQuery(query));
 assert.deepEqual(validateSearchQuery({q:'DJ Live',limit:20,maxDepth:2}),{q:'DJ Live',limit:20,maxDepth:2});
});
