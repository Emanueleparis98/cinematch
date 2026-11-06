import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createDebridClient,chooseVideo,torrentHash,enableDebridStreams} from '../extensions/cinematch/debrid.js';
import {parseStreams} from '../extensions/cinematch/streams.js';
const hash='a'.repeat(40),key='personal-test-key';
test('torrent hash survives addon parsing, invalid hashes stay unavailable',()=>{
  const streams=parseStreams({streams:[{infoHash:hash,fileIdx:2},{infoHash:'invalid'}]});
  assert.equal(streams[0].infoHash,hash);assert.equal(streams[0].fileIdx,2);
  assert.equal(enableDebridStreams(streams,true)[0].playable,true);
  assert.equal(enableDebridStreams(streams,true)[1].playable,false);
  assert.throws(()=>torrentHash({infoHash:'javascript:bad'}));
});
test('file selection excludes samples and avoids guessing the wrong episode',()=>{
  const files=[{id:1,path:'sample.mkv',bytes:9999},{id:2,path:'show.S01E02.mkv',bytes:200},{id:3,path:'show.S01E03.mkv',bytes:300}];
  assert.equal(chooseVideo(files,{season:1,episode:2}).id,2);
  assert.throws(()=>chooseVideo(files,{season:2,episode:5}));
  assert.throws(()=>chooseVideo([files[1]],{season:1,episode:3}));
  assert.equal(chooseVideo(files,{fileIdx:2}).id,3);
});
test('Real-Debrid uses only its API for the key and converts the selected torrent file',async()=>{
  const paths=[];let selected=false;
  const client=createDebridClient({provider:'rd',key,fetcher:async(url,init)=>{
    assert.equal(new URL(url).hostname,'api.real-debrid.com');assert.equal(init.headers.Authorization,'Bearer '+key);assert.equal(init.redirect,'error');assert.equal(url.includes(key),false);
    const path=new URL(url).pathname;paths.push(path);
    if(path.endsWith('addMagnet')){assert.equal(init.body.get('magnet'),'magnet:?xt=urn:btih:'+hash);return Response.json({id:'abc'});}
    if(path.includes('selectFiles')){selected=true;assert.equal(init.body.get('files'),'2');return new Response(null,{status:204});}
    if(path.includes('/info/'))return Response.json({status:selected?'downloaded':'waiting_files_selection',files:[{id:1,path:'sample.mkv',selected:0},{id:2,path:'movie.mkv',selected:1}],links:selected?['https://host.test/file']:[]});
    return Response.json({download:'https://cdn.test/movie.mkv'});
  }});
  const result=await client.resolve({infoHash:hash});assert.equal(result.url,'https://cdn.test/movie.mkv');assert.equal(paths.length,5);
});
test('TorBox uses cached-only creation and returns a CDN URL without the API key',async()=>{
  const client=createDebridClient({provider:'torbox',key,fetcher:async(url,init)=>{
    const u=new URL(url);assert.equal(u.hostname,'api.torbox.app');
    let data;
    if(u.pathname.endsWith('createtorrent')){assert.equal(init.body.get('add_only_if_cached'),'true');data={torrent_id:12};}
    else if(u.pathname.endsWith('mylist'))data=[{id:12,download_finished:true,files:[{id:0,name:'movie.mp4'}]}];
    else{assert.equal(u.searchParams.get('token'),key);assert.equal(u.searchParams.get('redirect'),'false');data='https://cdn.test/movie.mp4';}
    return Response.json({success:true,data});
  }});
  assert.equal((await client.resolve({infoHash:hash})).url,'https://cdn.test/movie.mp4');
});
test('API failures never echo credentials or private server response bodies',async()=>{
  const client=createDebridClient({provider:'rd',key,fetcher:async()=>new Response(key,{status:401})});
  await assert.rejects(()=>client.verify(),error=>!error.message.includes(key)&&/non valida/.test(error.message));
});
