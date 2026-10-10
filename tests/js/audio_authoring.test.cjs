const {test} = require('node:test');
const assert = require('node:assert/strict');
const {domFixture} = require('./dom_fixture.cjs');

function trackerFixture() {
    const f = domFixture(['audio-tracker-root', 'audio-instrument-select', 'sample-list']);
    f.load('static/js/audio.js', ['AudioTracker']);
    const tracker = Object.create(f.context.AudioTracker.prototype);
    Object.assign(tracker, {root: f.elements.get('audio-tracker-root'), currentTrackIndex: 0,
        rows: 24, cols: 32, noteNames: ['C','C#','D','D#','E','F','F#','G','G#','A','A#','B'],
        defaultDuration: 8, tracks: [{name: 'Pulse', channel: 0, instrument: 159, notes: []}], samples: [],
        envelopeEditor: {getEnvelopes() { return []; },loadEnvelopes() {}}});
    return {...f, tracker};
}

test('tracker notes are numeric JSON and retain column zero after project reload', () => {
    const f = trackerFixture();
    const cell = f.document.createElement('div'); cell.dataset.pitch = '48';
    f.tracker.toggleNote(11, 0, cell);
    const data = JSON.parse(JSON.stringify(f.tracker.getData()));
    assert.deepEqual(data.audio_tracks[0].notes[0], {row: 11, col: 0, pitch: 48, duration: 8});
    f.tracker.loadData(data);
    assert.equal(f.tracker.tracks[0].notes[0].col, 0);
    assert.equal(f.tracker.tracks[0].notes[0].pitch, 48);
});

test('DMC grid authors sample IDs and offers all sixteen hardware rates', () => {
    const f = trackerFixture();
    Object.assign(f.tracker, {tracks: [{name: 'DMC', channel: 3, instrument: 15, notes: []}],
        samples: [{name: 'kick', data: [1]}, {name: 'snare', data: [2]}]});
    f.tracker.loadTrackUI();
    const rows = f.tracker.root.children;
    assert.equal(rows[0].textContent, '0: kick');
    assert.equal(rows[33].textContent, '1: snare');
    rows[34].dispatchEvent(new f.Event('mousedown'));
    assert.equal(f.tracker.tracks[0].notes[0].pitch, 1);
    assert.equal(f.elements.get('audio-instrument-select').children.length, 16);
    assert.equal(f.elements.get('audio-instrument-select').value, 15);
});

test('removing a DPCM sample repairs remaining references without changing tonal notes', () => {
    const f = trackerFixture();
    f.tracker.samples = [{name:'one',data:[1]},{name:'two',data:[2]},{name:'three',data:[3]}];
    f.tracker.tracks = [
        {name:'DMC',channel:3,instrument:15,notes:[{pitch:0,col:0},{pitch:1,col:1},{pitch:2,col:2}]},
        {name:'Pulse',channel:0,instrument:159,notes:[{pitch:1,col:0}]}
    ];
    f.tracker.renderSampleList();
    f.elements.get('sample-list').children[1].children[0].click();
    assert.deepEqual(f.tracker.getData().audio_tracks[0].notes.map(note=>note.pitch), [0,1]);
    assert.equal(f.tracker.tracks[1].notes[0].pitch,1);
});

test('WAV resampling preserves DMC duration across source rates and rejects oversize or invalid data', () => {
    const f = trackerFixture();
    for (const sampleRate of [44100,48000]) {
        const samples = new Float32Array(sampleRate / 10);
        const encoded = f.tracker.encodeDPCM({sampleRate,getChannelData(){return samples;}},15);
        assert.equal(encoded.length, Math.ceil(Math.ceil(1789773 / 54 / 10) / 8));
    }
    assert.throws(()=>f.tracker.encodeDPCM({sampleRate:44100,getChannelData(){return new Float32Array(44100);}},15),/too long/);
    assert.throws(()=>f.tracker.encodeDPCM({sampleRate:0,getChannelData(){return new Float32Array(1);}},15),/valid audio/);
    assert.throws(()=>f.tracker.encodeDPCM({sampleRate:44100,getChannelData(){return Float32Array.from([NaN]);}},15),/invalid samples/);
});

test('music Play requests the selected track through the production emulator bootstrap', () => {
    const f = trackerFixture();
    const button = f.document.createElement('button'); button.id = 'btn-play-track'; f.document.body.appendChild(button);
    const played = [];
    f.window.editor = {runEmulator(options){played.push(options);}};
    f.tracker.bindEvents();
    button.click();
    assert.equal(played.length,0);
    assert.equal(f.alerts.length,1);
    f.tracker.tracks[0].notes = [{pitch:48,row:11,col:0,duration:8}];
    button.click();
    assert.deepEqual(JSON.parse(JSON.stringify(played)),[{previewTrackId:0}]);
});

test('envelope deletion repairs track IDs and a reload clears prior editing state', () => {
    const f = trackerFixture();
    f.load('static/js/envelopes.js', ['EnvelopeEditor']);
    const editor = Object.create(f.context.EnvelopeEditor.prototype);
    Object.assign(editor,{envelopes:[{name:'one',steps:[[15,1]]},{name:'two',steps:[[10,1]]}],
        currentEnvelopeIndex:0,renderList(){},renderGraph(){}});
    f.tracker.envelopeEditor = editor;
    Object.assign(f.tracker.tracks[0], {vol_env:0,pitch_env:1,arpeggio_env:null});
    f.tracker.bindEvents();
    editor.deleteEnvelope();
    assert.equal(f.tracker.tracks[0].vol_env,null);
    assert.equal(f.tracker.tracks[0].pitch_env,0);
    editor.currentEnvelopeIndex=0;
    editor.loadEnvelopes([{name:'new project',steps:[[12,1]]}]);
    assert.equal(editor.currentEnvelopeIndex,-1);
});

test('preview compiles current audio through Sound.Play without replacing saved game source', async () => {
    const calls = [];
    const f = domFixture(['code-editor'], {fetch:async(url,options)=>{
        calls.push({url,body:JSON.parse(options.body)});
        return {ok:false,text:async()=> 'fixture compilation end'};
    }});
    f.elements.get('code-editor').value = 'SUB Main()\nEND SUB';
    f.window.projectManager = {currentProject:'game',saveCurrentProject:async()=>true};
    f.window.audioTracker = {tracks:[{notes:[{pitch:48}]}],getData(){return {audio_tracks:this.tracks,samples:[],envelopes:[]};}};
    f.load('static/js/app.js'); f.document.events.get('DOMContentLoaded')[0]();
    f.window.dispatchEvent(new f.Event('request-compile-and-run',{detail:{previewTrackId:0}}));
    await new Promise(resolve=>setImmediate(resolve));
    assert.equal(calls.length,1);
    assert.equal(calls[0].body.project_name,null);
    assert.match(calls[0].body.source,/Sound\.Play\(0\)/);
    assert.equal(calls[0].body.assets.audio_tracks[0].notes[0].pitch,48);
    assert.equal(f.elements.get('code-editor').value,'SUB Main()\nEND SUB');
});
