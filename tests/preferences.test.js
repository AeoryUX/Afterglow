import test from 'node:test';
import assert from 'node:assert/strict';
import { defaultDesktopPreferences, shortcutFromEvent, shortcutLabel } from '../src/settings-view.js';
import { normalizeSettings } from '../src/preferences.js';
test('corrupt persisted preferences fall back to safe capture settings', () => {
  for (const value of [null, 'broken', [], {seconds:'60',fps:999,monitor:'"><script>',systemAudio:'false',microphone:1}]) {
    assert.deepEqual(normalizeSettings(value), {seconds:60,bitrate:12,fps:60,monitor:'screen',systemAudio:true,microphone:false,session:'auto',resolution:'native',showCursor:true});
  }
});
test('valid preferences survive while oversized legacy buffers are bounded', () => {
  const saved={seconds:120,bitrate:20,fps:30,monitor:'HDMI-A-1',systemAudio:false,microphone:true};
  assert.deepEqual(normalizeSettings(saved), {...saved,bitrate:12,session:'auto',resolution:'native',showCursor:true});
  assert.deepEqual(normalizeSettings({...saved,seconds:30}), {...saved,seconds:30,session:'auto',resolution:'native',showCursor:true});
});

test('shortcut recorder accepts modifiers and valid keys without global side effects',()=>{assert.deepEqual(shortcutFromEvent({code:'KeyS'}),{modifiers:[],key:'S'});const shortcut=shortcutFromEvent({code:'KeyS',ctrlKey:true,shiftKey:true});assert.deepEqual(shortcut,{modifiers:['CTRL','SHIFT'],key:'S'});assert.equal(shortcutLabel(shortcut),'Ctrl + Shift + S');assert.equal(defaultDesktopPreferences().shortcuts.enabled,false);assert.equal(defaultDesktopPreferences().closeToTray,true);});

test('WebKit key capture falls back when code is absent or unidentified',()=>{assert.deepEqual(shortcutFromEvent({key:'f8',ctrlKey:true}),null);assert.deepEqual(shortcutFromEvent({key:'F8',ctrlKey:true}),{modifiers:['CTRL'],key:'F8'});assert.deepEqual(shortcutFromEvent({code:'Unidentified',key:'s',altKey:true}),{modifiers:['ALT'],key:'S'});});

test('single function keys and slash keep no modifier',()=>{assert.deepEqual(shortcutFromEvent({code:'F2'}),{modifiers:[],key:'F2'});assert.deepEqual(shortcutFromEvent({code:'Slash',key:'/'}),{modifiers:[],key:'slash'});assert.deepEqual(shortcutFromEvent({key:'/'}),{modifiers:[],key:'slash'});assert.equal(shortcutFromEvent({key:'Control'}),null);assert.equal(shortcutLabel({modifiers:[],key:'slash'}),'/');});
