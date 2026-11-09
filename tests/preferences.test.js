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

test('shortcut recorder accepts modifiers and valid keys without global side effects',()=>{assert.equal(shortcutFromEvent({code:'KeyS'}),null);const shortcut=shortcutFromEvent({code:'KeyS',ctrlKey:true,shiftKey:true});assert.deepEqual(shortcut,{modifiers:['CTRL','SHIFT'],key:'S'});assert.equal(shortcutLabel(shortcut),'Ctrl + Shift + S');assert.equal(defaultDesktopPreferences().shortcuts.enabled,false);assert.equal(defaultDesktopPreferences().closeToTray,true);});
