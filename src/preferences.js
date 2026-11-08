export function normalizeSettings(value) {
  const saved = value && typeof value === 'object' && !Array.isArray(value) ? value : {};
  const pick = (key, allowed, fallback) => allowed.includes(saved[key]) ? saved[key] : fallback;
  const seconds = pick('seconds', [30, 60, 90, 120], 60);
  let bitrate = pick('bitrate', [8, 12, 16, 20], 12);
  if (seconds * bitrate > 1440) bitrate = 12;
  return {seconds, bitrate, fps: pick('fps', [30, 60], 60),
    monitor: typeof saved.monitor === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(saved.monitor) ? saved.monitor : 'screen',
    systemAudio: typeof saved.systemAudio === 'boolean' ? saved.systemAudio : true,
    microphone: typeof saved.microphone === 'boolean' ? saved.microphone : false};
}
