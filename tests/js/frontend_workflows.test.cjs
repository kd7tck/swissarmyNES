const {test} = require('node:test');
const assert = require('node:assert/strict');
const {domFixture, deferred} = require('./dom_fixture.cjs');

function editorFixture() {
    const f = domFixture(['code-editor', 'code-highlight', 'line-numbers']);
    f.load('static/js/ppu_viewer.js', ['PpuViewer']);
    f.load('static/js/editor.js', ['SwissEditor']);
    const editor = new f.context.SwissEditor('code-editor', 'code-highlight', 'line-numbers');
    const calls = [];
    Object.assign(editor, {emulator: {
        set_button: (...args) => calls.push(args), clear_audio_samples() {}, reset() {},
        get_pixels() { return 0; }, get_pixels_len() { return 256 * 240 * 4; },
        frame_rate() { return 60; }, step() { return false; },
        get_audio_samples() { return 0; }, get_audio_samples_len() { return 0; }
    }, emulatorRunning: true});
    return {...f, editor, calls};
}

test('overlay reopening keeps one keyboard handler and both Shift keys hold Select', () => {
    const f = editorFixture();
    for (let i = 0; i < 3; i++) f.editor.createEmulatorOverlay();
    assert.equal(f.document.events.get('keydown').length, 1);
    const key = (code, pressed) => f.document.dispatchEvent(new f.Event(pressed ? 'keydown' : 'keyup', {code}));
    key('ShiftLeft', true); key('ShiftRight', true); key('ShiftLeft', false);
    assert.equal(f.editor.lastSentState[2], true);
    key('ShiftRight', false);
    assert.deepEqual(f.calls, [[0, 2, true], [0, 2, false]]);
    f.editor.releaseInputs();
    assert.equal(f.editor.pressedKeys.size, 0);
});

test('keyboard/gamepad mixing preserves holds and disconnected pads release state', () => {
    const f = editorFixture();
    f.editor.createEmulatorOverlay();
    const pad = {connected: true, buttons: [{pressed: true}], axes: []};
    f.context.navigator.getGamepads = () => [pad];
    f.editor.pollGamepads();
    f.editor.handleEmulatorInput(new f.Event('keydown', {code: 'KeyZ'}), true);
    pad.connected = false;
    f.editor.pollGamepads();
    assert.equal(f.editor.lastSentState[0], true);
    f.editor.handleEmulatorInput(new f.Event('keyup', {code: 'KeyZ'}), false);
    assert.deepEqual(f.calls, [[0, 0, true], [0, 0, false]]);
    f.context.navigator.getGamepads = undefined;
    f.editor.pollGamepads();
    assert.ok(f.editor.gamepadState.every(value => value === false));
});

test('pause and reset discard scheduled audio, release inputs and refresh paused pixels', () => {
    const f = editorFixture();
    let stopped = 0, disconnected = 0, reset = 0, cleared = 0;
    f.editor.audioSources.add({source: {stop() { stopped++; }, disconnect() { disconnected++; }}, gain: {disconnect() { disconnected++; }}});
    f.editor.audioContext = {state: 'running', currentTime: 12, suspend() { this.state = 'suspended'; }};
    f.editor.nextStartTime = 100;
    f.editor.emulator.reset = () => { reset++; };
    f.editor.emulator.clear_audio_samples = () => { cleared++; };
    f.editor.keyboardState[0] = true; f.editor.syncInputs();
    f.editor.togglePause();
    assert.equal(stopped, 1); assert.equal(disconnected, 2);
    assert.equal(f.editor.nextStartTime, 12); assert.equal(f.editor.audioSources.size, 0);
    assert.equal(f.editor.lastSentState[0], false);
    f.editor.createEmulatorOverlay();
    f.editor.wasmMemory = {buffer: new ArrayBuffer(256 * 240 * 4)};
    new Uint8Array(f.editor.wasmMemory.buffer)[0] = 123;
    f.editor.resetEmulator();
    assert.equal(reset, 1); assert.equal(cleared, 2);
    assert.equal(f.editor.canvas.lastImage.data[0], 123);
    assert.equal(f.frames.size, 0, 'paused reset does not resume execution');
});

test('CPU snapshots use mapper identity and free WASM snapshots on mapping failure', () => {
    const f = editorFixture();
    let freed = 0;
    f.editor.emulator.get_cpu_state = () => ({pc: 0xC123, cycles: 17n, free() { freed++; }});
    f.editor.emulator.prg_offset = () => 7 * 16384 + 0x123;
    assert.equal(f.editor.getDebugState().bank, 7);
    f.editor.emulator.prg_offset = () => -1;
    assert.equal(f.editor.getDebugState().bank, null);
    f.editor.emulator.prg_offset = () => { throw new Error('mapping failure'); };
    assert.equal(f.editor.getDebugState(), null);
    assert.equal(freed, 3);
});

