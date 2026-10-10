const {test} = require('node:test');
const assert = require('node:assert/strict');
const {domFixture} = require('./dom_fixture.cjs');

test('128x128 PNG import preserves bitplanes, tile corners and transparent pixels', () => {
    const rgba = new Uint8ClampedArray(128 * 128 * 4);
    const pixel = (x, y, color, alpha = 255) => rgba.set([color, color, color, alpha], (y * 128 + x) * 4);
    pixel(0, 0, 85); pixel(7, 0, 170); pixel(8, 0, 255); pixel(127, 127, 255);
    pixel(1, 0, 255, 0);
    let imageSize = 128;
    const f = domFixture([], {
        FileReader: class { readAsDataURL() { this.onload({target: {result: 'image'}}); } },
        Image: class { set src(_) { this.width = imageSize; this.height = imageSize; this.onload(); } }
    });
    const create = f.document.createElement;
    f.document.createElement = tag => {
        const element = create(tag);
        if (tag === 'canvas') element.context.getImageData = () => ({data: rgba});
        return element;
    };
    f.load('static/js/chr.js', ['CHREditor']);
    const editor = Object.create(f.context.CHREditor.prototype);
    Object.assign(editor, {assets: {chr_bank: Array(4096).fill(0)}, renderPalette: ['#000000', '#555555', '#aaaaaa', '#ffffff'], render() {}, renderBank() {}});
    editor.processImportedImage({type: 'image/png'});
    assert.equal(editor.assets.chr_bank[0], 0x80, 'first pixel sets low plane, transparency stays zero');
    assert.equal(editor.assets.chr_bank[8], 0x01, 'top right pixel sets high plane');
    assert.equal(editor.assets.chr_bank[16], 0x80, 'next tile begins at its own bank offset');
    assert.equal(editor.assets.chr_bank[24], 0x80);
    assert.equal(editor.assets.chr_bank[255 * 16 + 7], 1, 'bottom right bank corner');
    assert.equal(editor.assets.chr_bank[255 * 16 + 15], 1);
    const before = [...editor.assets.chr_bank];
    imageSize = 64;
    editor.processImportedImage({type: 'image/png'});
    assert.deepEqual([...editor.assets.chr_bank], before);
    assert.match(f.alerts.at(-1), /128x128/);
});

test('CHR shifts publish one complete bank change and provide an unscaled picker canvas', () => {
    const f = domFixture();
    f.load('static/js/chr.js', ['CHREditor']);
    const editor = Object.create(f.context.CHREditor.prototype);
    let changes = 0, renders = 0;
    f.window.addEventListener('chr-changed', () => { changes++; });
    Object.assign(editor, {assets: {chr_bank: Array(4096).fill(0)}, currentTileIndex: 0, render() { renders++; }, renderBank() {}, bankCanvas: f.document.createElement('canvas')});
    editor.updatePixel(0, 0, 3);
    changes = 0; renders = 0;
    editor.shiftTile(1, 0);
    assert.equal(editor.getPixel(0, 0), 0); assert.equal(editor.getPixel(1, 0), 3);
    assert.equal(changes, 1); assert.equal(renders, 1);
    const canvas = editor.getCanvas();
    assert.equal(canvas.width, 128); assert.equal(canvas.height, 128);
});

test('metatile propagation keeps corner order/attribute neighbors and detached tile edits', () => {
    const f = domFixture();
    f.load('static/js/map.js', ['MapEditor']);
    const editor = Object.create(f.context.MapEditor.prototype);
    const nt = {data: Array(960).fill(0), attrs: Array(64).fill(0xAA), metatile_grid: Array(240).fill(-1)};
    const meta = {tiles: [1, 2, 3, 4], attr: 1};
    Object.assign(editor, {assets: {nametables: [nt], metatiles: [meta]}, currentNametableIndex: 0, render() {}});
    f.window.metatileEditor = {currentMetatileIndex: 0};
    f.window.chrEditor = {currentTileIndex: 9, setTile(value) { this.currentTileIndex = value; }};
    editor.placeMetatile(2, 2);
    assert.deepEqual([nt.data[66], nt.data[67], nt.data[98], nt.data[99]], [1, 2, 3, 4]);
    assert.equal(nt.attrs[0], 0x6A, 'only bottom-right quadrant changes');
    editor.placeTile(2, 2);
    assert.equal(nt.metatile_grid[17], -1);
    meta.tiles = [5, 6, 7, 8];
    editor.updateMetatileInstances(0);
    assert.equal(nt.data[66], 9, 'manual edit stays detached from the metatile');
    editor.mode = 'tile';
    editor.mouseX = 16; editor.mouseY = 16;
    editor.handlePick();
    assert.equal(f.window.chrEditor.currentTileIndex, 9);
});

