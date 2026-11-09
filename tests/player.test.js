import test from 'node:test';
import assert from 'node:assert/strict';
import {formatTime} from '../src/player.js';
test('player time formatting handles missing metadata and long clips',()=>{assert.equal(formatTime(NaN),'0:00');assert.equal(formatTime(Infinity),'0:00');assert.equal(formatTime(-1),'0:00');assert.equal(formatTime(65.5),'1:05');assert.equal(formatTime(3600),'60:00');});
