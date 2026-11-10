import test from 'node:test';
import assert from 'node:assert/strict';
import {bindPlayer,loadPlayer,resetPlayer} from '../src/player.js';

// Model a backend that has metadata but waits for play() before requesting frames.
// No browser or decoder is needed to exercise the actual controller's public events.
function fixture({invoke=async()=>{},native=false}={}){
 const elements=new Map();
 class Video extends EventTarget {
  paused=true;currentTime=0;duration=NaN;readyState=1;volume=1;muted=false;seeking=false;error=null;networkState=3;canPlayType(){return "probably"};playRequests=0;attributes=new Map();
  set src(value){this.attributes.set('src',value);this.currentSrc=value;}
  get src(){return this.attributes.get('src');}
  hasAttribute(key){return this.attributes.has(key);}
  removeAttribute(key){this.attributes.delete(key);this.currentSrc='';}
  load(){this.error=null;this.dispatchEvent(new Event('loadstart'));}
  pause(){this.paused=true;this.dispatchEvent(new Event('pause'));}
  async play(){this.playRequests++;this.paused=false;this.dispatchEvent(new Event('play'));}
 }
 const video=new Video(),dialog=new EventTarget();dialog.classList={values:new Set(),add(value){this.values.add(value)},remove(value){this.values.delete(value)},contains(value){return this.values.has(value)}};dialog.open=true;dialog.dataset={name:'one.mp4'};dialog.querySelector=selector=>{if(selector==='video')return video;if(!elements.has(selector))elements.set(selector,{setAttribute(){},textContent:'',disabled:false});return elements.get(selector);};
 globalThis.document={activeElement:null};bindPlayer(dialog,{invoke,native});resetPlayer(dialog);
 return {dialog,video,q:id=>dialog.querySelector('#'+id),close:()=>{dialog.open=false;dialog.dispatchEvent(new Event('close'));}};
}
test('metadata-only playback can request frames, pause, and unload on close',async()=>{
 const {dialog,video,q,close}=fixture();
 try{
  assert.equal(q('video-play').disabled,true);
  loadPlayer(dialog,'http://127.0.0.1:1234/private/one.mp4');
  assert.equal(q('video-play').disabled,false);
  assert.equal(q('video-forward').disabled,true);
  await q('video-play').onclick();assert.equal(video.playRequests,1);assert.equal(video.paused,false);
  await q('video-play').onclick();assert.equal(video.paused,true);
  close();assert.equal(video.hasAttribute('src'),false);assert.equal(video.paused,true);
 }finally{close();}
});
test('autoplay requests frames immediately and metadata enables seeking',async()=>{
 const {dialog,video,q,close}=fixture();
 try{
  loadPlayer(dialog,'http://127.0.0.1:1234/private/one.mp4',true);
  assert.equal(video.playRequests,1);
  video.duration=12;video.dispatchEvent(new Event('loadedmetadata'));
  assert.equal(q('video-forward').disabled,false);q('video-forward').onclick();assert.equal(video.currentTime,5);
  resetPlayer(dialog);assert.equal(q('video-play').disabled,true);assert.equal(video.hasAttribute('src'),false);
 }finally{close();}
});

test('compatible preview uses a local Blob and releases it on close',async()=>{
 const calls=[];const {dialog,video,q,close}=fixture({native:true,invoke:async(command)=>{calls.push(command);if(command==='preview_bytes')return new Uint8Array([0x1a,0x45,0xdf,0xa3]).buffer;}});
 try{
  loadPlayer(dialog,'http://127.0.0.1:1234/private/one.mp4');video.error={code:4,message:'Backend rejected HTTP media'};video.onerror();
  await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual(calls.slice(0,3),['report_playback_error','prepare_preview','preview_bytes']);
  assert.ok(video.src.startsWith('blob:'));const blobUrl=video.src;
  assert.equal((await fetch(blobUrl)).status,200);
  close();await assert.rejects(fetch(blobUrl));
 }finally{close();}
});
test('enlarging uses fullscreen and Escape shrinks without closing the clip',async()=>{
 const calls=[];const {dialog,q,close}=fixture({native:true,invoke:async(command,args)=>{calls.push(args.action);return false;}});
 try{
  await q('video-expand').onclick();assert.equal(dialog.classList.contains('expanded'),true);assert.deepEqual(calls,['state','fullscreen']);
  const cancel=new Event('cancel',{cancelable:true});dialog.dispatchEvent(cancel);
  assert.equal(cancel.defaultPrevented,true);assert.equal(dialog.open,true);assert.equal(dialog.classList.contains('expanded'),false);assert.equal(calls.at(-1),'windowed');
 }finally{close();}
});