test('suspended audio samples are drained instead of replayed after resume', () => {
    const f = editorFixture();
    let cleared = 0;
    f.editor.audioContext = {state: 'suspended'};
    f.editor.emulator.get_audio_samples_len = () => 800;
    f.editor.emulator.clear_audio_samples = () => { cleared++; };
    f.editor.emulatorLoop(0); f.editor.emulatorLoop(20);
    assert.equal(cleared, 1);
});

test('source maps retain included-file bank identity and exclude edited snapshots', () => {
    const f = editorFixture();
    const added = [];
    f.editor.emulator.clear_breakpoints = () => { added.length = 0; };
    f.editor.emulator.add_breakpoint = (bank, pc) => added.push([bank, pc]);
    f.window.projectManager = {currentProject: 'game', currentFile: 'main.swiss'};
    f.editor.compiledProject = 'game';
    f.editor.sourceMap = {version: 1, sources: {'main.swiss': 'main', 'included.swiss': 'included'}, entries: [
        {file: 'main.swiss', line: 1, kind: 'instruction', bank: 0, cpu_start: 0x8000, cpu_end: 0x8002},
        {file: 'included.swiss', line: 1, kind: 'instruction', bank: 2, cpu_start: 0x8000, cpu_end: 0x8002}
    ]};
    f.editor.editor.value = 'main'; f.editor.update(); f.editor.toggleBreakpoint(1);
    f.window.projectManager.currentFile = 'included.swiss';
    f.editor.editor.value = 'included'; f.editor.update(); f.editor.toggleBreakpoint(1);
    assert.deepEqual(added, [[0, 0x8000], [2, 0x8000]]);
    f.editor.showAddressTooltip(1, 0, 0);
    assert.equal(f.editor.tooltip.innerText, 'Bank 2: $8000');
    f.editor.editor.value = 'edited'; f.editor.editor.dispatchEvent(new f.Event('input'));
    assert.deepEqual(added, [[0, 0x8000]], 'unmodified main stays armed');
    f.editor.showAddressTooltip(1, 0, 0);
    assert.equal(f.editor.tooltip.innerText, 'No executable address');
});

