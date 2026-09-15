const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

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
