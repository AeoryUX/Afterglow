import test from 'node:test';
import assert from 'node:assert/strict';
import {bindPlayer,loadPlayer,resetPlayer} from '../src/player.js';

// Model a backend that has metadata but waits for play() before requesting frames.
// No browser or decoder is needed to exercise the actual controller's public events.
function fixture(){
 const elements=new Map();
 class Video extends EventTarget {
  paused=true;currentTime=0;duration=NaN;readyState=1;volume=1;muted=false;seeking=false;error=null;playRequests=0;attributes=new Map();
  set src(value){this.attributes.set('src',value);this.currentSrc=value;}
  get src(){return this.attributes.get('src');}
  hasAttribute(key){return this.attributes.has(key);}
  removeAttribute(key){this.attributes.delete(key);this.currentSrc='';}
  load(){this.dispatchEvent(new Event('loadstart'));}
  pause(){this.paused=true;this.dispatchEvent(new Event('pause'));}
  async play(){this.playRequests++;this.paused=false;this.dispatchEvent(new Event('play'));}
 }
 const video=new Video(),dialog=new EventTarget();dialog.open=true;dialog.dataset={name:'one.mp4'};dialog.querySelector=selector=>{if(selector==='video')return video;if(!elements.has(selector))elements.set(selector,{setAttribute(){},textContent:'',disabled:false});return elements.get(selector);};
 globalThis.document={activeElement:null};bindPlayer(dialog,{invoke:async()=>{},native:false});resetPlayer(dialog);
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
