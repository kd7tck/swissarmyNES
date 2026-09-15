const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

function fixture() {
    const callbacks = new Map();
    let next = 0;
    const context = vm.createContext({console, document: {addEventListener() {}},
        requestAnimationFrame(callback) { callbacks.set(++next, callback); return next; },
        cancelAnimationFrame(id) { callbacks.delete(id); }});
    vm.runInContext(fs.readFileSync('static/js/editor.js', 'utf8') + '\nthis.Editor = SwissEditor;', context);
    const editor = Object.create(context.Editor.prototype);
    let steps = 0;
    Object.assign(editor, {emulatorRunning: true, frameRequest: null, lastFrameTime: null, frameDebt: 0,
        frameCount: 0, memoryViewerOpen: false, ppuViewer: {isVisible: false}, wasmMemory: null,
        emulator: {frame_rate() {return 60.0988;}, step() {steps++; return false;}, get_pixels() {return 0;}, get_pixels_len() {return 0;},
            get_audio_samples() {return 0;}, get_audio_samples_len() {return 0;}},
        pollGamepads() {}, getDebugState() {return null;} });
    return {editor, callbacks, steps: () => steps};
}

test('display refresh does not set emulation speed', () => {
    for (const hz of [60, 120, 144]) {
        const f = fixture();
        f.editor.scheduleFrame();
        for (let i = 0; i <= hz; i++) {
            assert.equal(f.callbacks.size, 1);
            const [id, callback] = f.callbacks.entries().next().value;
            f.callbacks.delete(id);
            callback(i * 1000 / hz);
        }
        assert.equal(f.steps(), 60, `${hz} Hz display`);
        f.editor.cancelFrame();
        assert.equal(f.callbacks.size, 0);
    }
});

test('duplicate schedules and stalled frames stay bounded', () => {
    const f = fixture();
    f.editor.scheduleFrame();
    f.editor.scheduleFrame();
    assert.equal(f.callbacks.size, 1);
    f.editor.emulatorLoop(0);
    f.editor.emulatorLoop(100000);
    assert.ok(f.steps() <= 4);
    f.editor.cancelFrame();
    assert.equal(f.callbacks.size, 0);
});

test('PAL and Dendy cadence follows the emulator instead of NTSC', () => {
    for (const hz of [60, 120, 144]) {
        const f = fixture();
        f.editor.emulator.frame_rate = () => 50.007;
        f.editor.scheduleFrame();
        for (let i = 0; i <= hz; i++) {
            const [id, callback] = f.callbacks.entries().next().value;
            f.callbacks.delete(id);
            callback(i * 1000 / hz);
        }
        assert.equal(f.steps(), 50, `${hz} Hz display`);
    }
});
