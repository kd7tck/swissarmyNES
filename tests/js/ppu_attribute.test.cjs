const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

test('formatAttributeGrid formats 32-byte palette into BG and SP sub-palettes', () => {
    const memory = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
        memory[i] = i;
    }

    const context = vm.createContext({window: {}});
    vm.runInContext(fs.readFileSync('static/js/ppu_viewer.js', 'utf8') + '\nthis.Viewer = PpuViewer;', context);

    const formatted = context.Viewer.formatAttributeGrid(memory);
    assert.match(formatted, /BG 0: \$00 \$01 \$02 \$03/);
    assert.match(formatted, /BG 1: \$04 \$05 \$06 \$07/);
    assert.match(formatted, /SP 0: \$10 \$11 \$12 \$13/);
    assert.match(formatted, /SP 3: \$1C \$1D \$1E \$1F/);
});

test('formatAttributeGrid returns empty string for invalid/short data', () => {
    const context = vm.createContext({window: {}});
    vm.runInContext(fs.readFileSync('static/js/ppu_viewer.js', 'utf8') + '\nthis.Viewer = PpuViewer;', context);

    assert.equal(context.Viewer.formatAttributeGrid(null), '');
    assert.equal(context.Viewer.formatAttributeGrid(new Uint8Array(16)), '');
});