test('new nametables become paintable in the world editor without reloading', () => {
    const f = domFixture(['world-editor-root']);
    f.load('static/js/world.js', ['WorldEditor']);
    f.load('static/js/map.js', ['MapEditor']);
    const assets = {nametables: [], world: {width: 2, height: 1, data: [-1, -1]}};
    const world = new f.context.WorldEditor(); world.onProjectLoaded(assets);
    const map = Object.create(f.context.MapEditor.prototype);
    Object.assign(map, {assets, render() {}});
    map.addNametable();
    assert.equal(world.mapList.children.length, 2, 'eraser plus new map');
    world.mapList.children[1].onclick();
    world.handleMouse({clientX: 65, clientY: 1});
    assert.deepEqual([...world.getData().data], [-1, 0]);
});

test('map selection, replacement, deletion and world resizing preserve identities/adjacency', () => {
    const f = domFixture(['world-editor-root', 'map-editor-root']);
    f.load('static/js/world.js', ['WorldEditor']); f.load('static/js/map.js', ['MapEditor']);
    const maps = Array.from({length: 4}, (_, index) => ({name: `map${index}`, data: Array(960).fill(index), attrs: Array(64).fill(index), metatile_grid: Array(240).fill(-1)}));
    const assets = {chr_bank: [], palettes: [], nametables: maps, world: {width: 2, height: 2, data: [0, 1, 2, 3]}};
    const world = new f.context.WorldEditor(); world.onProjectLoaded(assets);
    const map = new f.context.MapEditor(); map.onProjectLoaded(assets);
    assert.equal(map.mapSelect.children.length, 4);
    map.mapSelect.onchange({target: {value: '1'}});
    assert.equal(map.currentNametableIndex, 1);
    world.selectedNametableIndex = 3;
    world.handleMouse({clientX: 1, clientY: 1});
    assert.deepEqual([...world.world.data], [3, 1, 2, 3], 'map replacement changes only selected cell');
    map.deleteNametable();
    assert.deepEqual([...world.world.data], [2, -1, 1, 2]);
    assert.equal(world.selectedNametableIndex, 2);
    assert.equal(world.resizeWorld(3, 2), true);
    assert.deepEqual([...world.world.data], [2, -1, -1, 1, 2, -1]);
    const before = JSON.stringify(world.getData());
    assert.equal(world.resizeWorld(255, 255), false);
    assert.equal(JSON.stringify(world.getData()), before);
    assert.equal(world.resizeWorld(1, 2), true);
    assert.deepEqual([...world.world.data], [2, 1]);
});

test('new metatile names can be used as source identifiers and invalid rename keeps identity', () => {
    const f = domFixture(); f.load('static/js/metatile.js', ['MetatileEditor']);
    const editor = Object.create(f.context.MetatileEditor.prototype);
    Object.assign(editor, {assets: {metatiles: []}, currentMetatileIndex: -1, renderList() {}, selectMetatile(index) { this.currentMetatileIndex = index; }});
    editor.addMetatile();
    assert.equal(editor.assets.metatiles[0].name, 'Meta0');
    editor.renameMetatile('invalid name');
    assert.equal(editor.assets.metatiles[0].name, 'Meta0');
    assert.match(f.alerts.at(-1), /identifiers/);
});

test('sprite preview masks palette bits and applies both hardware flips', () => {
    const f = domFixture();
    f.load('static/js/sprite.js', ['SpriteEditor']);
    const editor = Object.create(f.context.SpriteEditor.prototype);
    const bank = Array(4096).fill(0); bank[0] = 0x80;
    f.window.projectManager = {assets: {chr_bank: bank, palettes: Array.from({length: 8}, () => ({colors: [0, 1, 2, 3]}))}};
    f.window.paletteEditor = {nesPalette: ['000000', '123456', 'abcdef', 'ffffff']};
    const canvas = f.document.createElement('canvas');
    editor.drawTile(canvas.context, 0, 10, 20, 2, 0xE2);
    assert.deepEqual(canvas.draws, [{color: '#123456', args: [24, 34, 2, 2]}]);
});

test('renaming a metasprite keeps every animation frame reference usable', () => {
    const f = domFixture();
    f.load('static/js/sprite.js', ['SpriteEditor']);
    const editor = Object.create(f.context.SpriteEditor.prototype);
    Object.assign(editor, {currentMetasprite: {name: 'hero'}, animations: [{frames: [{metasprite: 'hero'}, {metasprite: 'other'}]}], renderMetaspriteList() {}});
    editor.renameMetasprite('player');
    assert.deepEqual(editor.animations[0].frames.map(frame => frame.metasprite), ['player', 'other']);
    editor.renameMetasprite('invalid name');
    assert.equal(editor.currentMetasprite.name, 'player');
    editor.deleteMetasprite();
    assert.match(f.alerts.at(-1), /animation frames/);
});

