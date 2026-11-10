const players=new WeakMap();
const paths={play:'<polygon points="8 5 19 12 8 19 8 5"/>',pause:'<path d="M9 5v14M15 5v14"/>',volume:'<path d="m11 5-6 4H2v6h3l6 4zM15 8a6 6 0 0 1 0 8M18 5a10 10 0 0 1 0 14"/>',muted:'<path d="m11 5-6 4H2v6h3l6 4zM16 9l5 6M21 9l-5 6"/>',expand:'<path d="M8 3H3v5M16 3h5v5M21 16v5h-5M8 21H3v-5"/>',shrink:'<path d="M3 8h5V3M16 3v5h5M21 16h-5v5M8 21v-5H3"/>',retry:'<path d="M3 10a9 9 0 1 1 2 8M3 3v7h7"/>',external:'<path d="M14 3h7v7M10 14 21 3M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5"/>',preview:'<path d="M12 3v3M12 18v3M3 12h3M18 12h3"/><rect x="7" y="7" width="10" height="10" rx="2"/>'};
const glyph=name=>`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name]}</svg>`;
const label=(name,text)=>`${glyph(name)}<span class="control-label">${text}</span>`;
export const formatTime=value=>{const s=Math.max(0,Math.floor(Number.isFinite(value)?value:0));return `${Math.floor(s/60)}:${String(s%60).padStart(2,'0')}`;};
export function playbackMarkup(){return `<div class="video-stage" tabindex="0" role="group" aria-label="Clip player"><video playsinline preload="metadata" crossorigin="anonymous"></video><div class="player-options" role="group" aria-label="Video tools"><button id="resume-preview" class="button subtle" type="button" hidden>Resume preview</button><button id="video-retry" class="button subtle" type="button" title="Retry playback">${label('retry','Retry')}</button><button id="video-compatible" class="button subtle" type="button" title="Create a compatible preview">${label('preview','Compatible preview')}</button><button id="video-external" class="button subtle" type="button" title="Open the original externally">${label('external','Open externally')}</button></div><div class="player-controls" role="group" aria-label="Playback controls"><input id="video-seek" type="range" min="0" max="1" step="0.05" value="0" aria-label="Seek clip" disabled><div class="player-control-row"><button id="video-back" class="button subtle" type="button" aria-label="Back 5 seconds" title="Back 5 seconds" disabled>−5s</button><button id="video-play" class="button primary" type="button" aria-label="Play clip" title="Play / pause (Space)" disabled>${label('play','Play')}</button><button id="video-forward" class="button subtle" type="button" aria-label="Forward 5 seconds" title="Forward 5 seconds" disabled>+5s</button><span id="video-time">0:00 / 0:00</span><div class="player-audio"><button id="video-mute" class="button subtle" type="button" aria-label="Mute clip" title="Mute / unmute (M)">${glyph('volume')}</button><input id="video-volume" type="range" min="0" max="1" step="0.05" value="1" aria-label="Playback volume"></div><button id="video-expand" class="button subtle" type="button" aria-label="Enlarge player" title="Enlarge player (F)" aria-pressed="false">${glyph('expand')}</button></div></div></div><div class="player-status-row"><p id="playback-status" role="status">Select Play to watch the clip.</p></div><p id="playback-error" role="alert" class="playback-error" hidden></p>`;}
export function bindPlayer(dialog,{invoke,native,onWindowState=()=>{}}){
 const video=dialog.querySelector('video'),stage=dialog.querySelector('.video-stage'),q=id=>dialog.querySelector('#'+id);
 const state={autoplay:false,generation:0,source:'',tried:false,preparing:false,timer:0,hideTimer:0,objectUrl:'',expanded:false,ownsFullscreen:false,transitioning:false,suspended:false};players.set(dialog,state);
 const error=message=>{q('playback-error').hidden=!message;q('playback-error').textContent=message;};
 const status=message=>q('playback-status').textContent=message;
 const stopTimer=()=>{clearTimeout(state.timer);state.timer=0;};
 const showControls=()=>{clearTimeout(state.hideTimer);stage.classList.remove('controls-hidden');if(!video.paused&&!state.preparing&&!video.error)state.hideTimer=setTimeout(()=>{if(dialog.open&&!video.paused&&!state.preparing)stage.classList.add('controls-hidden');},2500);};
 const expanded=value=>{state.expanded=value;dialog.classList.toggle('expanded',value);q('video-expand').innerHTML=glyph(value?'shrink':'expand');q('video-expand').setAttribute('aria-label',value?'Shrink player':'Enlarge player');q('video-expand').setAttribute('title',value?'Shrink player (F / Escape)':'Enlarge player (F)');q('video-expand').setAttribute('aria-pressed',String(value));};
 state.syncWindow=snapshot=>{if(!dialog.open||state.transitioning)return;expanded(!!snapshot.fullscreen);if(!snapshot.fullscreen)state.ownsFullscreen=false;};
 const setEnlarged=async value=>{
  if(state.transitioning)return;const generation=state.generation;state.transitioning=true;q('video-expand').disabled=true;showControls();
  try{
   let snapshot={fullscreen:value};
   if(native){snapshot=await invoke('get_window_state');if(!dialog.open||generation!==state.generation)return;
    if(value&&!snapshot.fullscreen)state.ownsFullscreen=true;
    if(snapshot.fullscreen!==value)snapshot=await invoke('set_window_fullscreen',{enabled:value});
   }
   if(!dialog.open||generation!==state.generation)return;
   expanded(!!snapshot.fullscreen);if(!snapshot.fullscreen)state.ownsFullscreen=false;onWindowState(snapshot);
  }catch(e){if(dialog.open&&generation===state.generation){error('Could not change fullscreen: '+String(e));if(native)invoke('get_window_state').then(snapshot=>{if(dialog.open&&generation===state.generation){expanded(!!snapshot.fullscreen);if(!snapshot.fullscreen)state.ownsFullscreen=false;onWindowState(snapshot);}}).catch(()=>{});}}
  finally{state.transitioning=false;q('video-expand').disabled=false;}
 };
 q('video-expand').onclick=()=>setEnlarged(!state.expanded);
 dialog.addEventListener('cancel',e=>{if(state.expanded||state.transitioning){e.preventDefault();void setEnlarged(false);}});
 const update=()=>{
  const ready=video.hasAttribute('src')&&!video.error&&!state.preparing;
  for(const id of ['video-play','video-back','video-forward'])q(id).disabled=!ready||(id!=='video-play'&&(video.seeking||!Number.isFinite(video.duration)));
  q('video-seek').disabled=!video.currentSrc||!!video.error||state.preparing||!Number.isFinite(video.duration);
  q('video-compatible').disabled=state.preparing;q('video-retry').disabled=state.preparing;
  q('video-play').innerHTML=label(video.paused?'play':'pause',video.paused?'Play':'Pause');q('video-play').setAttribute('aria-label',video.paused?'Play clip':'Pause clip');
  const duration=Number.isFinite(video.duration)?video.duration:0;q('video-time').textContent=`${formatTime(video.currentTime)} / ${formatTime(duration)}`;
  q('video-seek').max=String(duration||1);if(document.activeElement!==q('video-seek'))q('video-seek').value=String(video.currentTime||0);
  q('video-mute').innerHTML=glyph(video.muted?'muted':'volume');q('video-mute').setAttribute('aria-label',video.muted?'Unmute clip':'Mute clip');
 };
 const report=detail=>{if(native)invoke('report_playback_error',{name:dialog.dataset.name,detail}).catch(()=>{});};
 const releaseBlob=()=>{if(state.objectUrl){URL.revokeObjectURL(state.objectUrl);state.objectUrl='';}};
 const load=(url,compatible=false)=>{
  stopTimer();video.pause();state.source=url;dialog.dataset.mediaUrl=url;error('');status(compatible?'Compatible preview · original clip kept unchanged.':'Loading clip…');
  video.src=url;video.load();update();showControls();if(state.autoplay)video.play().catch(e=>{if(e.name==='NotAllowedError'&&dialog.open&&!state.preparing)status('Ready — press Play to start.');});
  const generation=state.generation;
  state.timer=setTimeout(()=>{if(dialog.open&&generation===state.generation&&video.readyState<2&&video.currentTime===0&&!state.preparing){if(native&&!state.tried){report('Original playback timed out; preparing local preview');void compatible();return;}error('The player is taking too long to load. Retry playback or open the clip externally.');status('Playback has not started.');report('Loading timeout; readyState='+video.readyState);showControls();}},15000);
 };
 const compatible=async()=>{
  if(!dialog.open||state.preparing)return;const generation=state.generation;state.tried=true;state.preparing=true;stopTimer();video.pause();video.removeAttribute('src');video.load();error('');status('Preparing a compatible preview… The original clip stays unchanged.');update();showControls();
  try{await invoke('prepare_preview',{name:dialog.dataset.name});if(!dialog.open||generation!==state.generation)return;
   const data=await invoke('preview_bytes');if(!dialog.open||generation!==state.generation)return;
   const binary=data instanceof ArrayBuffer?data:new Uint8Array(data);if(binary.byteLength>32*1024*1024)throw Error('Preview exceeds the playback memory limit');
   releaseBlob();state.objectUrl=URL.createObjectURL(new Blob([binary],{type:'video/mp4'}));state.preparing=false;
   if(state.suspended){state.source=state.objectUrl;status('Preview ready — select Resume preview to watch.');update();}else load(state.objectUrl,true);
  }catch(e){if(!dialog.open||generation!==state.generation)return;state.preparing=false;error(String(e));status('Preview could not be prepared. You can still edit or open the original externally.');update();showControls();}
 };
 video.onerror=()=>{
  if(!dialog.open||state.preparing||!video.hasAttribute('src')||!video.error)return;stopTimer();
  const code=video.error?.code||0,detail=video.error?.message||'The media backend could not load the clip.';
  report(`source=${state.objectUrl&&video.src===state.objectUrl?'local-h264-preview':'original'}; code=${code}; ${detail}; readyState=${video.readyState}; networkState=${video.networkState}; h264=${video.canPlayType('video/mp4; codecs="avc1.42E01E, mp4a.40.2"')}`);
  update();if(native&&!state.tried&&[2,3,4].includes(code)){void compatible();return;}
  error(`Playback failed (${code}): ${detail}`);status('Retry playback or open the original externally.');showControls();
 };
 for(const name of ['loadeddata','loadedmetadata','canplay','timeupdate','play','pause','ended','volumechange','seeking','seeked','loadstart','durationchange'])video.addEventListener(name,()=>{if(!dialog.open)return;if(video.readyState>=2||video.currentTime>0)stopTimer();update();if(['play','pause','ended','loadstart'].includes(name))showControls();if(!state.preparing&&video.hasAttribute('src')){if(name==='canplay')status(video.paused?(state.tried?'Compatible preview ready · original clip unchanged.':'Ready to play.'):(state.tried?'Playing compatible preview.':'Playing clip.'));else if(name==='play')status(state.tried?'Playing compatible preview.':'Playing clip.');else if(name==='pause')status(state.tried?'Compatible preview paused.':'Clip paused.');else if(name==='ended')status('End of clip.');}});
 q('video-play').onclick=async()=>{const generation=state.generation;try{if(video.paused)await video.play();else video.pause();}catch(e){if(dialog.open&&generation===state.generation&&!(e?.name==='AbortError'&&video.paused))error('Could not start playback: '+String(e));}if(dialog.open&&generation===state.generation){update();showControls();}};
 q('video-back').onclick=()=>{video.currentTime=Math.max(0,video.currentTime-5);};
 q('video-forward').onclick=()=>{video.currentTime=Math.min(video.duration||0,video.currentTime+5);};
 q('video-seek').oninput=e=>{if(Number.isFinite(video.duration))video.currentTime=Math.min(video.duration,Math.max(0,Number(e.target.value)));showControls();};
 q('video-mute').onclick=()=>{video.muted=!video.muted;update();showControls();};
 q('video-volume').oninput=e=>{video.volume=Number(e.target.value);video.muted=video.volume===0;update();showControls();};
 q('video-compatible').onclick=compatible;q('video-retry').onclick=async()=>{try{const generation=state.generation;const source=state.source||await invoke('clip_media_url',{name:dialog.dataset.name});if(dialog.open&&generation===state.generation)load(source,state.tried);}catch(e){error(String(e));status('The clip could not be loaded.');}};
 q('video-external').onclick=async()=>{try{await invoke('open_clip_external',{name:dialog.dataset.name});}catch(e){error(String(e));}};
 stage.addEventListener('pointermove',showControls);stage.addEventListener('focusin',showControls);stage.addEventListener('pointerleave',showControls);
 stage.addEventListener('keydown',e=>{if(e.altKey||e.ctrlKey||e.metaKey||e.target.closest?.('input,button'))return;const actions={' ':()=>q('video-play').click(),ArrowLeft:()=>q('video-back').click(),ArrowRight:()=>q('video-forward').click(),m:()=>q('video-mute').click(),f:()=>q('video-expand').click()};if(actions[e.key]){e.preventDefault();actions[e.key]();showControls();}});
 dialog.addEventListener('close',()=>{state.generation++;stopTimer();clearTimeout(state.hideTimer);state.preparing=false;video.pause();video.removeAttribute('src');video.load();releaseBlob();expanded(false);if(native&&state.ownsFullscreen){state.ownsFullscreen=false;invoke('set_window_fullscreen',{enabled:false}).then(onWindowState).catch(()=>{});}});
 state.reset=()=>{releaseBlob();state.suspended=false;state.autoplay=false;state.generation++;stopTimer();state.tried=false;state.preparing=false;state.source='';error('');status('Loading clip details…');video.pause();video.removeAttribute('src');video.load();update();showControls();};
 state.load=(url,autoplay=false)=>{state.autoplay=autoplay;load(url);};state.pause=()=>{state.suspended=true;state.autoplay=false;stopTimer();video.pause();video.removeAttribute('src');video.load();q('resume-preview').hidden=false;status('Preview paused to save memory.');update();showControls();};
 q('resume-preview').onclick=()=>{if(state.source&&!state.preparing){state.suspended=false;load(state.source,state.tried);q('resume-preview').hidden=true;}};
}
export const resetPlayer=dialog=>players.get(dialog)?.reset();
export const loadPlayer=(dialog,url,autoplay=false)=>players.get(dialog)?.load(url,autoplay);
export const pausePlayer=dialog=>players.get(dialog)?.pause();
export const syncPlayerWindow=(dialog,snapshot)=>players.get(dialog)?.syncWindow(snapshot);
