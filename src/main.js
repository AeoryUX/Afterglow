import { renderSettings, defaultDesktopPreferences, shortcutLabel, shortcutFromEvent } from './settings-view.js';
import { normalizeSettings } from './preferences.js';
import { listen } from '@tauri-apps/api/event';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { createIcons, Aperture, LayoutGrid, Circle, Settings2, ArrowUpRight, Video, Clock3, HardDrive, Monitor, Volume2, Mic, Zap, ChevronRight, Play, X, RefreshCw, Keyboard, Check, Cpu, House } from 'lucide';
import './style.css';

const native = isTauri();
let settings;
try { settings = JSON.parse(localStorage.getItem('afterglow-settings')); } catch {}
let monitorChosen = typeof settings?.monitor === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(settings.monitor);
settings = normalizeSettings(settings);
let appearance=localStorage.getItem('afterglow-appearance')||'caelestia';
if(!['caelestia','dark','light'].includes(appearance))appearance='caelestia';
let state = {running:false,gpu:'',recorderAvailable:false,wayland:false,outputDir:'~/Videos/Afterglow',log:'',starting:false};
let prefs=defaultDesktopPreferences(), settingsTab='capture', shortcutDraft=structuredClone(prefs.shortcuts), shortcutRecording=null, preferencesLoaded=false;
let captureOptions = [], clips = [], busy = false, filter = '', page = 'home', clipSort = 'newest';
const icon = (name, cls='') => `<i data-lucide="${name}" class="${cls}"></i>`;
const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const bytes = b => b >= 1e9 ? `${(b/1e9).toFixed(1)} GB` : `${(b/1e6).toFixed(1)} MB`;
function toast(message, error=false) {
  let el=document.querySelector('#toast');if(!el){el=document.createElement('div');el.id='toast';el.setAttribute('role','status');document.body.append(el);} el.textContent=message; el.className=`toast visible ${error?'error':''}`;
  clearTimeout(toast.timer); toast.timer=setTimeout(()=>el.classList.remove('visible'),5000);
}
function clipCard(c){return `<button class="clip-card" data-path="${esc(c.path)}" aria-label="Edit ${esc(c.name)}"><div class="clip-cover">${icon('play')}<span>MP4</span></div><div class="clip-info"><strong>${esc(c.name)}</strong><span>${esc(new Date(c.modified*1000).toLocaleString())} · ${bytes(c.bytes)}</span></div></button>`;}
function libraryPanel(recent=false){
 const shown=clips.filter(c=>recent||c.name.toLowerCase().includes(filter.toLowerCase())).slice().sort((a,b)=>clipSort==='name'&&!recent?a.name.localeCompare(b.name):clipSort==='oldest'&&!recent?a.modified-b.modified:b.modified-a.modified);
 return `<section class="library ${recent?'recent-clips':''}"><div class="section-heading"><div><h2>${recent?'Recent clips':'All clips'} <span>${clips.length}</span></h2><p>${recent?'Your latest saved moments.':'Select a clip to preview, rename, or trim it.'}</p></div>${recent?`<button class="button subtle" data-page="library">View library ${icon('arrow-up-right')}</button>`:`<div class="library-tools"><input aria-label="Search clips" id="search" type="search" placeholder="Search clips" value="${esc(filter)}"><select id="clip-sort" aria-label="Sort clips">${[['newest','Newest first'],['oldest','Oldest first'],['name','Name']].map(([value,label])=>`<option value="${value}" ${clipSort===value?'selected':''}>${label}</option>`).join('')}</select></div>`}</div><div class="clip-grid">${shown.length?(recent?shown.slice(0,3):shown).map(clipCard).join(''):`<div class="empty-state">${icon('video')}<h3>${filter&&!recent?'No matching clips':'No clips yet'}</h3><p>${filter&&!recent?'Try another filename.':'Save a replay from Home or with your shortcut. It will appear here.'}</p></div>`}</div></section>`;
}
function replayPanel(capture){return `<section class="replay-card"><div class="replay-copy"><div class="replay-topline"><div class="status-tag ${state.running?'live':''}"><span class="green-dot"></span>${state.starting?'STARTING REPLAY':state.captureError?'CAPTURE NEEDS ATTENTION':state.running?'REPLAY IS ACTIVE':'REPLAY IS OFF'}</div><span class="replay-duration">${capture.seconds}s</span></div><h2>Replay buffer</h2><p>Keep the last ${capture.seconds} seconds. Save a clip whenever you need it.</p><div class="replay-actions"><button class="button primary" id="toggle" ${!native||busy||state.starting?'disabled':''}>${icon(state.running?'circle':'zap')}<span>${state.starting?'Starting…':state.running?'Stop replay':'Start replay'}</span></button><button class="button secondary" id="save" ${!state.running||busy||state.storage?.canSave===false?'disabled':''}>${icon('video')} Save clip <kbd>${esc(shortcutLabel(prefs.shortcuts.save))}</kbd></button><button class="button subtle" id="game-mode" ${!native||!state.running||busy?'disabled':''}>${icon('monitor')} Game mode</button></div><small class="capture-note">${icon('monitor')}<span>${capture.fps} FPS · H.264 · ${capture.bitrate||12} Mbps</span></small><div id="capture-error" class="capture-error" role="alert" ${state.captureError?'':'hidden'}><span>${esc(state.captureError||'')}</span><button type="button" id="capture-diagnostics">Open diagnostics</button></div><div id="storage-warning" class="storage-warning" ${state.storage?.canSave===false?'':'hidden'}>The next clip needs more storage room. <button type="button" id="manage-storage">Manage capacity</button></div></div></section>`;}
function render() {
 const capture=state.running?(state.activeSettings||settings):settings;
 const section=page==='setup'?(settingsTab==='keybinds'?'keybinds':'settings'):page;
 const title={home:'Home',library:'Clip library',keybinds:'Keybinds',settings:'Settings'}[section];
 const nav=(id,glyph,label,extra='')=>`<button ${id==='keybinds'?'id="keybinds-nav"':`data-page="${id==='settings'?'setup':id}"`} class="${section===id?'active':''}" ${section===id?'aria-current="page"':''} title="${label}">${icon(glyph)}<span class="nav-label">${label}</span>${extra}</button>`;
 document.querySelector('#app').innerHTML=`<aside><a class="brand" href="#" aria-label="Afterglow home">${icon('aperture')}<span>afterglow<span class="brand-dot">.</span></span></a><div class="workspace-label">WORKSPACE</div><nav aria-label="Main navigation">${nav('home','house','Home')}${nav('library','layout-grid','Clip library',`<span class="nav-count">${clips.length}</span>`)}${nav('keybinds','keyboard','Keybinds')}${nav('settings','settings-2','Settings')}</nav><div class="sidebar-bottom"><button id="storage-capacity" class="capacity-button" aria-label="Set clip-library storage capacity">${icon('hard-drive')}<span><strong>Clip storage</strong><small id="capacity-label">${bytes(state.storage?.usedBytes||clips.reduce((n,c)=>n+c.bytes,0))} / ${prefs.storageLimitMb?bytes(prefs.storageLimitMb*1e6):'Unlimited'}</small><span class="capacity-track"><span id="capacity-meter" style="width:${prefs.storageLimitMb?Math.min(100,(state.storage?.usedBytes||0)/(prefs.storageLimitMb*1e6)*100):0}%"></span></span></span></button><div class="desktop-label">${icon('monitor')}<span>Linux · ${state.session==='x11'?'X11':'Wayland'}</span></div></div></aside><main><header><div class="breadcrumb">Afterglow ${icon('chevron-right')} <span>${title}</span></div><span class="version">0.1.7 <span>alpha</span></span></header><div class="content"><div class="page-heading"><div><h1>${title}</h1><p>${section==='home'?'Replay, storage, and your recent clips.':section==='library'?'Your saved clips and editing tools.':section==='keybinds'?'Your actions, your shortcuts.':'Capture and desktop preferences.'}</p></div><div class="heading-actions"><button id="open-library" class="button subtle" ${!native?'disabled':''}>${icon('hard-drive')} Open folder</button><button id="refresh" class="button icon-button subtle" aria-label="Refresh" title="Refresh">${icon('refresh-cw')}</button></div></div>${!native?`<div class="preview-notice">${icon('monitor')}<span>Interface preview · Recording is available in the desktop app.</span></div>`:''}${page==='home'?`${replayPanel(capture)}<div class="stats"><div>${icon('video')}<span>Saved clips<strong>${clips.length.toString().padStart(2,'0')}</strong></span></div><div>${icon('clock-3')}<span>Replay length<strong>${capture.seconds}<small> seconds</small></strong></span></div><div>${icon('hard-drive')}<span>App + capture RAM<strong id="ram-value">${state.memory?.bytes?bytes(state.memory.bytes):'—'}</strong><small class="ram-target">Target &lt; 1 GB</small></span></div></div>${libraryPanel(true)}`:page==='library'?libraryPanel():setupPanel()}<footer><span>${icon('hard-drive')} ${esc(state.outputDir)}</span><span>Clips stay on your device</span></footer></div></main><dialog id="player"><button id="close-player" class="button subtle">${icon('x')} Close</button><button id="resume-preview" class="button subtle" hidden>Resume preview</button><video controls preload="none"></video><p id="video-name"></p><div class="clip-tools"><form id="rename-form"><label>Clip name<input id="rename-value" required maxlength="160" placeholder="Name your highlight"></label><button type="submit" class="button secondary">Rename clip</button></form><form id="trim-form"><div class="trim-range"><label>Start (seconds)<input id="trim-start" type="number" value="0" min="0" step="0.01" required></label><label>End (seconds)<input id="trim-end" type="number" min="0.1" step="0.01" required></label></div><label class="trim-mode-label">Trim mode<select id="trim-mode"><option value="fast">Fast · keyframe cut</option><option value="precise">Frame-accurate · CPU</option></select></label><button type="submit" class="button primary">Export trimmed copy</button></form><p id="trim-status" role="status">Fast trim keeps the original quality and may include time near a keyframe. Choose frame-accurate for an exact cut.</p></div></dialog>`;
 createIcons({icons:{Aperture,LayoutGrid,Circle,Settings2,ArrowUpRight,Video,Clock3,HardDrive,Monitor,Volume2,Mic,Zap,ChevronRight,Play,X,RefreshCw,Keyboard,Check,Cpu,House}});
 bind();bindSettings();applyTheme();updateMemory();
}
function setupPanel() {
 return renderSettings({tab:settingsTab,settings,appearance,state,options:captureOptions,prefs,shortcutDraft,recording:shortcutRecording,icon,esc,bytes,native,themeLabel});
}
async function persistPreferences(next) {
 if(native)prefs=await invoke('save_preferences',{prefs:next});else{prefs=next;localStorage.setItem('afterglow-desktop-preferences',JSON.stringify(prefs));}
 return prefs;
}
function openSettings(tab){page='setup';settingsTab=tab;shortcutRecording=null;render();}
function bindSettings(){
 document.querySelector('#storage-capacity').onclick=()=>openSettings('storage');
 document.querySelector('#keybinds-nav').onclick=()=>openSettings('keybinds');
 const diagnostic=document.querySelector('#capture-diagnostics');if(diagnostic)diagnostic.onclick=()=>openSettings('diagnostics');
 const manage=document.querySelector('#manage-storage');if(manage)manage.onclick=()=>openSettings('storage');
 document.querySelectorAll('[data-settings-tab]').forEach(button=>{
  button.onclick=()=>{settingsTab=button.dataset.settingsTab;shortcutRecording=null;render();document.querySelector(`#tab-${settingsTab}`)?.focus();};
  button.onkeydown=e=>{const tabs=[...document.querySelectorAll('[data-settings-tab]')];let index=tabs.indexOf(button);if(['ArrowRight','ArrowLeft','Home','End'].includes(e.key)){e.preventDefault();index=e.key==='Home'?0:e.key==='End'?tabs.length-1:(index+(e.key==='ArrowRight'?1:-1)+tabs.length)%tabs.length;tabs[index].click();}};
 });
 document.querySelectorAll('[data-record-shortcut]').forEach(button=>button.onclick=()=>{shortcutRecording=button.dataset.recordShortcut;render();document.querySelector(`[data-record-shortcut=${shortcutRecording}]`)?.focus();});
 document.querySelectorAll('[data-shortcut-modifier],[data-shortcut-key]').forEach(input=>{input.onfocus=()=>{shortcutRecording=null;};input.onchange=()=>{const id=input.dataset.shortcutModifier||input.dataset.shortcutKey;shortcutDraft[id]={modifiers:document.querySelector(`[data-shortcut-modifier=${id}]`).value.split(' ').filter(Boolean),key:document.querySelector(`[data-shortcut-key=${id}]`).value};shortcutRecording=null;render();};});
 document.querySelectorAll('.copy-command').forEach(input=>input.onclick=()=>input.select());
 const apply=document.querySelector('#apply-shortcuts');if(apply)apply.onclick=async()=>{apply.disabled=true;try{prefs=await invoke('apply_shortcuts',{shortcuts:{...shortcutDraft,enabled:true}});shortcutDraft=structuredClone(prefs.shortcuts);state.shortcutStatus='Shortcuts active in Hyprland, including from the tray.';render();toast('Shortcuts applied.');}catch(e){toast(String(e),true);}finally{if(apply.isConnected)apply.disabled=false;}};
 const remove=document.querySelector('#remove-shortcuts');if(remove)remove.onclick=async()=>{try{prefs=await invoke('apply_shortcuts',{shortcuts:{...prefs.shortcuts,enabled:false}});shortcutDraft=structuredClone(prefs.shortcuts);state.shortcutStatus='Afterglow shortcuts removed.';render();toast('Shortcuts removed.');}catch(e){toast(String(e),true);}};
 const storage=document.querySelector('#storage-settings');if(storage){storage.elements.unlimited.onchange=()=>{storage.elements.capacityGb.disabled=storage.elements.unlimited.checked;};document.querySelectorAll('[data-capacity]').forEach(button=>button.onclick=()=>{const mb=Number(button.dataset.capacity);storage.elements.unlimited.checked=mb===0;storage.elements.capacityGb.disabled=mb===0;if(mb)storage.elements.capacityGb.value=mb/1000;document.querySelectorAll('[data-capacity]').forEach(b=>b.classList.toggle('selected',b===button));});storage.onsubmit=async e=>{e.preventDefault();const mb=storage.elements.unlimited.checked?0:Math.round(Number(storage.elements.capacityGb.value)*1000);try{await persistPreferences({...prefs,storageLimitMb:mb});render();await refresh();toast('Storage capacity saved. Existing clips are kept.');}catch(error){toast(String(error),true);}};}
 const behavior=document.querySelector('#behavior-settings');if(behavior){behavior.elements.appearance.onchange=e=>{appearance=e.target.value;localStorage.setItem('afterglow-appearance',appearance);applyTheme();};behavior.onsubmit=async e=>{e.preventDefault();try{await persistPreferences({...prefs,closeToTray:behavior.elements.closeToTray.checked,autoStartReplay:behavior.elements.autoStartReplay.checked,startInTray:behavior.elements.startInTray.checked,clipSound:behavior.elements.clipSound.checked});render();toast('Behavior preferences saved.');}catch(error){toast(String(error),true);}};}
}
document.addEventListener('keydown',e=>{if(!shortcutRecording)return;e.preventDefault();e.stopPropagation();if(e.key==='Escape'){shortcutRecording=null;render();return;}const shortcut=shortcutFromEvent(e);if(shortcut){shortcutDraft[shortcutRecording]=shortcut;shortcutRecording=null;render();}},true);
function onClick(id,handler){document.getElementById(id)?.addEventListener('click',handler);}
function bind() {
 document.querySelector('.brand').onclick=e=>{e.preventDefault();page='home';shortcutRecording=null;render();};
 document.querySelectorAll('[data-page]').forEach(b=>b.onclick=()=>{shortcutRecording=null;page=b.dataset.page;if(page==='setup'&&settingsTab==='keybinds')settingsTab='capture';render();});
 onClick('game-mode',async()=>{try{await invoke('game_mode');}catch(e){toast(String(e),true);}});
 onClick('refresh',()=>refresh(true));
 onClick('open-library',async()=>{try{await invoke('open_library');}catch(e){toast(String(e),true);}});
 onClick('toggle',async()=> {
   busy=true;render();let message='',error=false;
   try{await invoke(state.running?'stop_replay':'start_replay',state.running?{}:{settings});await refresh();message=state.running?'Replay started.':'Replay stopped.';}catch(e){message=String(e);error=true;}finally{busy=false;render();if(message)toast(message,error);}
 });
 onClick('save',async()=>{try{await invoke('save_replay');toast('Saving clip…');}catch(e){toast(String(e),true);}});
 document.querySelector('#clip-sort')?.addEventListener('change',e=>{clipSort=e.target.value;render();});
 document.querySelector('#search')?.addEventListener('input',e=>{const pos=e.target.selectionStart;filter=e.target.value;render();const input=document.querySelector('#search');input.focus();try{input.setSelectionRange(pos,pos);}catch{}});
 document.querySelector('#settings')?.addEventListener('submit',async e=>{e.preventDefault();const f=new FormData(e.target);const next=normalizeSettings({bitrate:Number(f.get('bitrate')),seconds:Number(f.get('seconds')),fps:Number(f.get('fps')),monitor:f.get('monitor').trim(),systemAudio:f.has('systemAudio'),microphone:f.has('microphone'),session:f.get('session'),resolution:f.get('resolution'),showCursor:f.has('showCursor')});try{await persistPreferences({...prefs,capture:next});monitorChosen=true;settings=next;localStorage.setItem('afterglow-settings',JSON.stringify(settings));render();toast('Capture preferences saved. Restart replay to apply changes.');}catch(error){toast(String(error),true);}});
 const form=document.querySelector('#settings');
 if(form){const budget=()=>{const seconds=Number(form.elements.seconds.value),select=form.elements.bitrate;for(const option of select.options)option.disabled=seconds*Number(option.value)>1440;if(select.selectedOptions[0].disabled)select.value='12';document.querySelector('#buffer-estimate').textContent=`About ${seconds*Number(select.value)/8} MB of encoded video; maximum 180 MB.`;};form.elements.seconds.onchange=budget;form.elements.bitrate.onchange=budget;budget();}
 document.querySelectorAll('[data-path]').forEach(b=>b.onclick=()=>openPlayer(b.dataset.path));
 const dialog=document.querySelector('#player'); document.querySelector('#close-player').onclick=()=>dialog.close();
 document.querySelector('#resume-preview').onclick=()=>{if(!busy&&dialog.dataset.mediaUrl){const v=dialog.querySelector('video');v.src=dialog.dataset.mediaUrl;v.load();document.querySelector('#resume-preview').hidden=true;}};
 dialog.addEventListener('close',()=>{if(native)invoke('set_editor_active',{active:false}).catch(()=>{});dialog.querySelector('video').pause();dialog.querySelector('video').removeAttribute('src');dialog.querySelector('video').load();if(!busy)render();});
 document.querySelector('#rename-form').onsubmit=async e=> {
   e.preventDefault();const name=dialog.dataset.name, newName=document.querySelector('#rename-value').value.trim().replace(/\.mp4$/i,'')+'.mp4';
   try {await invoke('rename_clip',{name,newName});dialog.close();await refresh(true);toast('Clip renamed.');}catch(error){toast(String(error),true);}
 };
 document.querySelector('#trim-form').onsubmit=async e=> {
   e.preventDefault(); const start=Number(document.querySelector('#trim-start').value),end=Number(document.querySelector('#trim-end').value);
   if(!Number.isFinite(start)||!Number.isFinite(end)||start<0||end-start<0.1){toast('Choose a valid range of at least 0.1 seconds.',true);return;}
   const precise=document.querySelector('#trim-mode').value==='precise';
   busy=true;pausePreview(dialog);dialog.querySelectorAll('.clip-tools button,.clip-tools input,.clip-tools select,#resume-preview').forEach(b=>b.disabled=true);document.querySelector('#trim-status').classList.remove('export-error');document.querySelector('#trim-status').textContent=precise?'Exporting an accurate cut… Preview is paused to keep memory low.':'Exporting a fast trimmed copy…';
   let message='';
   try{const name=await invoke('trim_clip',{name:dialog.dataset.name,start,end,precise});message='Saved '+name;dialog.close();await refresh(true);toast(message);}
   catch(error){message=String(error);if(dialog.open){document.querySelector('#trim-status').textContent=message+' The original clip is kept; you can retry here.';document.querySelector('#trim-status').classList.add('export-error');}else{toast(message,true);}}
   finally{busy=false;if(dialog.isConnected)dialog.querySelectorAll('.clip-tools button,.clip-tools input,.clip-tools select,#resume-preview').forEach(b=>b.disabled=false);if(!dialog.open)await refresh();}

 };
}
async function openPlayer(path) {
 const d=document.querySelector('#player'),v=d.querySelector('video'),name=path.split('/').pop();
 if(native){try{await invoke('set_editor_active',{active:true});}catch(error){toast(String(error),true);return;}}
 d.dataset.name=name;d.dataset.mediaUrl='';document.querySelector('#resume-preview').hidden=true;document.querySelector('#trim-status').classList.remove('export-error');document.querySelector('#trim-mode').value='fast'; document.querySelector('#video-name').textContent=name;document.querySelector('#rename-value').value=name.replace(/\.mp4$/i,'');
 document.querySelector('#trim-end').value='';document.querySelector('#trim-start').value='0';document.querySelector('#trim-status').textContent='Fast trim keeps the original quality and may include time near a keyframe. Choose frame-accurate for an exact cut.';
 d.querySelectorAll('.clip-tools button').forEach(b=>b.disabled=false);document.querySelector('#trim-form button').disabled=true;
 v.onerror=()=>toast('Cannot play this clip in WebKit. Use Open folder to play it externally.',true);d.showModal();
 try{const url=await invoke('clip_media_url',{name});if(d.open&&d.dataset.name===name){d.dataset.mediaUrl=url;v.src=url;v.load();}const duration=await invoke('clip_duration',{name});if(d.open&&d.dataset.name===name){document.querySelector('#trim-end').value=(Math.floor(duration*100)/100).toFixed(2);document.querySelector('#trim-form button').disabled=false;}}
 catch(error){toast(String(error),true);}

}
function pausePreview(dialog){const video=dialog.querySelector('video');video.onerror=null;video.pause();video.removeAttribute('src');video.load();const resume=document.querySelector('#resume-preview');if(resume)resume.hidden=false;}
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
 const tag=document.querySelector('.status-tag');if(tag){
 tag.classList.toggle('live',state.running);tag.innerHTML=`<span class="green-dot"></span>${state.starting?'STARTING REPLAY':state.captureError?'CAPTURE NEEDS ATTENTION':state.running?'REPLAY IS ACTIVE':'REPLAY IS OFF'}`;
 document.querySelector('#toggle span').textContent=state.starting?'Starting…':state.running?'Stop replay':'Start replay';document.querySelector('#toggle').disabled=!native||busy||state.starting;
 document.querySelector('#save').disabled=!state.running||busy||state.storage?.canSave===false;document.querySelector('#game-mode').disabled=!native||!state.running||busy;
 document.querySelector('.capture-note span').textContent=`${capture.fps} FPS · H.264 · ${capture.bitrate||12} Mbps`;
 document.querySelector('.replay-duration').textContent=`${capture.seconds}s`;
 const failure=document.querySelector('#capture-error');failure.hidden=!state.captureError;failure.querySelector('span').textContent=state.captureError||'';
 document.querySelector('#storage-warning').hidden=state.storage?.canSave!==false;
 }
 const count=document.querySelector('.nav-count');if(count)count.textContent=clips.length;
 const stats=document.querySelector('.stats');if(stats){stats.querySelector('div:first-child strong').textContent=clips.length.toString().padStart(2,'0');stats.querySelector('div:nth-child(2) strong').firstChild.nodeValue=String(capture.seconds);}
 const shortcutStatus=document.querySelector('#shortcut-status');if(shortcutStatus&&state.shortcutStatus)shortcutStatus.textContent=state.shortcutStatus;
 const storedSize=document.querySelector('#storage-used-value');if(storedSize)storedSize.textContent=bytes(state.storage?.usedBytes||0);
 const capacity=document.querySelector('#capacity-label');if(capacity)capacity.textContent=`${bytes(state.storage?.usedBytes||clips.reduce((n,c)=>n+c.bytes,0))} / ${prefs.storageLimitMb?bytes(prefs.storageLimitMb*1e6):'Unlimited'}`;
 const meter=document.querySelector('#capacity-meter');if(meter)meter.style.width=`${prefs.storageLimitMb?Math.min(100,(state.storage?.usedBytes||0)/(prefs.storageLimitMb*1e6)*100):0}%`;
 const log=document.querySelector('.diagnostics pre');if(log)log.textContent=state.log||'No capture log yet.';
}
async function refresh(manual=false) {
 if (!native) {if(manual)toast('Preview mode. Use npm run tauri dev on your Linux desktop.');return;}
 try {const [statusResult,clipsResult]=await Promise.allSettled([invoke('status',{force:manual}),invoke('list_clips')]);if(statusResult.status==='rejected')throw statusResult.reason;const next=statusResult.value,items=clipsResult.status==='fulfilled'?clipsResult.value:clips;if(clipsResult.status==='rejected'&&manual)toast('Could not load clips: '+String(clipsResult.reason),true); const changed=JSON.stringify({...next,memory:null,theme:null})!==JSON.stringify({...state,memory:null,theme:null})||JSON.stringify(items)!==JSON.stringify(clips); const failed=state.running&&!next.running&&!next.starting&&!next.stoppedIntentionally;state=next;clips=items;if(next.preferences)prefs=next.preferences;applyTheme();updateMemory();updateCaptureStatus();if(changed&&!document.querySelector('#player')?.open&&page!=='setup')render();if(manual&&!document.querySelector('#player')?.open)render();if(failed)toast('Capture stopped. Open Settings → Diagnostics for the recorder error.',true);} catch(e){state.starting=false;state.captureError='Could not refresh capture status: '+String(e);updateCaptureStatus();if(manual)toast(String(e),true);}
}
async function initializePreferences(){
 try{if(native)prefs=await invoke('get_preferences');else{const stored=JSON.parse(localStorage.getItem('afterglow-desktop-preferences')||'null');if(stored)prefs={...prefs,...stored};}if(native&&!prefs.capture&&monitorChosen)prefs=await invoke('save_preferences',{prefs:{...prefs,capture:settings}});if(prefs.capture){settings=normalizeSettings(prefs.capture);monitorChosen=true;}shortcutDraft=structuredClone(prefs.shortcuts);preferencesLoaded=true;render();}catch(error){if(native)toast('Could not load saved settings: '+String(error),true);}await refresh();
}
render();initializePreferences();if(native)listen('clip-saved',async()=>{await refresh();toast('Clip saved to your library.');}).catch(()=>{});if(native)invoke('capture_options').then(options=>{captureOptions=options;if(!monitorChosen&&options.length){settings.monitor=options.find(n=>n!=='screen')||options[0];const input=document.querySelector('[name=monitor]');if(input&&input.value==='screen')input.value=settings.monitor;}const list=document.querySelector('#monitor-options');if(list)list.innerHTML=options.map(n=>`<option value="${esc(n)}"></option>`).join('');}).catch(()=>{});setInterval(()=>{if(!busy)refresh();},2500);

if(native)listen('capture-failed',async event=>{state.captureError=String(event.payload);await refresh();toast(state.captureError||'Capture could not start. Open Diagnostics.',true);}).catch(()=>{});
if(native)listen('memory-pressure',()=>{const dialog=document.querySelector('#player');if(dialog?.open){pausePreview(dialog);if(!busy)document.querySelector('#trim-status').textContent='Preview paused to keep memory low. Trimming is still available.';}}).catch(()=>{});
