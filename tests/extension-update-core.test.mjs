import {test} from 'node:test';
import assert from 'node:assert/strict';
import {validUpdate,versionIsNewer,readUpdateBody,verifyLoadedFolder,installUpdate} from '../extensions/cinematch/update-core.js';
const encode=value=>typeof value==='string'?new TextEncoder().encode(value):new Uint8Array(value);
class MemoryFolder{
  constructor(state={files:new Map(),directories:new Set(['']),writes:[],fail:null},prefix=''){this.state=state;this.prefix=prefix;}
  async getDirectoryHandle(name,{create=false}={}){const path=this.prefix+name+'/';if(!this.state.directories.has(path)){if(!create)throw new DOMException('Missing','NotFoundError');this.state.directories.add(path);}return new MemoryFolder(this.state,path);}
  async getFileHandle(name,{create=false}={}){
    const path=this.prefix+name,state=this.state;
    if(!state.files.has(path)){if(!create)throw new DOMException('Missing','NotFoundError');state.files.set(path,encode(''));}
    return {getFile:async()=>({size:state.files.get(path).length,arrayBuffer:async()=>state.files.get(path).slice().buffer}),createWritable:async()=>{
      let pending;return {write:async value=>{if(state.fail===path){state.fail=null;throw new Error('Disk full');}pending=encode(value);},close:async()=>{state.writes.push(path);state.files.set(path,pending);},abort:async()=>{}};
    }};
  }
  async removeEntry(name){this.state.files.delete(this.prefix+name);}
}
test('update metadata rejects oversized downloads and invalid versions',()=>{
  const info={version:'3.19.2',sha256:'a'.repeat(64),size:100};
  assert.equal(validUpdate(info).version,'3.19.2');assert.equal(versionIsNewer('3.19.2','3.19.1'),true);
  for(const patch of [{size:51*1024*1024},{size:0},{version:'3.19.x'},{version:'99999999999999999999.1.0'},{sha256:'invalid'}])assert.throws(()=>validUpdate({...info,...patch}));
});
test('download limit cancels a streaming body before collecting all its bytes',async()=>{
  let cancelled=false;
  const body=new ReadableStream({pull(controller){controller.enqueue(new Uint8Array(8));},cancel(){cancelled=true;}});
  await assert.rejects(()=>readUpdateBody(new Response(body),{maxBytes:10}),/dimensione/);assert.equal(cancelled,true);
  await assert.rejects(()=>readUpdateBody(new Response('short'),{maxBytes:100,expectedSize:10}),/dimensione/);
  assert.equal(new TextDecoder().decode(await readUpdateBody(new Response('ok'),{maxBytes:2,expectedSize:2})),'ok');
});
test('selected directory must be the folder actually served by this extension',async()=>{
  const folder=new MemoryFolder();
  const fetcher=async url=>{const name=new URL(url).pathname.slice(1);return new Response(folder.state.files.get(name)??'',{status:folder.state.files.has(name)?200:404});};
  await verifyLoadedFolder(folder,name=>'https://extension.test/'+name,{fetcher});assert.equal(folder.state.files.size,0,'proof file is removed');
  await assert.rejects(()=>verifyLoadedFolder(folder,name=>'https://extension.test/'+name,{fetcher:async()=>new Response('',{status:404})}),/non è quella caricata/);
  assert.equal(folder.state.files.size,0);
});
test('updates skip identical files, preserve personal files and write the manifest last',async()=>{
  const folder=new MemoryFolder();folder.state.files.set('same.js',encode('same'));folder.state.files.set('manifest.json',encode('old'));folder.state.files.set('personal.json',encode('private'));
  const result=await installUpdate(folder,[{path:'manifest.json',data:encode('new')},{path:'same.js',data:encode('same')},{path:'new.js',data:encode('added')}]);
  assert.deepEqual(result,{written:2,unchanged:1});assert.deepEqual(folder.state.writes,['new.js','manifest.json']);assert.equal(new TextDecoder().decode(folder.state.files.get('personal.json')),'private');
});
test('failed writes restore replaced files and remove newly created payload files',async()=>{
  const folder=new MemoryFolder();folder.state.files.set('a.js',encode('old-a'));folder.state.files.set('manifest.json',encode('old-manifest'));folder.state.fail='b.js';
  await assert.rejects(()=>installUpdate(folder,[{path:'a.js',data:encode('new-a')},{path:'new.js',data:encode('new')},{path:'b.js',data:encode('failure')},{path:'manifest.json',data:encode('new-manifest')}]),/ripristinati/);
  assert.equal(new TextDecoder().decode(folder.state.files.get('a.js')),'old-a');assert.equal(new TextDecoder().decode(folder.state.files.get('manifest.json')),'old-manifest');assert.equal(folder.state.files.has('new.js'),false);assert.equal(folder.state.files.has('b.js'),false);
});
test('an unreadable file or insufficient backup budget aborts before any write',async()=>{
  const folder=new MemoryFolder();folder.state.files.set('a.js',encode('large-old-file'));
  await assert.rejects(()=>installUpdate(folder,[{path:'a.js',data:encode('new')}],{maxBackupBytes:2}),/annullato/);assert.deepEqual(folder.state.writes,[]);
});
