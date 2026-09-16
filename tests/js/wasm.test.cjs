const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');

test('generated browser WASM matches every canonical CPU trace record', async () => {
    // Load the exact browser binding without changing the project's module type.
    const source = fs.readFileSync('static/wasm/swiss_emulator.js');
    const binding = await import(`data:text/javascript;base64,${source.toString('base64')}`);
    binding.initSync({module: fs.readFileSync('static/wasm/swiss_emulator_bg.wasm')});
    const rom = Uint8Array.from(fs.readFileSync('emulator/tests/fixtures/nestest.nes'));
    rom[16 + 0x3ffc] = 0;
    rom[16 + 0x3ffd] = 0xc0;
    const emulator = new binding.Emulator();
    try {
        emulator.load_rom(rom);
        const lines = fs.readFileSync('emulator/tests/fixtures/nestest.trace', 'utf8').trim().split(/\r?\n/);
        assert.equal(lines.length, 8991);
        for (const [index, line] of lines.entries()) {
            const hex = tag => parseInt(line.split(tag)[1].slice(0, 2), 16);
            const expected = [parseInt(line.slice(0, 4), 16), hex('A:'), hex('X:'), hex('Y:'),
                hex('P:'), hex('SP:'), Number(line.split('CYC:')[1])];
            const state = emulator.trace_instruction();
            try {
                assert.deepEqual([state.pc, state.acc, state.x, state.y, state.status, state.sp, state.cycles],
                    expected, `record ${index + 1}`);
            } finally { state.free(); }
        }
        emulator.update_palettes();
        assert.equal(emulator.get_palettes_len(), 128);
        assert.ok(Math.abs(emulator.frame_rate() - 60.0988) < 0.002);
        // Verify the generated bundle resolves the same corrected cartridge
        // loader as native, including absence and nonvolatile-only declarations.
        for (const [declaration, capacity] of [[0, 0], [1, 128], [0x70, 8192]]) {
            const image = new Uint8Array(16 + 16384 + 8192);
            image.set([0x4e, 0x45, 0x53, 0x1a, 1, 1, declaration & 0xf0 ? 2 : 0, 8]);
            image[10] = declaration;
            image[16 + 0x3ffd] = 0x80;
            emulator.load_rom(image);
            assert.equal(emulator.prg_ram_len(), capacity);
        }
    } finally { emulator.free(); }
});
