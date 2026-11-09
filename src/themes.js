const palette=(background,surface,surfaceLow,surfaceHigh,foreground,muted,border,accent,onAccent,accentContainer,onAccentContainer)=>({background,surface,'surface-low':surfaceLow,'surface-high':surfaceHigh,foreground,muted,border,accent,'on-accent':onAccent,'accent-container':accentContainer,'on-accent-container':onAccentContainer});
export const themes={
 dark:{label:'Forest dark',light:false,colours:palette('#101211','#20281c','#141715','#2b3427','#e7eae6','#a2af98','#3a4532','#c1ec91','#203016','#283322','#d3edb5')},
 light:{label:'Forest light',light:true,colours:palette('#f7faf0','#eaf0df','#f1f5e9','#e1e8d4','#1a2115','#46513d','#c5ceba','#486d24','#ffffff','#c8edaa','#284015')},
 graphite:{label:'Graphite',light:false,colours:palette('#141416','#232326','#1b1b1e','#2e2e32','#ededf0','#b0b0b8','#44444c','#c9c9df','#262633','#343442','#ececff')},
 midnight:{label:'Midnight blue',light:false,colours:palette('#0e141e','#1c2738','#151d2b','#29354a','#e4ebf8','#a6b5cf','#40516d','#a9c9ff','#0e305e','#274467','#d5e6ff')},
 lavender:{label:'Lavender',light:false,colours:palette('#16121d','#282232','#1e1927','#342d40','#eee7f6','#bcb0cd','#51465f','#d4bcff','#362153','#493364','#edddff')},
 rose:{label:'Rose light',light:true,colours:palette('#fff8fa','#f5e5eb','#faeff3','#edd9e2','#2c1b24','#66515d','#cfb9c5','#8a3f68','#ffffff','#ffd8e9','#42132d')},
 sand:{label:'Sand light',light:true,colours:palette('#fcf8f0','#eee6d9','#f6efe4','#e4d9c8','#292117','#655b4b','#c9bba6','#765729','#ffffff','#f4dfb8','#3c2b12')},
};
export const themeChoices=[['caelestia','Follow Caelestia'],...Object.entries(themes).map(([key,t])=>[key,t.label])];
export const validAppearance=value=>value==='caelestia'||Object.hasOwn(themes,value);
export const getTheme=value=>themes[value]||themes.dark;
