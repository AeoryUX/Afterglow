import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeSettings } from '../src/preferences.js';
test('corrupt persisted preferences fall back to safe capture settings', () => {
  for (const value of [null, 'broken', [], {seconds:'60',fps:999,monitor:'"><script>',systemAudio:'false',microphone:1}]) {
    assert.deepEqual(normalizeSettings(value), {seconds:60,bitrate:12,fps:60,monitor:'screen',systemAudio:true,microphone:false});
  }
});
test('valid preferences survive while oversized legacy buffers are bounded', () => {
  const saved={seconds:120,bitrate:20,fps:30,monitor:'HDMI-A-1',systemAudio:false,microphone:true};
  assert.deepEqual(normalizeSettings(saved), {...saved,bitrate:12});
  assert.deepEqual(normalizeSettings({...saved,seconds:30}), {...saved,seconds:30});
});
