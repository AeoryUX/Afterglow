import test from 'node:test';
import assert from 'node:assert/strict';
import {themes,themeChoices,getTheme,validAppearance} from '../src/themes.js';
test('built-in themes have complete safe palettes and preserve legacy choices',()=>{for(const [id,t]of Object.entries(themes)){assert.ok(validAppearance(id));assert.equal(Object.keys(t.colours).length,11);for(const c of Object.values(t.colours))assert.match(c,/^#[a-f0-9]{6}$/i);assert.equal(typeof t.light,'boolean');}assert.ok(themeChoices.length>=8);assert.ok(validAppearance('caelestia'));assert.equal(getTheme('unknown'),themes.dark);assert.equal(validAppearance('<script>'),false);});