test('sprite animation preview uses 60Hz durations on 60/120/144Hz displays and one loop', () => {
    for (const hz of [60, 120, 144]) {
        const f = domFixture(['anim-preview-canvas']);
        f.load('static/js/sprite.js', ['SpriteEditor']);
        const editor = Object.create(f.context.SpriteEditor.prototype);
        const drawn = [];
        Object.assign(editor, {animationRequest: null, currentAnimation: {frames: [{metasprite: 'a', duration: 30}, {metasprite: 'b', duration: 30}], does_loop: false},
            metasprites: [{name: 'a', tiles: [{tile: 1, x: 0, y: 0}]}, {name: 'b', tiles: [{tile: 2, x: 0, y: 0}]}],
            drawTile(ctx, tile) { drawn.push(tile); }});
        editor.playAnimation(); editor.playAnimation();
        assert.equal(f.frames.size, 1);
        for (let frame = 0; frame <= hz; frame++) {
            const [id, callback] = f.frames.entries().next().value;
            f.frames.delete(id); callback(frame * 1000 / hz);
            if (frame === 0) assert.equal(drawn.at(-1), 1);
            if (frame === hz) assert.equal(drawn.at(-1), 2);
        }
        assert.equal(f.frames.size, 0, `${hz}Hz preview ends after one second`);
    }
});

test('SFX arpeggio drawing survives JSON export/reimport and malformed import is rejected', async () => {
    const f = domFixture(['sfx-seq-editor-container'], {Blob});
    let exported;
    f.context.URL = {createObjectURL(blob) { exported = blob; return 'blob:sfx'; }, revokeObjectURL() {}};
    f.document.body.removeChild = child => child.remove();
    f.load('static/js/sfx.js', ['SFXEditor']);
    const editor = Object.create(f.context.SFXEditor.prototype);
    Object.assign(editor, {sfxList: [], currentIndex: -1, currentTab: 'arpeggio', seqEditor: null, renderList() {}, updateEditor() {}});
    editor.importSFX({name: 'chord', channel: 0, vol_sequence: [15, 15, 15], arpeggio_sequence: [0, 4, 7]}, 'chord');
    editor.renderSequenceEditor();
    assert.equal(editor.seqEditor.label, 'Semitones from note 48');
    // Draw step1 at semitone4 in the actual sequence canvas.
    editor.seqEditor.startDrawing({clientX: 15, clientY: 150 - (4 + 48) / 95 * 150});
    editor.seqEditor.stopDrawing();
    editor.exportSFX();
    const json = JSON.parse(await exported.text());
    assert.deepEqual(json.arpeggio_sequence, [0, 4, 7]);
    editor.importSFX(json, 'reload');
    assert.deepEqual([...editor.getData()[1].arpeggio_sequence], [0, 4, 7]);
    const count = editor.sfxList.length;
    editor.importSFX(null, 'invalid'); editor.importSFX({arpeggio_sequence: [128]}, 'invalid');
    editor.importSFX({channel: 3}, 'unsupported');
    assert.equal(editor.sfxList.length, count);
    assert.equal(f.alerts.length, 3);
});

test('SFX preview follows noncumulative timer offsets and semitone0/4/7 sequence', () => {
    const f = domFixture([], {setTimeout() { return 1; }, clearTimeout() {}});
    f.load('static/js/synth.js', ['SynthEngine']);
    const synth = Object.create(f.context.SynthEngine.prototype);
    const frequencies = [];
    Object.assign(synth, {isPlaying: true, ctx: {currentTime: 0}, stepIndex: 0, nextStepTime: 0, stepDuration: 1 / 60,
        sfx: {channel: 0, vol_sequence: [15], pitch_sequence: [5], arpeggio_sequence: [0, 4, 7], duty_sequence: [0], does_loop: true},
        gainNode: {gain: {setValueAtTime() {}}}, osc: {frequency: {setValueAtTime(value) { frequencies.push(value); }}, setPeriodicWave() {}}, pulseWaves: []});
    synth.scheduleNextStep();
    const expected = [0x6A, 0x54, 0x47].map(period => 1789773 / (16 * (period + 5 + 1)));
    assert.deepEqual(frequencies.slice(0, 3), expected);
    assert.equal(frequencies[3], frequencies[0], 'pitch offset does not accumulate over loops');
});

test('finite SFX preview holds shorter sequences and stops at the scheduled final step', () => {
    const f = domFixture([], {setTimeout() { return 1; }, clearTimeout() {}});
    f.load('static/js/synth.js', ['SynthEngine']);
    const synth = Object.create(f.context.SynthEngine.prototype);
    const volumes = [], stopTimes = [];
    let disconnected = 0;
    Object.assign(synth, {isPlaying: true, ctx: {currentTime: 0}, stepIndex: 0, nextStepTime: 0, stepDuration: 1 / 60,
        sfx: {channel: 0, vol_sequence: [12], pitch_sequence: [], arpeggio_sequence: [0, 4, 7], duty_sequence: [], does_loop: false},
        gainNode: {gain: {setValueAtTime(value) { volumes.push(value); }}, disconnect() { disconnected++; }},
        osc: {frequency: {setValueAtTime() {}}, setPeriodicWave() {}, stop(time) { stopTimes.push(time); }, disconnect() { disconnected++; }}, pulseWaves: []});
    synth.scheduleNextStep();
    assert.deepEqual(volumes, [0.8, 0.8, 0.8]);
    assert.equal(stopTimes.length, 1); assert.ok(Math.abs(stopTimes[0] - 3 / 60) < 1e-9);
    assert.equal(disconnected, 0, 'queued audio remains connected until its scheduled end');
    synth.osc.onended();
    assert.equal(disconnected, 2); assert.equal(synth.osc, null);
});
