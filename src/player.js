const players=new WeakMap();
export const formatTime=value=>{const s=Math.max(0,Math.floor(Number.isFinite(value)?value:0));return `${Math.floor(s/60)}:${String(s%60).padStart(2,'0')}`;};
export function playbackMarkup(){return `<div class="video-stage"><video controls playsinline preload="metadata" crossorigin="anonymous"></video></div><div class="player-controls" role="group" aria-label="Playback controls"><button id="video-back" class="button subtle" type="button" aria-label="Back 5 seconds" disabled>−5s</button><button id="video-play" class="button primary" type="button" aria-label="Play clip" disabled>Play</button><button id="video-forward" class="button subtle" type="button" aria-label="Forward 5 seconds" disabled>+5s</button><span id="video-time">0:00 / 0:00</span><input id="video-seek" type="range" min="0" max="1" step="0.05" value="0" aria-label="Seek clip" disabled><button id="video-mute" class="button subtle" type="button" aria-label="Mute clip">Mute</button><input id="video-volume" type="range" min="0" max="1" step="0.05" value="1" aria-label="Playback volume"></div><div class="player-status-row"><p id="playback-status" role="status">Select Play to watch the clip.</p><div class="player-options"><button id="video-retry" class="button subtle" type="button">Retry playback</button><button id="video-compatible" class="button subtle" type="button">Compatible preview</button><button id="video-external" class="button subtle" type="button">Open externally</button></div></div><p id="playback-error" role="alert" class="playback-error" hidden></p>`;}
export function bindPlayer(dialog,{invoke,native}){
 const video=dialog.querySelector('video'),q=id=>dialog.querySelector('#'+id);
 const state={autoplay:false,generation:0,source:'',tried:false,preparing:false,loading:false,timer:0};players.set(dialog,state);
 const error=message=>{q('playback-error').hidden=!message;q('playback-error').textContent=message;};
 const status=message=>q('playback-status').textContent=message;
 const update=()=>{
  const ready=video.hasAttribute('src')&&!video.error&&!state.preparing;
  for(const id of ['video-play','video-back','video-forward'])q(id).disabled=!ready||(id!=='video-play'&&(video.seeking||!Number.isFinite(video.duration)));q('video-seek').disabled=!video.currentSrc||!!video.error||state.preparing||!Number.isFinite(video.duration);
  q('video-compatible').disabled=state.preparing;q('video-retry').disabled=state.preparing;
  const play=q('video-play');play.textContent=video.paused?'Play':'Pause';play.setAttribute('aria-label',video.paused?'Play clip':'Pause clip');
  const duration=Number.isFinite(video.duration)?video.duration:0;
  q('video-time').textContent=`${formatTime(video.currentTime)} / ${formatTime(duration)}`;
  q('video-seek').max=String(duration||1);if(document.activeElement!==q('video-seek'))q('video-seek').value=String(video.currentTime||0);
  q('video-mute').textContent=video.muted?'Unmute':'Mute';q('video-mute').setAttribute('aria-label',video.muted?'Unmute clip':'Mute clip');
 };
 const report=detail=>{if(native)invoke('report_playback_error',{name:dialog.dataset.name,detail}).catch(()=>{});};
 const stopTimer=()=>{clearTimeout(state.timer);state.timer=0;};
 const load=(url,compatible=false)=>{
  stopTimer();video.pause();state.source=url;dialog.dataset.mediaUrl=url;state.loading=true;error('');status(compatible?'Compatible preview · original clip kept unchanged.':'Loading clip…');
  video.src=url;video.load();update();if(state.autoplay)video.play().catch(e=>{if(e.name==='NotAllowedError'&&dialog.open&&!state.preparing)status('Ready — press Play to start.');});
  const generation=state.generation;
  state.timer=setTimeout(()=>{if(dialog.open&&generation===state.generation&&video.readyState<2&&video.currentTime===0&&!state.preparing){error('The player is taking too long to load. Try a compatible preview or open the clip externally.');status('Playback has not started.');report('Loading timeout; readyState='+video.readyState);}},15000);
 };
 const compatible=async()=>{
  if(!dialog.open||state.preparing)return;const generation=state.generation;state.tried=true;state.preparing=true;stopTimer();video.pause();video.removeAttribute('src');video.load();error('');status('Preparing a compatible preview… The original clip stays unchanged.');update();
  try{const url=await invoke('prepare_preview',{name:dialog.dataset.name});if(!dialog.open||generation!==state.generation)return;state.preparing=false;load(url,true);}
  catch(e){if(!dialog.open||generation!==state.generation)return;state.preparing=false;error(String(e));status('Preview could not be prepared. You can still edit or open the original externally.');update();}
 };
 video.onerror=()=>{
  if(!dialog.open||state.preparing||!video.hasAttribute('src')||!video.error)return;stopTimer();
  const code=video.error?.code||0,detail=video.error?.message||'The media backend could not load the clip.';
  report(`code=${code}; ${detail}; readyState=${video.readyState}; networkState=${video.networkState}; h264=${video.canPlayType('video/mp4; codecs="avc1.42E01E, mp4a.40.2"')}; webm=${video.canPlayType('video/webm; codecs="vp8, opus"')}`);
  state.loading=false;update();
  if(native&&!state.tried&&[2,3,4].includes(code)){void compatible();return;}
  error(`Playback failed (${code}): ${detail}`);status('Try a compatible preview, retry playback, or open the original externally.');
 };
 for(const name of ['loadeddata','loadedmetadata','canplay','timeupdate','play','pause','ended','volumechange','seeking','seeked','loadstart','durationchange'])video.addEventListener(name,()=>{if(!dialog.open)return;if(video.readyState>=2||video.currentTime>0){stopTimer();state.loading=false;}update();if(name==='canplay'){status(state.tried?'Compatible preview ready · original clip unchanged.':'Ready to play.');}});
 q('video-play').onclick=async()=>{try{if(video.paused){await video.play();status(state.tried?'Playing compatible preview.':'Playing clip.');}else video.pause();}catch(e){error('Could not start playback: '+String(e));}update();};
 q('video-back').onclick=()=>{video.currentTime=Math.max(0,video.currentTime-5);};
 q('video-forward').onclick=()=>{video.currentTime=Math.min(video.duration||0,video.currentTime+5);};
 q('video-seek').oninput=e=>{if(Number.isFinite(video.duration))video.currentTime=Math.min(video.duration,Math.max(0,Number(e.target.value)));};
 q('video-mute').onclick=()=>{video.muted=!video.muted;update();};
 q('video-volume').oninput=e=>{video.volume=Number(e.target.value);video.muted=video.volume===0;update();};
 q('video-compatible').onclick=compatible;q('video-retry').onclick=async()=>{try{const source=state.source||await invoke('clip_media_url',{name:dialog.dataset.name});if(dialog.open)load(source,state.tried);}catch(e){error(String(e));status('The clip could not be loaded.');}};
 q('video-external').onclick=async()=>{try{await invoke('open_clip_external',{name:dialog.dataset.name});}catch(e){error(String(e));}};
 dialog.addEventListener('close',()=>{state.generation++;stopTimer();state.preparing=false;video.pause();video.removeAttribute('src');video.load();});
 state.reset=()=>{state.generation++;stopTimer();state.tried=false;state.preparing=false;state.source='';state.loading=false;error('');status('Loading clip details…');video.pause();video.removeAttribute('src');video.load();update();};
 state.load=(url,autoplay=false)=>{state.autoplay=autoplay;load(url);};state.pause=()=>{stopTimer();video.pause();video.removeAttribute('src');video.load();q('resume-preview').hidden=false;status('Preview paused to save memory.');update();};
 q('resume-preview').onclick=()=>{if(state.source&&!state.preparing){load(state.source,state.tried);q('resume-preview').hidden=true;}};
}
export const resetPlayer=dialog=>players.get(dialog)?.reset();
export const loadPlayer=(dialog,url,autoplay=false)=>players.get(dialog)?.load(url,autoplay);
export const pausePlayer=dialog=>players.get(dialog)?.pause();
