const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const {domFixture} = require('./dom_fixture.cjs');

test('PPU viewer stays hidden, validates buffer bounds and uses memory after growth', () => {
    const f = domFixture();
    f.load('static/js/ppu_viewer.js', ['PpuViewer']);
    const viewer = new f.context.PpuViewer();
    assert.equal(viewer.container.style.display, 'none');
    assert.equal(viewer.isVisible, false);
    viewer.show(); viewer.hide();
    assert.equal(f.frames.size, 0);
    viewer.emulator = {update_pattern_tables() {}, get_pattern_tables: () => 0, get_pattern_tables_len: () => 256 * 128 * 4};
    viewer.drawPatternTables(); // Missing memory must be safe.
    f.window.wasmMemory = {buffer: new ArrayBuffer(256 * 128 * 4)};
    viewer.drawPatternTables();
    f.window.wasmMemory.buffer = new ArrayBuffer(256 * 128 * 4 + 16);
    new Uint8Array(f.window.wasmMemory.buffer)[0] = 77;
    viewer.drawPatternTables();
    assert.equal(viewer.patternCanvas.lastImage.data[0], 77);
    viewer.emulator.get_pattern_tables = () => 17;
    assert.throws(() => viewer.drawPatternTables(), /Invalid pattern buffer bounds/);
    viewer.emulator.get_pattern_tables_len = () => 4;
    assert.throws(() => viewer.drawPatternTables(), /Invalid pattern buffer length/);
});

test('nametable viewport wraps horizontal and vertical seams without a second loop', () => {
    const f = domFixture();
    f.load('static/js/ppu_viewer.js', ['PpuViewer']);
    const viewer = new f.context.PpuViewer();
    f.window.wasmMemory = {buffer: new ArrayBuffer(512 * 480 * 4)};
    viewer.emulator = {update_nametables() {}, get_nametables: () => 0, get_nametables_len: () => 512 * 480 * 4, viewport_position: () => [400, 300]};
    const rectangles = [];
    viewer.nametableCanvas.context.strokeRect = (...rect) => rectangles.push(rect);
    viewer.currentTab = 'Nametables'; viewer.show();
    assert.deepEqual(rectangles, [[400, 300, 256, 240], [400, -180, 256, 240], [-112, 300, 256, 240], [-112, -180, 256, 240]]);
    assert.equal(f.frames.size, 0);
});

test('palette viewer consumes all 32 colors in two rows', () => {
    const bytes = Uint8Array.from({length: 128}, (_, i) => i);
    const context = vm.createContext({window: {wasmMemory: {buffer: bytes.buffer}}});
    vm.runInContext(fs.readFileSync('static/js/ppu_viewer.js', 'utf8') + '\nthis.Viewer = PpuViewer;', context);
    const viewer = Object.create(context.Viewer.prototype);
    let rendered;
    viewer.emulator = {update_palettes() {}, get_palettes: () => 0, get_palettes_len: () => 128};
    viewer.paletteCanvas = {style: {}, getContext: () => ({
        createImageData(width, height) {
            assert.equal(width, 16);
            assert.equal(height, 2);
            return {data: new Uint8Array(width * height * 4)};
        },
        putImageData(data) {rendered = data.data;}
    })};
    viewer.drawPalettes();
    assert.deepEqual(rendered, bytes);
    assert.equal(viewer.paletteCanvas.width, 16);
    assert.equal(viewer.paletteCanvas.height, 2);
    viewer.emulator.get_palettes_len = () => 124;
    assert.throws(() => viewer.drawPalettes(), /Invalid palette buffer/);
});

test('OAM entry formatting pads tile hex and attr binary correctly', () => {
    const memory = new Uint8Array(256);
    // Sprite 0: Y=10, Tile=0x0A, Attr=3, X=20
    memory[0] = 10;
    memory[1] = 0x0A;
    memory[2] = 3;
    memory[3] = 20;

    // Sprite 1: Offscreen Y=240
    memory[4] = 240;

    const context = vm.createContext({window: {}});
    vm.runInContext(fs.readFileSync('static/js/ppu_viewer.js', 'utf8') + '\nthis.Viewer = PpuViewer;', context);

    const formatted = context.Viewer.formatOamEntries(memory);
    assert.match(formatted, /Sprite 0: X=20 Y=10 Tile=\$0A Attr=00000011/);
    assert.match(formatted, /color:#555/); // Offscreen sprite highlighted dim
    assert.match(formatted, /Top=11 Palette=3 Priority=front Flip=--/);
});

test('OAM entry formatting handles empty or null input gracefully', () => {
    const context = vm.createContext({window: {}});
    vm.runInContext(fs.readFileSync('static/js/ppu_viewer.js', 'utf8') + '\nthis.Viewer = PpuViewer;', context);

    assert.equal(context.Viewer.formatOamEntries(null), 'No OAM data available');
    assert.equal(context.Viewer.formatOamEntries(new Uint8Array(0)), 'No OAM data available');
});
