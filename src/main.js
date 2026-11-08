import { normalizeSettings } from './preferences.js';
import { listen } from '@tauri-apps/api/event';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { createIcons, Aperture, LayoutGrid, Circle, Settings2, ArrowUpRight, Video, Clock3, HardDrive, Monitor, Volume2, Mic, Zap, ChevronRight, Play, X, RefreshCw, Keyboard, Check, Cpu } from 'lucide';
import './style.css';

const native = isTauri();
let settings;
try { settings = JSON.parse(localStorage.getItem('afterglow-settings')); } catch {}
let monitorChosen = typeof settings?.monitor === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(settings.monitor);
settings = normalizeSettings(settings);
let appearance=localStorage.getItem('afterglow-appearance')||'caelestia';
if(!['caelestia','dark','light'].includes(appearance))appearance='caelestia';
let state = {running:false,gpu:'',recorderAvailable:false,wayland:false,outputDir:'~/Videos/Afterglow',log:''};
let captureOptions = [], clips = [], busy = false, filter = '', page = 'library';
const icon = (name, cls='') => `<i data-lucide="${name}" class="${cls}"></i>`;
const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const bytes = b => b >= 1e9 ? `${(b/1e9).toFixed(1)} GB` : `${(b/1e6).toFixed(1)} MB`;
function toast(message, error=false) {
  let el=document.querySelector('#toast');if(!el){el=document.createElement('div');el.id='toast';el.setAttribute('role','status');document.body.append(el);} el.textContent=message; el.className=`toast visible ${error?'error':''}`;
  clearTimeout(toast.timer); toast.timer=setTimeout(()=>el.classList.remove('visible'),5000);
}
function render() {
  const capture=state.running?(state.activeSettings||settings):settings;
  const shown=clips.filter(c=>c.name.toLowerCase().includes(filter.toLowerCase()));
  document.querySelector('#app').innerHTML=`
  <aside><a class="brand" href="#">${icon('aperture')}<span>afterglow<span class="brand-dot">.</span></span></a>
  <div class="workspace-label">YOUR WORKSPACE</div>
  <nav><button data-page="library" class="${page==='library'?'active':''}">${icon('layout-grid')} Clip library <span class="nav-count">${clips.length}</span></button><button data-page="setup" class="${page==='setup'?'active':''}">${icon('settings-2')} Capture setup</button></nav>
  <div class="sidebar-bottom"><div class="platform">${icon('cpu')}<div>NVIDIA edition<small>Linux · Wayland</small></div><span class="beta">α</span></div><div class="roadmap"><span class="green-dot"></span> NVIDIA first <span>AMD & Intel next</span></div></div></aside>
  <main><header><div class="breadcrumb">Workspace ${icon('chevron-right')} <span>${page==='library'?'Clip library':'Capture setup'}</span></div><div class="header-right"><span class="local-label">${icon('hard-drive')} Stored locally</span><span class="version">v0.1.3 alpha</span></div></header>
  <div class="content"><div class="page-heading"><div><div class="eyebrow">GOOD GAMES DESERVE A REPLAY</div><h1>${page==='library'?'Keep the moment.':'Make it yours.'}</h1><p>${page==='library'?'Your best plays, a shortcut away.':'A simple setup for your next great play.'}</p></div><div class="heading-actions"><button id="open-library" class="button subtle" ${!native?'disabled':''}> ${icon('hard-drive')} Open folder</button><button id="refresh" class="button subtle">${icon('refresh-cw')} Refresh</button></div></div>
  ${!native?`<div class="preview-notice">${icon('monitor')} <span><strong>Interface preview.</strong> Launch the desktop app on CachyOS to capture gameplay. Recording controls are disabled here.</span></div>`:''}
  <section class="replay-card"><div class="replay-art"><div class="orbit orbit-one"></div><div class="orbit orbit-two"></div><div class="orbit orbit-three"></div><div class="replay-symbol">${icon('aperture')}</div><span class="art-label">ALWAYS IN THE MOMENT</span></div><div class="replay-copy"><div class="status-tag ${state.running?'live':''}"><span class="green-dot"></span>${state.running?'REPLAY IS ACTIVE':'READY WHEN YOU ARE'}</div><h2>${state.running?'The next great play is covered.':'You play. We’ll keep the highlight.'}</h2><p>Keep your last ${capture.seconds} seconds on standby.<br>Save the moment without leaving your game.</p><div class="replay-actions"><button class="button primary" id="toggle" ${!native||busy?'disabled':''}>${icon(state.running?'circle':'zap')}${state.running?'Stop replay':'Start replay'}</button><button class="button secondary" id="save" ${!state.running||busy?'disabled':''}>${icon('video')} Save clip <kbd>Super F8</kbd></button></div><button class="button subtle game-mode" id="game-mode" ${!native||!state.running||busy?'disabled':''}>${icon('monitor')} Game mode</button><small class="capture-note">${icon('monitor')} Display capture · ${capture.fps} FPS · H.264 · ${capture.bitrate||12} Mbps</small></div></section>
  <div class="stats"><div>${icon('video')}<span>Clips saved<strong>${clips.length.toString().padStart(2,'0')}</strong></span></div><div>${icon('clock-3')}<span>Replay length<strong>${capture.seconds}<small> seconds</small></strong></span></div><div>${icon('hard-drive')}<span>App + capture RAM<strong id="ram-value">${state.memory?.bytes?bytes(state.memory.bytes):'—'}</strong><small class="ram-target">Target &lt; 1 GB</small></span></div><div>${icon('cpu')}<span>Capture backend<strong class="gpu-name">${esc(native?(state.gpu||'NVIDIA unavailable'):'NVIDIA / NVENC')}</strong></span></div></div>
  ${page==='library'?`<section class="library"><div class="section-heading"><div><h2>Your clips <span>${clips.length}</span></h2><p>The ones worth watching again.</p></div><input aria-label="Search clips" id="search" type="search" placeholder="Search clips…" value="${esc(filter)}"></div><div class="clip-grid">${shown.length?shown.map(c=>`<button class="clip-card" data-path="${esc(c.path)}"><div class="clip-cover">${icon('play')}<span>H.264 / MP4</span></div><div class="clip-info"><strong>${esc(c.name)}</strong><span>${esc(new Date(c.modified*1000).toLocaleString())} · ${bytes(c.bytes)}</span></div></button>`).join(''):`<div class="empty-state"><div class="empty-icon">${icon('video')}</div><h3>${filter?'No matching clips':'Your first highlight starts here'}</h3><p>${filter?'Try searching for a different filename.':'Start replay, jump into a game, and save a moment.<br>Your clips will land right here.'}</p>${filter?'':`<div class="shortcut">${icon('keyboard')}<kbd>Super</kbd><span>+</span><kbd>F8</kbd><span>to keep the last ${capture.seconds}s</span></div>`}</div>`}</div></section>`:setupPanel()}
  <footer><span>${icon('hard-drive')} ${esc(state.outputDir)} · ${bytes(clips.reduce((n,c)=>n+c.bytes,0))}</span><span>Made for the way you play. <span class="green-dot"></span></span></footer></div></main><dialog id="player"><button id="close-player" class="button subtle">${icon('x')} Close</button><video controls preload="none"></video><p id="video-name"></p><div class="clip-tools"><form id="rename-form"><label>Clip name<input id="rename-value" required maxlength="160" placeholder="Name your highlight"></label><button type="submit" class="button secondary">Rename clip</button></form><form id="trim-form"><div class="trim-range"><label>Start (seconds)<input id="trim-start" type="number" value="0" min="0" step="0.01" required></label><label>End (seconds)<input id="trim-end" type="number" min="0.1" step="0.01" required></label></div><button type="submit" class="button primary">Export trimmed copy</button></form><p id="trim-status" role="status">The original clip is kept. Export uses FFmpeg on the CPU.</p></div></dialog>`;
  createIcons({icons:{Aperture,LayoutGrid,Circle,Settings2,ArrowUpRight,Video,Clock3,HardDrive,Monitor,Volume2,Mic,Zap,ChevronRight,Play,X,RefreshCw,Keyboard,Check,Cpu}});
  bind();applyTheme();updateMemory();
}
function setupPanel() {
 return `<section class="setup"><div class="section-heading"><div><h2>Capture preferences</h2><p>Changes apply the next time you start replay.</p></div></div><form id="settings"><label>Replay length<select name="seconds">${[30,60,90,120].map(n=>`<option value="${n}" ${settings.seconds===n?'selected':''}>${n} seconds</option>`).join('')}</select></label><label>Frame rate<select name="fps">${[30,60].map(n=>`<option value="${n}" ${settings.fps===n?'selected':''}>${n} FPS</option>`).join('')}</select></label><label>Video bitrate<select name="bitrate">${[8,12,16,20].map(n=>`<option value="${n}" ${settings.bitrate===n?'selected':''}>${n} Mbps</option>`).join('')}</select><small id="buffer-estimate"></small></label><label>Appearance<select name="appearance">${[['caelestia','Follow Caelestia'],['dark','Afterglow dark'],['light','Afterglow light']].map(([value,label])=>`<option value="${value}" ${appearance===value?'selected':''}>${label}</option>`).join('')}</select><small id="theme-status">${esc(themeLabel())}</small></label><label>Monitor name<input name="monitor" list="monitor-options" required pattern="[a-zA-Z0-9_-]+" maxlength="80" value="${esc(settings.monitor)}"><datalist id="monitor-options">${captureOptions.map(n=>`<option value="${esc(n)}"></option>`).join('')}</datalist><small>Use the name from <code>hyprctl monitors</code>, e.g. DP-1.</small></label><div class="audio-options"><label>${icon('volume-2')} System audio<input type="checkbox" name="systemAudio" ${settings.systemAudio?'checked':''}></label><label>${icon('mic')} Microphone<input type="checkbox" name="microphone" ${settings.microphone?'checked':''}></label></div><div class="settings-footer"><span>H.264 · Native display resolution</span><button class="button primary" type="submit">Save preferences</button></div></form><div class="diagnostics"><h3>Desktop checks</h3><p>${native?(state.gpu?'✓ NVIDIA detected':'○ NVIDIA not detected'):'○ Desktop app required'}</p><p>${state.recorderAvailable?'✓':'○'} GPU Screen Recorder ${state.recorderAvailable?'installed':'not detected'}</p><p>${state.wayland?'✓':'○'} Wayland session</p><p>${state.ffmpegAvailable?'✓':'○'} FFmpeg for trimming</p><p>Default buffer: about 90 MB of encoded video (60s at 12 Mbps), plus audio and capture overhead.</p><p>Game mode closes the library while replay keeps running. Reopen from the tray or launch Afterglow again. The library switches to game mode above 850 MB measured RAM while recording.</p><details><summary>Capture log</summary><pre>${esc(state.log||'No capture log yet.')}</pre></details></div><div class="hotkey-help"><h3>Set your Hyprland shortcut</h3><p>Add this to your Hyprland user configuration:</p><pre>bind = SUPER, F8, exec, afterglow --save-replay</pre><p>The shortcut works while the app and replay buffer are running. See README for Caelestia setup details.</p></div></section>`;
}
function bind() {
 document.querySelectorAll('[data-page]').forEach(b=>b.onclick=()=>{page=b.dataset.page;render();});
 document.querySelector('#game-mode').onclick=async()=>{try{await invoke('game_mode');}catch(e){toast(String(e),true);}};
 document.querySelector('#refresh').onclick=()=>refresh(true);
 document.querySelector('#open-library').onclick=async()=>{try{await invoke('open_library');}catch(e){toast(String(e),true);}};
 document.querySelector('#toggle').onclick=async()=> {
   busy=true;render(); let message='', error=false;
   try { await invoke(state.running?'stop_replay':'start_replay', state.running?{}:{settings}); await refresh(); message=state.running?'Replay started. Watch for capture errors in setup.':'Replay is stopping.'; } catch(e){message=String(e);error=true;} finally {busy=false;render();if(message)toast(message,error);}
 };
 document.querySelector('#save').onclick=async()=> {try {await invoke('save_replay');toast('Save requested. The clip appears when writing finishes.');}catch(e){toast(String(e),true);}};
 document.querySelector('#search')?.addEventListener('input',e=>{const pos=e.target.selectionStart;filter=e.target.value;render();const input=document.querySelector('#search');input.focus();try{input.setSelectionRange(pos,pos);}catch{}});
 document.querySelector('#settings')?.addEventListener('submit',e=>{e.preventDefault();const f=new FormData(e.target);monitorChosen=true;settings=normalizeSettings({bitrate:Number(f.get('bitrate')),seconds:Number(f.get('seconds')),fps:Number(f.get('fps')),monitor:f.get('monitor').trim(),systemAudio:f.has('systemAudio'),microphone:f.has('microphone')});localStorage.setItem('afterglow-settings',JSON.stringify(settings));render();toast('Preferences saved. Restart replay to apply changes.');});
 const form=document.querySelector('#settings');
 if(form){const budget=()=>{const seconds=Number(form.elements.seconds.value),select=form.elements.bitrate;for(const option of select.options)option.disabled=seconds*Number(option.value)>1440;if(select.selectedOptions[0].disabled)select.value='12';document.querySelector('#buffer-estimate').textContent=`About ${seconds*Number(select.value)/8} MB of encoded video; maximum 180 MB.`;};form.elements.seconds.onchange=budget;form.elements.bitrate.onchange=budget;form.elements.appearance.onchange=e=>{appearance=e.target.value;localStorage.setItem('afterglow-appearance',appearance);applyTheme();};budget();}
 document.querySelectorAll('[data-path]').forEach(b=>b.onclick=()=>openPlayer(b.dataset.path));
 const dialog=document.querySelector('#player'); document.querySelector('#close-player').onclick=()=>dialog.close();
 dialog.addEventListener('close',()=>{dialog.querySelector('video').pause();dialog.querySelector('video').removeAttribute('src');dialog.querySelector('video').load();if(!busy)render();});
 document.querySelector('#rename-form').onsubmit=async e=> {
   e.preventDefault();const name=dialog.dataset.name, newName=document.querySelector('#rename-value').value.trim().replace(/\.mp4$/i,'')+'.mp4';
   try {await invoke('rename_clip',{name,newName});dialog.close();await refresh(true);toast('Clip renamed.');}catch(error){toast(String(error),true);}
 };
 document.querySelector('#trim-form').onsubmit=async e=> {
   e.preventDefault(); const start=Number(document.querySelector('#trim-start').value),end=Number(document.querySelector('#trim-end').value);
   if(!Number.isFinite(start)||!Number.isFinite(end)||start<0||end-start<0.1){toast('Choose a valid range of at least 0.1 seconds.',true);return;}
   busy=true;dialog.querySelectorAll('.clip-tools button').forEach(b=>b.disabled=true);document.querySelector('#trim-status').textContent='Exporting trimmed copy… Replay capture can continue.';
   let message,failed=false;
   try{const name=await invoke('trim_clip',{name:dialog.dataset.name,start,end});message='Saved '+name;}catch(error){message=String(error);failed=true;}
   finally{busy=false;dialog.close();await refresh(true);toast(message,failed);}
 };
}
async function openPlayer(path) {
 const d=document.querySelector('#player'),v=d.querySelector('video'),name=path.split('/').pop();
 d.dataset.name=name; document.querySelector('#video-name').textContent=name;document.querySelector('#rename-value').value=name.replace(/\.mp4$/i,'');
 document.querySelector('#trim-end').value='';document.querySelector('#trim-start').value='0';document.querySelector('#trim-status').textContent='The original clip is kept. Export uses FFmpeg on the CPU.';
 d.querySelectorAll('.clip-tools button').forEach(b=>b.disabled=false);document.querySelector('#trim-form button').disabled=true;
 v.onerror=()=>toast('Cannot play this clip in WebKit. Use Open folder to play it externally.',true);d.showModal();
 try{const url=await invoke('clip_media_url',{name});if(d.open&&d.dataset.name===name){v.src=url;v.load();}const duration=await invoke('clip_duration',{name});if(d.open&&d.dataset.name===name){document.querySelector('#trim-end').value=(Math.floor(duration*100)/100).toFixed(2);document.querySelector('#trim-form button').disabled=false;}}
 catch(error){toast(String(error),true);}

}
function themeLabel(){return appearance==='caelestia'?(state.theme?.available?`Synced with ${state.theme.name}${state.theme.error?' (waiting for scheme update)':''}`:'Caelestia scheme unavailable; using Afterglow dark.'):'Built-in appearance';}
function applyTheme(){
 const light=appearance==='light'||(appearance==='caelestia'&&state.theme?.available&&state.theme.light);
 const base=light?{background:'#f7faf0',surface:'#eaf0df','surface-low':'#f1f5e9','surface-high':'#e1e8d4',foreground:'#1a2115',muted:'#46513d',border:'#c5ceba',accent:'#486d24','on-accent':'#ffffff','accent-container':'#c8edaa','on-accent-container':'#284015'}:{background:'#101211',surface:'#20281c','surface-low':'#141715','surface-high':'#2b3427',foreground:'#e7eae6',muted:'#a2af98',border:'#3a4532',accent:'#c1ec91','on-accent':'#203016','accent-container':'#283322','on-accent-container':'#d3edb5'};
 if(appearance==='caelestia'&&state.theme?.available){const map={surface:'background',surfaceContainer:'surface',surfaceContainerLow:'surface-low',surfaceContainerHigh:'surface-high',onSurface:'foreground',onSurfaceVariant:'muted',outlineVariant:'border',primary:'accent',onPrimary:'on-accent',primaryContainer:'accent-container',onPrimaryContainer:'on-accent-container'};for(const [source,target]of Object.entries(map)){const value=state.theme.colours?.[source];if(/^#[0-9a-f]{6}$/i.test(value||''))base[target]=value;}}
 for(const[key,value]of Object.entries(base))document.documentElement.style.setProperty('--'+key,value);
 document.documentElement.style.colorScheme=light?'light':'dark';
 const status=document.querySelector('#theme-status');if(status)status.textContent=themeLabel();
}
function updateMemory(){const el=document.querySelector('#ram-value');if(el){el.textContent=state.memory?.bytes?`${state.memory.approximate?'~ ':''}${bytes(state.memory.bytes)}`:'—';el.classList.toggle('ram-warning',(state.memory?.bytes||0)>=750000000);el.title='Estimated memory across Afterglow, playback processes, and its recorder. Shared memory is apportioned when available.';}}
function updateCaptureStatus(){
 const capture=state.running?(state.activeSettings||settings):settings;
 const tag=document.querySelector('.status-tag');if(!tag)return;
 tag.classList.toggle('live',state.running);tag.innerHTML=`<span class="green-dot"></span>${state.running?'REPLAY IS ACTIVE':'READY WHEN YOU ARE'}`;
 document.querySelector('.replay-copy h2').textContent=state.running?'The next great play is covered.':'You play. We’ll keep the highlight.';
 document.querySelector('.replay-copy p').innerHTML=`Keep your last ${Number(capture.seconds)} seconds on standby.<br>Save the moment without leaving your game.`;
 const toggle=document.querySelector('#toggle');toggle.lastChild.nodeValue=state.running?'Stop replay':'Start replay';toggle.disabled=!native||busy;
 document.querySelector('#save').disabled=!state.running||busy;document.querySelector('#game-mode').disabled=!native||!state.running||busy;
 document.querySelector('.capture-note').lastChild.nodeValue=`Display capture · ${capture.fps} FPS · H.264 · ${capture.bitrate||12} Mbps`;
 document.querySelector('.nav-count').textContent=clips.length;
 document.querySelector('.stats>div:first-child strong').textContent=clips.length.toString().padStart(2,'0');
 document.querySelector('.stats>div:nth-child(2) strong').firstChild.nodeValue=String(capture.seconds);
 const log=document.querySelector('.diagnostics pre');if(log)log.textContent=state.log||'No capture log yet.';
}
async function refresh(manual=false) {
 if (!native) {if(manual)toast('Preview mode. Use npm run tauri dev on your Linux desktop.');return;}
 try {const [next,items]=await Promise.all([invoke('status',{force:manual}),invoke('list_clips')]); const changed=JSON.stringify({...next,memory:null,theme:null})!==JSON.stringify({...state,memory:null,theme:null})||JSON.stringify(items)!==JSON.stringify(clips); const failed=state.running&&!next.running&&!next.stoppedIntentionally;state=next;clips=items;applyTheme();updateMemory();updateCaptureStatus();if(changed&&!document.querySelector('#player')?.open&&page!=='setup')render();if(manual&&!document.querySelector('#player')?.open)render();if(failed)toast('Capture stopped. Check Capture setup → Capture log.',true);} catch(e){if(manual)toast(String(e),true);}
}
render();refresh();if(native)listen('clip-saved',async()=>{await refresh();toast('Clip saved to your library.');}).catch(()=>{});if(native)invoke('capture_options').then(options=>{captureOptions=options;if(!monitorChosen&&options.length){settings.monitor=options.find(n=>n!=='screen')||options[0];const input=document.querySelector('[name=monitor]');if(input&&input.value==='screen')input.value=settings.monitor;}const list=document.querySelector('#monitor-options');if(list)list.innerHTML=options.map(n=>`<option value="${esc(n)}"></option>`).join('');}).catch(()=>{});setInterval(()=>{if(!busy)refresh();},2500);
