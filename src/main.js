import {playbackMarkup,bindPlayer,resetPlayer,loadPlayer,pausePlayer} from './player.js';
import {getTheme, validAppearance} from './themes.js';
import { renderSettings, defaultDesktopPreferences, shortcutLabel, shortcutFromEvent } from './settings-view.js';
import { normalizeSettings } from './preferences.js';
import { listen } from '@tauri-apps/api/event';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { createIcons, Aperture, LayoutGrid, Circle, Settings2, ArrowUpRight, Video, Clock3, HardDrive, Monitor, Volume2, Mic, Zap, ChevronRight, Play, X, RefreshCw, Keyboard, Check, Cpu, House, Minus, Maximize2, Trash2, Pencil, ExternalLink, Folder, Plus } from 'lucide';
import './style.css';

const native = isTauri();
let settings;
try { settings = JSON.parse(localStorage.getItem('afterglow-settings')); } catch {}
let monitorChosen = typeof settings?.monitor === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(settings.monitor);
settings = normalizeSettings(settings);
let appearance=localStorage.getItem('afterglow-appearance')||'caelestia';
if(!validAppearance(appearance))appearance='caelestia';
let state = {running:false,gpu:'',recorderAvailable:false,wayland:false,outputDir:'~/Videos/Afterglow',log:'',starting:false};
let prefs=defaultDesktopPreferences(), settingsTab='capture', shortcutDraft=structuredClone(prefs.shortcuts), shortcutRecording=null, preferencesLoaded=false;
let shortcutEditVersion=0,shortcutSaving=false,shortcutQueued=false,shortcutTimer,shortcutError='',prefsRevision=0;
let folderData={folders:[],assignments:{}},folderFilter='all',folderRevision=0,lastFoldersRendered='';
const selectedClips=new Set();let folderTargets=[],deleteTargets=[],fullscreen=false,editorTransition=Promise.resolve(),playerOpenVersion=0;
let captureOptions = [], clips = [], busy = false, filter = '', page = 'home', clipSort = 'newest';
const icon = (name, cls='') => `<i data-lucide="${name}" class="${cls}"></i>`;
const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const bytes = b => b >= 1e9 ? `${(b/1e9).toFixed(1)} GB` : `${(b/1e6).toFixed(1)} MB`;
function toast(message, error=false) {
  let el=document.querySelector('#toast');if(!el){el=document.createElement('div');el.id='toast';el.setAttribute('role','status');document.body.append(el);} el.textContent=message; el.className=`toast visible ${error?'error':''}`;
  clearTimeout(toast.timer); toast.timer=setTimeout(()=>el.classList.remove('visible'),5000);
}
function clipCard(c,recent=false){return `<article class="clip-card ${selectedClips.has(c.name)&&!recent?'selected':''}" data-path="${esc(c.path)}">${recent?'':`<button type="button" class="clip-select" data-select-clip="${esc(c.name)}" aria-label="Select ${esc(c.name)}" aria-pressed="${selectedClips.has(c.name)}">${icon('check')}</button>`}<button type="button" class="clip-cover" data-play-clip="${esc(c.path)}" aria-label="Play ${esc(c.name)}">${icon('play')}<span>MP4</span></button><div class="clip-info"><strong>${esc(c.name)}</strong><span>${esc(new Date(c.modified*1000).toLocaleString())} · ${bytes(c.bytes)}</span><small class="clip-folder-label">${esc(folderData.folders.find(f=>f.id===folderData.assignments[c.name])?.name||'')}</small></div><div class="clip-actions" role="group" aria-label="Actions for ${esc(c.name)}"><button type="button" data-play-clip="${esc(c.path)}" aria-label="Play ${esc(c.name)}">${icon('play')} Play</button><button type="button" data-edit-clip="${esc(c.path)}" aria-label="Edit ${esc(c.name)}">${icon('pencil')} Edit</button><button type="button" data-folder-clip="${esc(c.name)}" aria-label="Move ${esc(c.name)} to folder" title="Move to folder">${icon('folder')}</button><button type="button" data-external-clip="${esc(c.name)}" aria-label="Open ${esc(c.name)} externally" title="Open externally">${icon('external-link')}</button><button type="button" data-delete-clip="${esc(c.name)}" aria-label="Delete ${esc(c.name)}" title="Delete clip" class="delete-action">${icon('trash-2')}</button></div></article>`;}
function libraryPanel(recent=false){
 const shown=clips.filter(c=>recent||((folderFilter==='all'||folderData.assignments[c.name]===folderFilter)&&c.name.toLowerCase().includes(filter.toLowerCase()))).slice().sort((a,b)=>clipSort==='name'&&!recent?a.name.localeCompare(b.name):clipSort==='oldest'&&!recent?a.modified-b.modified:b.modified-a.modified);
 return `<section class="library ${recent?'recent-clips':''}">${recent?'':folderPanel()}<div class="section-heading"><div><h2>${recent?'Recent clips':esc(folderFilter==='all'?'All clips':folderData.folders.find(f=>f.id===folderFilter)?.name||'All clips')} <span>${recent?clips.length:shown.length}</span></h2><p>${recent?'Your latest saved moments.':'Select a clip to preview, rename, or trim it.'}</p></div>${recent?`<button class="button subtle" data-page="library">View library ${icon('arrow-up-right')}</button>`:`<div class="library-tools"><input aria-label="Search clips" id="search" type="search" placeholder="Search clips" value="${esc(filter)}"><select id="clip-sort" aria-label="Sort clips">${[['newest','Newest first'],['oldest','Oldest first'],['name','Name']].map(([value,label])=>`<option value="${value}" ${clipSort===value?'selected':''}>${label}</option>`).join('')}</select></div>`}</div>${recent?'':selectionToolbar(shown)}<div class="clip-grid">${shown.length?(recent?shown.slice(0,3):shown).map(c=>clipCard(c,recent)).join(''):`<div class="empty-state">${icon('video')}<h3>${filter&&!recent?'No matching clips':'No clips yet'}</h3><p>${filter&&!recent?'Try another filename.':'Save a replay from Home or with your shortcut. It will appear here.'}</p></div>`}</div></section>`;
}
function visibleClips(){return clips.filter(c=>(folderFilter==='all'||folderData.assignments[c.name]===folderFilter)&&c.name.toLowerCase().includes(filter.toLowerCase()));}
function selectionToolbar(shown){return `<div class="selection-toolbar" role="group" aria-label="Clip selection"><strong>${selectedClips.size} selected</strong><button id="select-visible" type="button" class="button subtle" ${!shown.length?'disabled':''}>Select visible</button><button id="clear-selection" type="button" class="button subtle" ${!selectedClips.size?'disabled':''}>Clear selection</button><div class="selection-actions"><button id="move-selected" type="button" class="button secondary" ${!selectedClips.size?'disabled':''}>${icon('folder')} Move to folder</button><button id="delete-selected" type="button" class="button subtle" ${!selectedClips.size?'disabled':''}>${icon('trash-2')} Delete selected</button></div></div>`;}
function fullscreenLabel(value){fullscreen=value;document.querySelectorAll('[data-window-action=fullscreen]').forEach(b=>{b.setAttribute('aria-label',value?'Exit fullscreen':'Enter fullscreen');b.title=value?'Exit fullscreen (F11)':'Enter fullscreen (F11)';b.setAttribute('aria-pressed',String(value));});}
async function windowFullscreen(){fullscreenLabel(await invoke('window_action',{action:'fullscreen'}));}
function folderPanel(){
 const count=id=>clips.filter(c=>id==='all'||folderData.assignments[c.name]===id).length;
 const button=(id,name)=>`<button type="button" data-folder-filter="${esc(id)}" class="folder-tile ${folderFilter===id?'active':''}" aria-pressed="${folderFilter===id}">${icon('folder')}<span>${esc(name)}<small>${count(id)} ${count(id)===1?'clip':'clips'}</small></span></button>`;
 const current=folderData.folders.find(f=>f.id===folderFilter);
 return `<div class="folder-heading"><h2>Folders</h2><button id="create-folder" class="button subtle" type="button">${icon('plus')} New folder</button></div><div class="folder-grid" role="group" aria-label="Clip folders">${button('all','All clips')}${folderData.folders.map(f=>button(f.id,f.name)).join('')}</div>${current?`<div class="folder-manage"><button id="rename-folder" class="button subtle" type="button">Rename folder</button><button id="remove-folder" class="button subtle" type="button">Remove folder</button></div>`:''}`;
}
function folderDialog(){return `<dialog id="folder-dialog" aria-labelledby="folder-title"><h2 id="folder-title"></h2><form id="folder-form"><label id="folder-name-label">Folder name<input id="folder-name" maxlength="80" autocomplete="off"></label><label id="folder-target-label" hidden>Folder<select id="folder-target"></select></label><p id="folder-description"></p><p id="folder-error" class="playback-error" role="alert" hidden></p><div class="delete-buttons"><button id="folder-cancel" class="button subtle" type="button">Cancel</button><button id="folder-save" class="button primary" type="submit">Save</button></div></form></dialog>`;}
function editFolder(action,clipName=''){
 const dialog=document.querySelector('#folder-dialog'),current=folderData.folders.find(f=>f.id===folderFilter);dialog.dataset.action=action;dialog.dataset.clip=clipName;folderTargets=action==='moveMany'?[...selectedClips]:[clipName];dialog.dataset.id=current?.id||'';
 const titles={create:'New folder',rename:'Rename folder',remove:'Remove folder?',move:'Move clip to folder',moveMany:'Move selected clips'};
 document.querySelector('#folder-title').textContent=titles[action];const name=document.querySelector('#folder-name');name.value=action==='rename'?current.name:'';name.required=action==='create'||action==='rename';document.querySelector('#folder-name-label').hidden=!name.required;
 document.querySelector('#folder-target-label').hidden=!['move','moveMany'].includes(action);const select=document.querySelector('#folder-target');select.innerHTML=`<option value="">No folder</option>`+folderData.folders.map(f=>`<option value="${esc(f.id)}">${esc(f.name)}</option>`).join('');select.value=action==='moveMany'?'':folderData.assignments[clipName]||'';
 document.querySelector('#folder-description').textContent=action==='remove'?`Remove “${current.name}”? Its clips will stay in All clips.`:action==='moveMany'?`${folderTargets.length} selected clips`:action==='move'?clipName:'Folders organize clips inside Afterglow. Files stay in their current location.';
 document.querySelector('#folder-save').textContent=action==='remove'?'Remove folder':['move','moveMany'].includes(action)?'Move clips':'Save folder';document.querySelector('#folder-error').hidden=true;dialog.showModal();if(name.required)name.focus();
}
function replayPanel(capture){return `<section class="replay-card"><div class="replay-copy"><div class="replay-topline"><div class="status-tag ${state.running?'live':''}"><span class="green-dot"></span>${state.starting?'STARTING REPLAY':state.captureError?'CAPTURE NEEDS ATTENTION':state.running?'REPLAY IS ACTIVE':'REPLAY IS OFF'}</div><span class="replay-duration">${capture.seconds}s</span></div><h2>Replay buffer</h2><p>Keep the last ${capture.seconds} seconds. Save a clip whenever you need it.</p><div class="replay-actions"><button class="button primary" id="toggle" ${!native||busy||state.starting?'disabled':''}>${icon(state.running?'circle':'zap')}<span>${state.starting?'Starting…':state.running?'Stop replay':'Start replay'}</span></button><button class="button secondary" id="save" ${!state.running||busy||state.storage?.canSave===false?'disabled':''}>${icon('video')} Save clip <kbd>${esc(shortcutLabel(prefs.shortcuts.save))}</kbd></button><button class="button subtle" id="game-mode" ${!native||!state.running||busy?'disabled':''}>${icon('monitor')} Game mode</button></div><small class="capture-note">${icon('monitor')}<span>${capture.fps} FPS · H.264 · ${capture.bitrate||12} Mbps</span></small><div id="capture-error" class="capture-error" role="alert" ${state.captureError?'':'hidden'}><span>${esc(state.captureError||'')}</span><button type="button" id="capture-diagnostics">Open diagnostics</button></div><div id="storage-warning" class="storage-warning" ${state.storage?.canSave===false?'':'hidden'}>The next clip needs more storage room. <button type="button" id="manage-storage">Manage capacity</button></div></div></section>`;}
function windowButtons(){return `<div class="window-controls" role="group" aria-label="Window controls"><button data-window-action="minimize" type="button" aria-label="Minimize to tray" title="Minimize to tray" ${!native?'disabled':''}>${icon('minus')}</button><button data-window-action="fullscreen" type="button" aria-label="Enter fullscreen" title="Enter fullscreen (F11)" ${!native?'disabled':''}>${icon('maximize-2')}</button><button data-window-action="close" type="button" aria-label="Close window" title="Close window" ${!native?'disabled':''}>${icon('x')}</button></div>`;}
function render() {
 const capture=state.running?(state.activeSettings||settings):settings;
 const section=page==='setup'?(settingsTab==='keybinds'?'keybinds':'settings'):page;
 const title={home:'Home',library:'Clip library',keybinds:'Keybinds',settings:'Settings'}[section];
 const nav=(id,glyph,label,extra='')=>`<button ${id==='keybinds'?'id="keybinds-nav"':`data-page="${id==='settings'?'setup':id}"`} class="${section===id?'active':''}" ${section===id?'aria-current="page"':''} title="${label}">${icon(glyph)}<span class="nav-label">${label}</span>${extra}</button>`;
 document.querySelector('#app').innerHTML=`<aside><a class="brand" href="#" aria-label="Afterglow home">${icon('aperture')}<span>afterglow<span class="brand-dot">.</span></span></a><div class="workspace-label">WORKSPACE</div><nav aria-label="Main navigation">${nav('home','house','Home')}${nav('library','layout-grid','Clip library',`<span class="nav-count">${clips.length}</span>`)}${nav('keybinds','keyboard','Keybinds')}${nav('settings','settings-2','Settings')}</nav><div class="sidebar-bottom"><button id="storage-capacity" class="capacity-button" aria-label="Set clip-library storage capacity">${icon('hard-drive')}<span><strong>Clip storage</strong><small id="capacity-label">${bytes(state.storage?.usedBytes||clips.reduce((n,c)=>n+c.bytes,0))} / ${prefs.storageLimitMb?bytes(prefs.storageLimitMb*1e6):'Unlimited'}</small><span class="capacity-track"><span id="capacity-meter" style="width:${prefs.storageLimitMb?Math.min(100,(state.storage?.usedBytes||0)/(prefs.storageLimitMb*1e6)*100):0}%"></span></span></span></button><div class="desktop-label">${icon('monitor')}<span>Linux · ${state.session==='x11'?'X11':'Wayland'}</span></div></div></aside><main><header><div class="breadcrumb">Afterglow ${icon('chevron-right')} <span>${title}</span></div><div class="titlebar-right"><span class="version">0.2.2 <span>alpha</span></span>${windowButtons()}</div></header><div class="content"><div class="page-heading"><div><h1>${title}</h1><p>${section==='home'?'Replay, storage, and your recent clips.':section==='library'?'Your saved clips and editing tools.':section==='keybinds'?'Your actions, your shortcuts.':'Capture and desktop preferences.'}</p></div><div class="heading-actions"><button id="open-library" class="button subtle" ${!native?'disabled':''}>${icon('hard-drive')} Open folder</button><button id="refresh" class="button icon-button subtle" aria-label="Refresh" title="Refresh">${icon('refresh-cw')}</button></div></div>${!native?`<div class="preview-notice">${icon('monitor')}<span>Interface preview · Recording is available in the desktop app.</span></div>`:''}${page==='home'?`${replayPanel(capture)}<div class="stats"><div>${icon('video')}<span>Saved clips<strong>${clips.length.toString().padStart(2,'0')}</strong></span></div><div>${icon('clock-3')}<span>Replay length<strong>${capture.seconds}<small> seconds</small></strong></span></div><div>${icon('hard-drive')}<span>App + capture RAM<strong id="ram-value">${state.memory?.bytes?bytes(state.memory.bytes):'—'}</strong><small class="ram-target">Target &lt; 1 GB</small></span></div></div>${libraryPanel(true)}`:page==='library'?libraryPanel():setupPanel()}<footer><span>${icon('hard-drive')} ${esc(state.outputDir)}</span><span>Clips stay on your device</span></footer></div></main><dialog id="player"><div class="player-titlebar"><h2>Clip preview</h2><button id="close-player" class="button subtle" type="button" aria-label="Close clip">${icon('x')} Close clip</button><button id="resume-preview" class="button subtle" hidden>Resume preview</button></div>${playbackMarkup()}<p id="video-name"></p><div class="clip-tools"><form id="rename-form"><label>Clip name<input id="rename-value" required maxlength="160" placeholder="Name your highlight"></label><button type="submit" class="button secondary">Rename clip</button></form><form id="trim-form"><div class="trim-range"><label>Start (seconds)<input id="trim-start" type="number" value="0" min="0" step="0.01" required></label><label>End (seconds)<input id="trim-end" type="number" min="0.1" step="0.01" required></label></div><label class="trim-mode-label">Trim mode<select id="trim-mode"><option value="fast">Fast · keyframe cut</option><option value="precise">Frame-accurate · CPU</option></select></label><button type="submit" class="button primary">Export trimmed copy</button></form><p id="trim-status" role="status">Fast trim keeps the original quality and may include time near a keyframe. Choose frame-accurate for an exact cut.</p></div></dialog><dialog id="delete-confirm" aria-labelledby="delete-title"><h2 id="delete-title">Delete clip?</h2><p id="delete-filename"></p><p>This permanently removes this clip from your library. This cannot be undone.</p><p id="delete-error" class="playback-error" role="alert" hidden></p><div class="delete-buttons"><button id="delete-cancel" class="button subtle" type="button" autofocus>Cancel</button><button id="delete-accept" class="button primary" type="button">Delete clip</button></div></dialog>${folderDialog()}`;
 createIcons({icons:{Aperture,LayoutGrid,Circle,Settings2,ArrowUpRight,Video,Clock3,HardDrive,Monitor,Volume2,Mic,Zap,ChevronRight,Play,X,RefreshCw,Keyboard,Check,Cpu,House,Minus,Maximize2,Trash2,Pencil,ExternalLink,Folder,Plus}});
 lastFoldersRendered=JSON.stringify(folderData);bind();bindSettings();applyTheme();updateMemory();
}
function setupPanel() {
 return renderSettings({tab:settingsTab,settings,appearance,state:{...state,shortcutError},options:captureOptions,prefs,shortcutDraft,recording:shortcutRecording,icon,esc,bytes,native,themeLabel});
}
async function persistPreferences(next) {
 if(native)prefs=await invoke('save_preferences',{prefs:next});else{prefs=next;localStorage.setItem('afterglow-desktop-preferences',JSON.stringify(prefs));}
 prefsRevision++;return prefs;
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
 document.querySelectorAll('[data-shortcut-modifier],[data-shortcut-key]').forEach(input=>{input.onfocus=()=>{shortcutRecording=null;};input.onchange=()=>{const id=input.dataset.shortcutModifier||input.dataset.shortcutKey;shortcutDraft[id]={modifiers:document.querySelector(`[data-shortcut-modifier=${id}]`).value.split(' ').filter(Boolean),key:document.querySelector(`[data-shortcut-key=${id}]`).value};shortcutRecording=null;shortcutChanged();render();};});
 document.querySelectorAll('.copy-command').forEach(input=>input.onclick=()=>input.select());
 const apply=document.querySelector('#apply-shortcuts');if(apply)apply.onclick=()=>applyShortcutDraft(true);
 const remove=document.querySelector('#remove-shortcuts');if(remove)remove.onclick=async()=>{clearTimeout(shortcutTimer);shortcutEditVersion++;shortcutQueued=false;try{prefs=await invoke('apply_shortcuts',{shortcuts:{...prefs.shortcuts,enabled:false}});prefsRevision++;shortcutError='';shortcutDraft=structuredClone(prefs.shortcuts);state.shortcutStatus='Afterglow shortcuts removed.';render();toast('Shortcuts removed.');}catch(e){toast(String(e),true);}};
 const storage=document.querySelector('#storage-settings');if(storage){storage.elements.unlimited.onchange=()=>{storage.elements.capacityGb.disabled=storage.elements.unlimited.checked;};document.querySelectorAll('[data-capacity]').forEach(button=>button.onclick=()=>{const mb=Number(button.dataset.capacity);storage.elements.unlimited.checked=mb===0;storage.elements.capacityGb.disabled=mb===0;if(mb)storage.elements.capacityGb.value=mb/1000;document.querySelectorAll('[data-capacity]').forEach(b=>b.classList.toggle('selected',b===button));});storage.onsubmit=async e=>{e.preventDefault();const mb=storage.elements.unlimited.checked?0:Math.round(Number(storage.elements.capacityGb.value)*1000);try{await persistPreferences({...prefs,storageLimitMb:mb});render();await refresh();toast('Storage capacity saved. Existing clips are kept.');}catch(error){toast(String(error),true);}};}
 const behavior=document.querySelector('#behavior-settings');if(behavior){behavior.elements.startWithSystem.onchange=()=>{behavior.elements.showWindowOnLogin.disabled=!behavior.elements.startWithSystem.checked;};behavior.elements.appearance.onchange=e=>{appearance=e.target.value;localStorage.setItem('afterglow-appearance',appearance);applyTheme();};behavior.onsubmit=async e=>{e.preventDefault();try{await persistPreferences({...prefs,closeToTray:behavior.elements.closeToTray.checked,autoStartReplay:behavior.elements.autoStartReplay.checked,startInTray:behavior.elements.startInTray.checked,clipSound:behavior.elements.clipSound.checked,startWithSystem:behavior.elements.startWithSystem.checked,showWindowOnLogin:behavior.elements.showWindowOnLogin.checked});render();toast('Behavior preferences saved.');}catch(error){toast(String(error),true);}};}
}
document.addEventListener('keydown',e=>{if(!shortcutRecording)return;e.preventDefault();e.stopPropagation();if(e.key==='Escape'){shortcutRecording=null;render();return;}const shortcut=shortcutFromEvent(e);if(shortcut){shortcutDraft[shortcutRecording]=shortcut;shortcutRecording=null;shortcutChanged();render();}},true);
function shortcutChanged(){shortcutEditVersion++;shortcutQueued=true;shortcutError='';clearTimeout(shortcutTimer);shortcutTimer=setTimeout(()=>applyShortcutDraft(false),350);}
async function applyShortcutDraft(notify){
 clearTimeout(shortcutTimer);if(shortcutSaving)return;
 const version=shortcutEditVersion,shortcuts={...structuredClone(shortcutDraft),enabled:true};shortcutSaving=true;shortcutQueued=false;state.shortcutStatus='Applying shortcuts…';updateCaptureStatus();
 try{
  const updated=native?await invoke('apply_shortcuts',{shortcuts}):{...prefs,shortcuts:{...shortcuts,enabled:false}};
  if(version!==shortcutEditVersion&&!shortcutQueued)return;
  prefs=updated;prefsRevision++;state.preferences=updated;shortcutError='';
  if(version===shortcutEditVersion)shortcutDraft=structuredClone(prefs.shortcuts);
  if(!native)localStorage.setItem('afterglow-desktop-preferences',JSON.stringify(prefs));
  state.shortcutStatus=native?'Shortcuts active in Hyprland, including from the tray.':'Preview updated. Apply shortcuts in the desktop app.';
  if(!shortcutRecording&&!document.querySelector('dialog[open]')&&(page!=='setup'||settingsTab==='keybinds'))render();else updateCaptureStatus();if(notify)toast('Shortcuts applied.');
 }catch(error){shortcutError=String(error);state.shortcutStatus='Could not apply changes. Review the error and retry.';updateCaptureStatus();if(notify)toast(shortcutError,true);}
 finally{shortcutSaving=false;if(shortcutQueued){clearTimeout(shortcutTimer);shortcutTimer=setTimeout(()=>applyShortcutDraft(false),100);}}
}
function onClick(id,handler){document.getElementById(id)?.addEventListener('click',handler);}
function bind() {
 document.querySelectorAll('[data-window-action]').forEach(button=>button.onclick=async()=>{try{fullscreenLabel(await invoke('window_action',{action:button.dataset.windowAction}));}catch(e){toast(String(e),true);}});
 if(native)invoke('window_action',{action:'state'}).then(fullscreenLabel).catch(()=>{});
 const header=document.querySelector('header');header.onmousedown=e=>{if(native&&e.button===0&&!e.target.closest('button,input,select'))invoke('window_action',{action:'drag'}).catch(()=>{});};header.ondblclick=e=>{if(native&&!e.target.closest('button,input,select'))windowFullscreen().catch(e=>toast(String(e),true));};
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
 document.querySelectorAll('[data-folder-filter]').forEach(b=>b.onclick=()=>{folderFilter=b.dataset.folderFilter;filter='';render();});
 onClick('create-folder',()=>editFolder('create'));onClick('rename-folder',()=>editFolder('rename'));onClick('remove-folder',()=>editFolder('remove'));
 document.querySelectorAll('[data-select-clip]').forEach(b=>b.onclick=()=>{const name=b.dataset.selectClip;if(selectedClips.has(name))selectedClips.delete(name);else if(selectedClips.size<500)selectedClips.add(name);else toast('Select up to 500 clips at a time.',true);render();});
 onClick('select-visible',()=>{for(const c of visibleClips()){if(selectedClips.size>=500)break;selectedClips.add(c.name);}render();});
 onClick('clear-selection',()=>{selectedClips.clear();render();});
 onClick('move-selected',()=>editFolder('moveMany'));
 onClick('delete-selected',()=>confirmDelete([...selectedClips]));
 document.querySelectorAll('[data-folder-clip]').forEach(b=>b.onclick=()=>editFolder('move',b.dataset.folderClip));
 const foldersDialog=document.querySelector('#folder-dialog');onClick('folder-cancel',()=>foldersDialog.close());
 document.querySelector('#folder-form').onsubmit=async e=>{e.preventDefault();const save=document.querySelector('#folder-save');save.disabled=true;try{const action=foldersDialog.dataset.action,id=foldersDialog.dataset.id,name=document.querySelector('#folder-name').value;
 const command={create:'create_folder',rename:'rename_folder',remove:'remove_folder',move:'assign_clip_folder',moveMany:'assign_clips_folder'}[action];const args=action==='moveMany'?{names:folderTargets,folderId:document.querySelector('#folder-target').value||null}:action==='move'?{name:foldersDialog.dataset.clip,folderId:document.querySelector('#folder-target').value||null}:action==='remove'?{id}:action==='rename'?{id,name}:{name};
 folderData=await invoke(command,args);folderRevision++;if(action==='moveMany')selectedClips.clear();if(action==='remove')folderFilter='all';foldersDialog.close();await refresh(true);toast(action==='remove'?'Folder removed. Clips kept.':'Folder updated.');
 }catch(error){const box=document.querySelector('#folder-error');box.hidden=false;box.textContent=String(error);}finally{if(save.isConnected)save.disabled=false;}};
 foldersDialog.addEventListener('close',()=>{if(!document.querySelector('dialog[open]'))render();});
 document.querySelectorAll('[data-path]').forEach(b=>b.onclick=e=>{if(!e.target.closest('button'))openPlayer(b.dataset.path);});
 document.querySelectorAll('[data-play-clip]').forEach(b=>b.onclick=()=>openPlayer(b.dataset.playClip));
 document.querySelectorAll('[data-edit-clip]').forEach(b=>b.onclick=()=>openPlayer(b.dataset.editClip,true));
 document.querySelectorAll('[data-external-clip]').forEach(b=>b.onclick=async()=>{try{await invoke('open_clip_external',{name:b.dataset.externalClip});}catch(e){toast(String(e),true);}});
 document.querySelectorAll('[data-delete-clip]').forEach(b=>b.onclick=()=>confirmDelete(b.dataset.deleteClip));
 const deletion=document.querySelector('#delete-confirm');
 onClick('delete-cancel',()=>deletion.close());
 onClick('delete-accept',async()=>{const button=document.querySelector('#delete-accept');button.disabled=true;try{
 const result=deleteTargets.length===1?(await invoke('delete_clip',{name:deleteTargets[0]}),{deleted:[deleteTargets[0]],failed:[]}):await invoke('delete_clips',{names:deleteTargets});
 for(const name of result.deleted)selectedClips.delete(name);
 if(result.failed.length){deleteTargets=result.failed.map(f=>f.name);const error=document.querySelector('#delete-error');error.hidden=false;error.textContent=`${result.deleted.length} clips deleted. Could not delete:\n`+result.failed.map(f=>`${f.name}: ${f.error}`).join('\n');document.querySelector('#delete-filename').textContent=deleteTargets.join('\n');button.textContent='Retry remaining';await refresh();}
 else{deletion.close();await refresh(true);toast(`${result.deleted.length} ${result.deleted.length===1?'clip':'clips'} deleted.`);}
 }catch(e){const error=document.querySelector('#delete-error');error.hidden=false;error.textContent=String(e);}finally{if(button.isConnected)button.disabled=false;}});
 deletion.addEventListener('close',()=>{if(!document.querySelector('dialog[open]'))refresh(true);});
 const dialog=document.querySelector('#player'); document.querySelector('#close-player').onclick=()=>dialog.close();
 bindPlayer(dialog,{invoke,native});
 dialog.addEventListener('close',()=>{playerOpenVersion++;if(native)editorTransition=editorTransition.catch(()=>{}).then(()=>invoke('set_editor_active',{active:false})).catch(()=>{});dialog.querySelector('video').pause();dialog.querySelector('video').removeAttribute('src');dialog.querySelector('video').load();if(!busy)render();});
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
async function openPlayer(path,editing=false) {
 const ticket=++playerOpenVersion,name=path.split('/').pop();
 if(native){try{await (editorTransition=editorTransition.catch(()=>{}).then(()=>invoke('set_editor_active',{active:true})));}catch(error){toast(String(error),true);return;}}
 if(ticket!==playerOpenVersion)return;
 // Status polling may have rendered a new dialog while activation was pending.
 const d=document.querySelector('#player');if(!d)return;
 d.dataset.name=name;d.dataset.mediaUrl='';document.querySelector('#resume-preview').hidden=true;document.querySelector('#trim-status').classList.remove('export-error');document.querySelector('#trim-mode').value='fast';document.querySelector('#video-name').textContent=name;document.querySelector('#rename-value').value=name.replace(/\.mp4$/i,'');
 document.querySelector('#trim-end').value='';document.querySelector('#trim-start').value='0';document.querySelector('#trim-status').textContent='Fast trim keeps the original quality and may include time near a keyframe. Choose frame-accurate for an exact cut.';
 d.querySelectorAll('.clip-tools button').forEach(b=>b.disabled=false);document.querySelector('#trim-form button').disabled=true;
 resetPlayer(d);d.showModal();if(editing)document.querySelector('#rename-value').focus();
 const current=()=>ticket===playerOpenVersion&&d.open&&d.isConnected&&d.dataset.name===name;
 await Promise.allSettled([
  invoke('clip_media_url',{name}).then(url=>{if(current())loadPlayer(d,url,!editing);}).catch(error=>{if(current()){document.querySelector('#playback-error').hidden=false;document.querySelector('#playback-error').textContent=String(error);document.querySelector('#playback-status').textContent='The clip could not be loaded. Open externally or check Diagnostics.';}}),
  invoke('clip_duration',{name}).then(duration=>{if(current()){document.querySelector('#trim-end').value=(Math.floor(duration*100)/100).toFixed(2);document.querySelector('#trim-form button').disabled=false;}}).catch(error=>{if(current())document.querySelector('#trim-status').textContent='Trimming is unavailable until clip details can be read: '+String(error);})
 ]);
}

function confirmDelete(names){if(busy){toast('Wait for the current operation to finish.',true);return;}deleteTargets=Array.isArray(names)?[...names]:[names];if(!deleteTargets.length)return;const d=document.querySelector('#delete-confirm');document.querySelector('#delete-title').textContent=deleteTargets.length===1?'Delete clip?':`Delete ${deleteTargets.length} clips?`;document.querySelector('#delete-filename').textContent=deleteTargets.join('\n');document.querySelector('#delete-error').hidden=true;document.querySelector('#delete-accept').disabled=false;document.querySelector('#delete-accept').textContent=deleteTargets.length===1?'Delete clip':`Delete ${deleteTargets.length} clips`;d.showModal();}

function pausePreview(dialog){pausePlayer(dialog);}
function themeLabel(){return appearance==='caelestia'?(state.theme?.available?`Synced with ${state.theme.name}${state.theme.error?' (waiting for scheme update)':''}`:'Caelestia scheme unavailable; using Afterglow dark.'):getTheme(appearance).label;}
function applyTheme(){
 const builtin=getTheme(appearance);
 const light=appearance==='caelestia'&&state.theme?.available?state.theme.light:builtin.light;
 const base={...builtin.colours};
 if(appearance==='caelestia'&&state.theme?.available){const map={surface:'background',surfaceContainer:'surface',surfaceContainerLow:'surface-low',surfaceContainerHigh:'surface-high',onSurface:'foreground',onSurfaceVariant:'muted',outlineVariant:'border',primary:'accent',onPrimary:'on-accent',primaryContainer:'accent-container',onPrimaryContainer:'on-accent-container'};for(const [source,target]of Object.entries(map)){const value=state.theme.colours?.[source];if(/^#[0-9a-f]{6}$/i.test(value||''))base[target]=value;}}
 for(const[key,value]of Object.entries(base))document.documentElement.style.setProperty('--'+key,value);
 document.documentElement.style.colorScheme=light?'light':'dark';
 const status=document.querySelector('#theme-status');if(status)status.textContent=themeLabel();
}
function updateMemory(){const el=document.querySelector('#ram-value');if(el){el.textContent=state.memory?.bytes?`${state.memory.approximate?'~ ':''}${bytes(state.memory.bytes)}`:'—';el.classList.toggle('ram-warning',(state.memory?.bytes||0)>=750000000);el.title='Estimated memory across Afterglow, playback processes, and its recorder. Shared memory is apportioned when available.';}}
function updateCaptureStatus(){
 const capture=state.running?(state.activeSettings||settings):settings;
 const saveKey=document.querySelector('#save kbd');if(saveKey)saveKey.textContent=shortcutLabel(prefs.shortcuts.save);
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
 const errorBox=document.querySelector('#shortcut-error');if(errorBox){errorBox.hidden=!shortcutError;errorBox.textContent=shortcutError;}
 const shortcutStatus=document.querySelector('#shortcut-status');if(shortcutStatus&&state.shortcutStatus)shortcutStatus.textContent=state.shortcutStatus;
 const storedSize=document.querySelector('#storage-used-value');if(storedSize)storedSize.textContent=bytes(state.storage?.usedBytes||0);
 const capacity=document.querySelector('#capacity-label');if(capacity)capacity.textContent=`${bytes(state.storage?.usedBytes||clips.reduce((n,c)=>n+c.bytes,0))} / ${prefs.storageLimitMb?bytes(prefs.storageLimitMb*1e6):'Unlimited'}`;
 const meter=document.querySelector('#capacity-meter');if(meter)meter.style.width=`${prefs.storageLimitMb?Math.min(100,(state.storage?.usedBytes||0)/(prefs.storageLimitMb*1e6)*100):0}%`;
 const log=document.querySelector('.diagnostics pre');if(log)log.textContent=state.log||'No capture log yet.';
}
async function refresh(manual=false) {
 if (!native) {if(manual)toast('Preview mode. Use npm run tauri dev on your Linux desktop.');return;}
 const revision=prefsRevision,folderVersion=folderRevision;try {const [statusResult,clipsResult,foldersResult]=await Promise.allSettled([invoke('status',{force:manual}),invoke('list_clips'),invoke('get_folders')]);if(statusResult.status==='rejected')throw statusResult.reason;if(foldersResult.status==='fulfilled'&&folderVersion===folderRevision){folderData=foldersResult.value;if(folderFilter!=='all'&&!folderData.folders.some(f=>f.id===folderFilter))folderFilter='all';}else if(foldersResult.status==='rejected'&&manual)toast('Could not load folders: '+String(foldersResult.reason),true);const next=statusResult.value,items=clipsResult.status==='fulfilled'?clipsResult.value:clips;if(clipsResult.status==='rejected'&&manual)toast('Could not load clips: '+String(clipsResult.reason),true); const changed=(foldersResult.status==='fulfilled'&&JSON.stringify(foldersResult.value)!==lastFoldersRendered)||JSON.stringify({...next,memory:null,theme:null})!==JSON.stringify({...state,memory:null,theme:null})||JSON.stringify(items)!==JSON.stringify(clips); const failed=state.running&&!next.running&&!next.starting&&!next.stoppedIntentionally;state=next;clips=items;for(const name of selectedClips)if(!clips.some(c=>c.name===name))selectedClips.delete(name);if(next.preferences&&revision===prefsRevision)prefs=next.preferences;else if(next.preferences)state.preferences=prefs;applyTheme();updateMemory();updateCaptureStatus();if(changed&&!document.querySelector('dialog[open]')&&page!=='setup')render();if(manual&&!document.querySelector('dialog[open]'))render();if(failed)toast('Capture stopped. Open Settings → Diagnostics for the recorder error.',true);} catch(e){state.starting=false;state.captureError='Could not refresh capture status: '+String(e);updateCaptureStatus();if(manual)toast(String(e),true);}
}
async function initializePreferences(){
 try{if(native)prefs=await invoke('get_preferences');else{const stored=JSON.parse(localStorage.getItem('afterglow-desktop-preferences')||'null');if(stored)prefs={...prefs,...stored};}if(native&&!prefs.capture&&monitorChosen)prefs=await invoke('save_preferences',{prefs:{...prefs,capture:settings}});if(prefs.capture){settings=normalizeSettings(prefs.capture);monitorChosen=true;}shortcutDraft=structuredClone(prefs.shortcuts);preferencesLoaded=true;render();}catch(error){if(native)toast('Could not load saved settings: '+String(error),true);}await refresh();
}
render();initializePreferences();if(native)listen('clip-saved',async()=>{await refresh();toast('Clip saved to your library.');}).catch(()=>{});if(native)invoke('capture_options').then(options=>{captureOptions=options;if(!monitorChosen&&options.length){settings.monitor=options.find(n=>n!=='screen')||options[0];const input=document.querySelector('[name=monitor]');if(input&&input.value==='screen')input.value=settings.monitor;}const list=document.querySelector('#monitor-options');if(list)list.innerHTML=options.map(n=>`<option value="${esc(n)}"></option>`).join('');}).catch(()=>{});setInterval(()=>{if(!busy)refresh();},2500);

if(native)listen('capture-failed',async event=>{state.captureError=String(event.payload);await refresh();toast(state.captureError||'Capture could not start. Open Diagnostics.',true);}).catch(()=>{});
if(native)listen('memory-pressure',()=>{const dialog=document.querySelector('#player');if(dialog?.open){pausePreview(dialog);if(!busy)document.querySelector('#trim-status').textContent='Preview paused to keep memory low. Trimming is still available.';}}).catch(()=>{});

if(native)listen('tauri://resize',()=>{invoke('window_action',{action:'state'}).then(fullscreenLabel).catch(()=>{});}).catch(()=>{});
document.addEventListener('keydown',e=>{if(e.defaultPrevented||shortcutRecording||!native)return;if(e.key==='F11'){e.preventDefault();windowFullscreen().catch(error=>toast(String(error),true));}else if(e.key==='Escape'&&fullscreen&&!document.querySelector('dialog[open]')){e.preventDefault();invoke('window_action',{action:'windowed'}).then(fullscreenLabel).catch(()=>{});}});
