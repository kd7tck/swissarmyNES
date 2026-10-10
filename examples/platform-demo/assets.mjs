// Authored source assets remain compact; the installer expands them into the
// project's editable JSON. No downloaded graphics, music, or generated ROM is needed.
const glyphs = {
  'A': [14, 17, 17, 31, 17, 17, 17], 'B': [30, 17, 17, 30, 17, 17, 30],
  'D': [30, 17, 17, 17, 17, 17, 30], 'E': [31, 16, 16, 30, 16, 16, 31],
  'F': [31, 16, 16, 30, 16, 16, 16], 'L': [16, 16, 16, 16, 16, 16, 31],
  'M': [17, 27, 21, 21, 17, 17, 17], 'O': [14, 17, 17, 17, 17, 17, 14],
  'P': [30, 17, 17, 30, 16, 16, 16], 'R': [30, 17, 17, 30, 20, 18, 17],
  'S': [15, 16, 16, 14, 1, 1, 30], 'T': [31, 4, 4, 4, 4, 4, 4],
  'V': [17, 17, 17, 17, 17, 10, 4], 'W': [17, 17, 17, 21, 21, 21, 10],
  'X': [17, 17, 10, 4, 10, 17, 17], '0': [14, 17, 19, 21, 25, 17, 14],
  '1': [4, 12, 4, 4, 4, 4, 14],
};

export function buildAssets() {
  const chr_bank = Array(8192).fill(0);
  const tile = (index, rows, color, bank) => {
    for (let y = 0; y < 8; y++) {
      const bits = rows[y] ?? 0;
      chr_bank[bank + index * 16 + y] = color & 1 ? bits : 0;
      chr_bank[bank + index * 16 + y + 8] = color & 2 ? bits : 0;
    }
  };
  for (const [letter, rows] of Object.entries(glyphs)) {
    tile(letter.charCodeAt(0), rows.map(row => row << 2), 1, 4096);
  }
  tile(1, [255, 255, 255, 255, 255, 255, 255, 255], 1, 4096);
  tile(2, [255, 129, 165, 129, 165, 129, 129, 255], 2, 4096);
  const metasprites = [];
  for (let frame = 0; frame < 3; frame++) {
    const tiles = [];
    for (let row = 0; row < 4; row++) {
      for (let col = 0; col < 2; col++) {
        const index = 128 + frame * 8 + row * 2 + col;
        const rows = Array.from({length: 8}, (_, y) => (row === 0 && y < 2 ? 60 : 126));
        tile(index, rows, frame + 1, 0);
        tiles.push({x: col * 8, y: row * 8, tile: index, attr: 0});
      }
    }
    metasprites.push({name: `Walk${frame}`, tiles});
  }
  const nametables = [0, 1].map(room => {
    const data = Array(960).fill(0);
    for (let x = 0; x < 32; x++) data[29 * 32 + x] = 1;
    const wall = room === 0 ? 22 : 10;
    for (let y = 16; y < 20; y++) data[y * 32 + wall] = 2;
    data[5 * 32 + 1] = 48 + room;
    return {name: `Room${room}`, data, attrs: Array(64).fill(0), metatile_grid: []};
  });
  return {
    chr_bank,
    palettes: [
      {name: 'BG0', colors: [15, 33, 17, 48]}, {name: 'SP0', colors: [15, 48, 33, 22]},
    ],
    nametables,
    world: {width: 2, height: 1, data: [0, 1]},
    metasprites,
    animations: [{name: 'Walk', frames: [0, 1, 2].map(frame => ({metasprite: `Walk${frame}`, duration: 5})), does_loop: true}],
    metatiles: [], samples: [], envelopes: [],
    audio_tracks: [{name: 'Theme', channel: 0, instrument: 159, priority: 1,
      notes: [36, 40, 43, 48].map((pitch, col) => ({pitch, col, row: 0, duration: 30}))}],
    sound_effects: [{name: 'Button', channel: 0, priority: 10, speed: 2, does_loop: false,
      vol_sequence: [15, 8, 0], pitch_sequence: [0], duty_sequence: [1], arpeggio_sequence: [0, 4, 7]}],
  };
}