test('RAM viewer bounds visible work and reaches the final internal RAM byte after growth', () => {
    const f = editorFixture();
    const content = f.document.createElement('div'); content.id = 'mem-content'; f.document.body.appendChild(content);
    f.editor.createEmulatorOverlay();
    const viewer = f.elements.get('memory-viewer'); viewer.clientHeight = 180; viewer.scrollTop = 0;
    f.editor.memoryViewerOpen = true;
    f.editor.wasmMemory = {buffer: new ArrayBuffer(2048)};
    f.editor.emulator.get_wram = () => 0; f.editor.emulator.get_wram_len = () => 2048;
    new Uint8Array(f.editor.wasmMemory.buffer)[0] = 0x11;
    f.editor.updateMemoryView();
    assert.match(content.innerHTML, />0000</); assert.match(content.innerHTML, />11</);
    assert.ok((content.innerHTML.match(/color:#fe9/g) || []).length <= 14);
    f.editor.wasmMemory.buffer = new ArrayBuffer(4096);
    new Uint8Array(f.editor.wasmMemory.buffer)[2047] = 0xAB;
    viewer.scrollTop = 100000;
    f.editor.updateMemoryView();
    assert.match(content.innerHTML, />07F0</); assert.match(content.innerHTML, />AB</);
    assert.doesNotMatch(content.innerHTML, />0000</);
});

function projectFixture(fetch) {
    const f = domFixture(['code-editor', 'file-list', 'current-project-name', 'file-explorer'], {fetch});
    f.load('static/js/project.js', ['ProjectManager']);
    const manager = Object.create(f.context.ProjectManager.prototype);
    Object.assign(manager, {currentProject: 'old', currentFile: 'main.swiss', assets: {chr_bank: [1]},
        fileList: f.elements.get('file-list'), loadRevision: 0, fileLoadRevision: 0});
    f.window.projectManager = manager;
    return {...f, manager};
}

test('project save failure aborts compile before issuing the compile API request', async () => {
    const calls = [];
    const f = projectFixture(async (url) => { calls.push(url); return {ok: false}; });
    f.load('static/js/app.js');
    // ProjectManager is already mounted by the fixture; initialize only app.js.
    f.document.events.get('DOMContentLoaded').at(-1)();
    f.window.dispatchEvent(new f.Event('request-compile-and-run'));
    await new Promise(resolve => setImmediate(resolve));
    assert.deepEqual(calls, ['/api/projects/old/files/main.swiss']);
    assert.ok(f.alerts.some(text => text.includes('Error saving file')));
});

test('saving snapshots source/assets before a project switch during network IO', async () => {
    const first = deferred(), calls = [];
    const f = projectFixture(async (url, options) => {
        calls.push({url, body: JSON.parse(options.body)});
        return calls.length === 1 ? first.promise : {ok: true};
    });
    f.elements.get('code-editor').value = 'old edited source';
    f.window.sfxEditor = {getData() { return [{name: 'old sound'}]; }};
    const save = f.manager.saveCurrentProject(true);
    f.manager.currentProject = 'new'; f.manager.currentFile = 'other.swiss';
    f.manager.assets = {chr_bank: [2]};
    f.window.sfxEditor = {getData() { return [{name: 'new sound'}]; }};
    first.resolve({ok: true});
    assert.equal(await save, true);
    assert.equal(calls[1].url, '/api/projects/old');
    assert.deepEqual(calls[1].body.assets, {chr_bank: [1], sound_effects: [{name: 'old sound'}]});
    assert.equal(calls[0].body.content, 'old edited source');
});

test('overlapping saves retain request order and project compilation uses persisted assets', async () => {
    const first = deferred(), calls = [];
    const f = projectFixture(async (url, options) => {
        calls.push({url, body: JSON.parse(options.body)});
        if (url === '/api/compile') return {ok: false, text: async () => 'expected fixture end'};
        return calls.length === 1 ? first.promise : {ok: true};
    });
    f.elements.get('code-editor').value = 'first';
    const save1 = f.manager.saveCurrentProject(true);
    f.elements.get('code-editor').value = 'second';
    const save2 = f.manager.saveCurrentProject(true);
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(calls.length, 1, 'second save waits for the first source/assets pair');
    first.resolve({ok: true});
    assert.equal(await save1, true); assert.equal(await save2, true);
    assert.deepEqual(calls.filter(call => call.url.includes('/files/')).map(call => call.body.content), ['first', 'second']);
    f.load('static/js/app.js'); f.document.events.get('DOMContentLoaded').at(-1)();
    f.window.dispatchEvent(new f.Event('request-compile-and-run'));
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(calls.at(-1).url, '/api/compile');
    assert.equal(calls.at(-1).body.assets, null, 'backend reads just-saved editor component snapshots');
});

test('late project load cannot overwrite the newer project or mix its source/assets', async () => {
    const old = deferred();
    const f = projectFixture(async url => {
        if (url === '/api/projects/slow') return old.promise;
        if (url.endsWith('/files/main.swiss')) return {ok: true, text: async () => url.includes('/fast/') ? 'fast source' : 'slow source'};
        if (url.endsWith('/files')) return {ok: true, json: async () => ['main.swiss']};
        return {ok: true, json: async () => ({assets: {marker: 'fast'}})};
    });
    const slowLoad = f.manager.loadProject('slow');
    assert.equal(f.manager.currentProject, 'old');
    assert.equal(await f.manager.loadProject('fast'), true);
    old.resolve({ok: true, json: async () => ({assets: {marker: 'slow'}})});
    assert.equal(await slowLoad, false);
    assert.equal(f.manager.currentProject, 'fast');
    assert.equal(f.manager.assets.marker, 'fast');
    assert.equal(f.elements.get('code-editor').value, 'fast source');
});

test('switching to an included file saves the previous edited source first', async () => {
    const calls = [];
    const f = projectFixture(async (url, options) => {
        calls.push({url, content: options ? JSON.parse(options.body).content : null});
        return {ok: true, text: async () => 'included source'};
    });
    f.elements.get('code-editor').value = 'edited main';
    f.manager.renderFileList(['main.swiss', 'included.swiss']);
    assert.equal(await f.manager.loadFile('included.swiss'), true);
    assert.deepEqual(calls.map(call => call.url), ['/api/projects/old/files/main.swiss', '/api/projects/old', '/api/projects/old/files/included.swiss']);
    assert.equal(calls[0].content, 'edited main');
    assert.equal(f.manager.currentFile, 'included.swiss');
    assert.equal(f.elements.get('code-editor').value, 'included source');
});

test('an edit during autosave prevents compilation of stale persisted source', async () => {
    const save = deferred();
    const calls = [];
    const f = domFixture(['code-editor'], {fetch: async url => { calls.push(url); return {ok: false, text: async () => 'unexpected'}; }});
    f.window.projectManager = {currentProject: 'game', assets: {}, saveCurrentProject: () => save.promise};
    f.load('static/js/app.js'); f.document.events.get('DOMContentLoaded')[0]();
    f.window.dispatchEvent(new f.Event('request-compile-and-run'));
    f.elements.get('code-editor').value = 'changed'; f.elements.get('code-editor').dispatchEvent(new f.Event('input'));
    save.resolve(true);
    await new Promise(resolve => setImmediate(resolve));
    assert.deepEqual(calls, []);
});

test('a stale compile response cannot load a ROM or show an obsolete error', async () => {
    const response = deferred();
    const f = domFixture(['code-editor'], {fetch: () => response.promise});
    let loaded = 0;
    f.window.addEventListener('emulator-load-rom', () => { loaded++; });
    f.load('static/js/app.js'); f.document.events.get('DOMContentLoaded')[0]();
    f.window.dispatchEvent(new f.Event('request-compile-and-run'));
    f.elements.get('code-editor').dispatchEvent(new f.Event('input'));
    response.resolve({ok: false, text: async () => 'old error'});
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(loaded, 0); assert.deepEqual(f.alerts, []);
});
