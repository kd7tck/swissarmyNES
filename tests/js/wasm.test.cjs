const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');

let bindingPromise;
function browserBinding() {
    // Initialize the exact generated browser artifact once for all equivalence cases.
    bindingPromise ??= (async () => {
        const source = fs.readFileSync('static/wasm/swiss_emulator.js');
        const binding = await import(`data:text/javascript;base64,${source.toString('base64')}`);
        binding.initSync({module: fs.readFileSync('static/wasm/swiss_emulator_bg.wasm')});
        return binding;
    })();
    return bindingPromise;
}

function nrom(code, prgDeclaration = 0) {
    const image = new Uint8Array(16 + 16384 + 8192);
    image.set([0x4e, 0x45, 0x53, 0x1a, 1, 1, prgDeclaration & 0xf0 ? 2 : 0, 8]);
    image[10] = prgDeclaration;
    image.set(code, 16);
    image[16 + 0x3ffd] = 0x80;
    return image;
}

test('generated browser WASM matches every canonical CPU trace record', async () => {
    // Load the exact browser binding without changing the project's module type.
    const binding = await browserBinding();
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
                hex('P:'), hex('SP:'), BigInt(line.split('CYC:')[1].trim())];
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

test('browser WASM agrees on decimal-disabled ALU, exact cycles, and JAM errors', async () => {
    const binding = await browserBinding();
    const emulator = new binding.Emulator();
    try {
        for (const [opcode, a, b, carry, expected] of [
            [0x69, 0x99, 1, 0, 0x9a], [0x69, 0xff, 0, 1, 0],
            [0x69, 0x7f, 1, 0, 0x80], [0xe9, 0, 1, 1, 0xff],
            [0xe9, 0x80, 1, 1, 0x7f], [0xe9, 0x10, 1, 1, 0x0f],
        ]) {
            emulator.load_rom(nrom([0xf8, carry ? 0x38 : 0x18, 0xa9, a, opcode, b]));
            for (let i=0; i<4; i++) emulator.trace_instruction().free();
            const state = emulator.get_cpu_state();
            try {
                assert.equal(state.acc, expected);
                assert.equal(state.status & 8, 8, 'D is retained but arithmetic remains binary');
                assert.equal(state.cycles, 15n, 'u64 cycle getter must use exact BigInt');
            } finally { state.free(); }
        }
        for (const opcode of [0x02,0x12,0x22,0x32,0x42,0x52,0x62,0x72,0x92,0xb2,0xd2,0xf2]) {
            emulator.load_rom(nrom([opcode]));
            assert.throws(() => emulator.trace_instruction());
            assert.throws(() => emulator.step());
        }
        emulator.load_rom(nrom([0xea]));
        emulator.reset();
        const state = emulator.trace_instruction();
        try { assert.equal(state.pc, 0x8000); } finally { state.free(); }
    } finally { emulator.free(); }
});

test('browser battery data excludes volatile RAM and survives reset and invalid replacement', async () => {
    const binding = await browserBinding();
    const emulator = new binding.Emulator();
    try {
        const rom = nrom([0xa9,0x11,0x8d,0,0x60,0xa9,0xa5,0x8d,0,0x70], 0x66);
        emulator.load_rom(rom);
        for (let i=0; i<4; i++) emulator.trace_instruction().free();
        const saved = emulator.save_ram_snapshot();
        assert.equal(saved.length, 4096);
        assert.equal(saved[0], 0xa5);
        emulator.reset();
        assert.deepEqual(emulator.save_ram_snapshot(), saved);
        assert.throws(() => emulator.load_rom(new Uint8Array([1,2,3])));
        assert.deepEqual(emulator.save_ram_snapshot(), saved);
        assert.throws(() => emulator.restore_save_ram(saved.slice(1)));
        assert.deepEqual(emulator.save_ram_snapshot(), saved);
        const replacement = saved.slice();
        replacement[0] = 0x42;
        emulator.restore_save_ram(replacement);
        assert.equal(emulator.peek_cpu(0x7000), 0x42);
        emulator.load_rom(nrom([0xea]));
        assert.equal(emulator.save_ram_snapshot().length, 0);
    } finally { emulator.free(); }
});
