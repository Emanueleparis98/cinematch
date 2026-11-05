import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createPlayer} from '../extensions/cinematch/player-engine.js';
class Surface{
  constructor(){this.children=[];this.style={};this.ownerDocument={createElement:()=>new Surface()};}
  append(child){this.children.push(child);child.parent=this;}
  replaceChildren(){this.children=[];}
  remove(){if(this.parent)this.parent.children=this.parent.children.filter(child=>child!==this);}
}
class Video extends EventTarget{
  canPlayType(){return '';}
  pause(){}
  load(){}
  removeAttribute(){}
}
const deferred=()=>{let resolve;const promise=new Promise(done=>{resolve=done;});return {promise,resolve};};
const probeMedia=async()=>({kind:'matroska',codecs:[]});
const stream=name=>({url:`https://video.test/${name}.mkv`,kind:'video',subtitles:[]});
globalThis.chrome={permissions:{request:async()=>true,contains:async()=>true}};
test('a late decoder cannot overwrite the active decoder, emit stale times or clear its surface',async()=>{
  const oldFactory=deferred(),newStarted=deferred(),factoryEntered=deferred(),container=new Surface(),times=[],pauses=[],notices=[];let oldOptions;
  const factory=async options=>{
    const name=options.url.includes('/old.')?'old':'new';
    if(name==='old'){oldOptions=options;factoryEntered.resolve();await oldFactory.promise;}
    return {volume(){},start:async()=>{if(name==='new')newStarted.resolve();},pause:async()=>pauses.push(name),destroy:async()=>options.container.replaceChildren(),get playing(){return true;}};
  };
  const player=createPlayer({video:new Video(),codecContainer:container,probeMedia,codecFactory:factory,onTime:value=>times.push(value),onStatus:value=>notices.push(value)});
  const old=player.load(stream('old'));await factoryEntered.promise;
  await player.load(stream('new'));await newStarted.promise;
  const activeSurface=container.children[0];oldFactory.resolve();await old;
  const noticeCount=notices.length;oldOptions.onTime(99,100);oldOptions.onAudioUnavailable();assert.equal(notices.length,noticeCount);
  assert.deepEqual(times,[]);assert.equal(container.children[0],activeSurface);assert.equal(player.stream.url,stream('new').url);
  await player.pause();assert.deepEqual(pauses,['new']);
  await player.destroy();
});
test('a superseded start completion keeps the newer decoder available for controls',async()=>{
  const started=deferred(),finish=deferred(),pauses=[];
  const factory=async options=>{
    const name=options.url.includes('/old.')?'old':'new';
    return {volume(){},start:async()=>{if(name==='old'){started.resolve();await finish.promise;}},pause:async()=>pauses.push(name),destroy:async()=>options.container.replaceChildren(),get playing(){return true;}};
  };
  const player=createPlayer({video:new Video(),codecContainer:new Surface(),probeMedia,codecFactory:factory});
  const old=player.load(stream('old'));await started.promise;await player.load(stream('new'));finish.resolve();await old;
  pauses.length=0;await player.pause();assert.deepEqual(pauses,['new']);await player.destroy();
});
test('stopping while a probe is pending prevents decoder startup',async()=>{
  const probe=deferred();let starts=0;
  const player=createPlayer({video:new Video(),codecContainer:new Surface(),probeMedia:()=>probe.promise,codecFactory:async()=>{starts++;}});
  const pending=player.load(stream('old'));await new Promise(resolve=>setTimeout(resolve,0));await player.destroy();probe.resolve({kind:'matroska',codecs:[]});await pending;assert.equal(starts,0);
});
