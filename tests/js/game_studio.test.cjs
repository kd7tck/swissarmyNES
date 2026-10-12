const {test} = require('node:test');
const assert = require('node:assert/strict');
const {domFixture,deferred} = require('./dom_fixture.cjs');
const StudioStarter = require('../../static/js/studio_starter.js');

function fixture(fetch = async () => ({ok:true,json:async()=>({})})) {
    const fields = ['create','enable','save','run','test','code','add-scene','remove-scene','add-transition',
        'title','brief','start','animation','scene-name','map','x','y','movement','music','sfx','status','workspace','project','graph','transitions','summary','new-name',
        'tests-add','tests-add-step','tests-remove','tests-run','tests-select','tests-name','tests-steps','tests-result',
        'entity-add','entity-remove','entity','entity-name','entity-animation','entity-behavior','place','place-type',
        ...['player','entity'].flatMap(owner=>['x','y','width','height'].map(key=>`${owner}-hitbox-${key}`)),
        'combat-enabled','combat-fields','combat-health','combat-immunity','combat-damage','combat-reach','combat-button','combat-death','combat-mode','combat-cooldown','enemy-fields','enemy-health','enemy-damage','enemy-motion','enemy-range',
        'spawner-fields','spawner-target','spawner-capacity','spawner-interval','spawner-delay','spawner-total',
        ...['player','entity'].flatMap(owner=>['palette','flip_x','flip_y','face_movement'].map(key=>`${owner}-${key}`)),
        'entity-color-variant','variant-color1','variant-color2','variant-color3',
        'instance','instance-x','instance-y','instance-target','instance-end','instance-remove','placement','budget','door-field','patrol-field',
        'variable-add','variables','events-add','events-select','event-name','event-scene','event-trigger','event-button','event-frames','event-once',
        'event-button-field','event-timer-field','event-remove','event-up','event-down','condition-add','action-add','event-conditions','event-actions'];
    const f = domFixture(['studio',...fields.map(id=>`studio-${id}`)],{fetch});
    const starter = StudioStarter.build();
    const calls = [];
    f.window.projectManager = {currentProject:'adventure',loadRevision:1,fileLoadRevision:1,assets:starter.assets,design:starter.design,
        saveCurrentProject:async()=>true,refreshFiles:async()=>{},refreshProjects:async()=>{},loadProject:async name=>calls.push(name)};
    f.load('static/js/studio_starter.js');
    f.load('static/js/studio_entities.js');
    f.load('static/js/studio_events.js');
    f.load('static/js/game_studio.js',['GameStudio']);
    const studio = new f.context.GameStudio(); f.window.gameStudio = studio;
    return {...f,studio,calls,starter,field:id=>f.elements.get(`studio-${id}`)};
}

function addSpawner(f) {
    const editor=f.studio.entityEditor;
    editor.addDefinition();f.field('entity-behavior').value='enemy';f.field('entity-behavior').onchange();const target=editor.definitionId;
    editor.addDefinition();f.field('entity-name').value='Enemy nest';f.field('entity-name').dispatchEvent(new f.Event('input'));
    f.field('entity-behavior').value='spawner';f.field('entity-behavior').onchange();const spawner=editor.definitionId;
    f.studio.selectedScene='meadow';f.studio.render();editor.place();
    return {editor,target,spawner,instance:editor.instanceId};
}

test('color variants share animation and CHR and change only an unused palette color',()=>{
    const f=fixture(),editor=f.studio.entityEditor;editor.addDefinition();
    f.window.projectManager.assets.metasprites[1].tiles[0].attr=1;f.window.projectManager.assets.metasprites[2].tiles[0].attr=2;
    f.field('entity-name').value='😀'.repeat(30);f.field('entity-name').dispatchEvent(new f.Event('input'));
    const original=editor.definition(),assets=f.studio.assets(),chr=JSON.stringify(assets.chr_bank),frames=JSON.stringify(assets.animations),metas=JSON.stringify(assets.metasprites);
    const old=assets.palettes.find(palette=>palette.name==='SP0').colors.slice();
    editor.colorVariant();const variant=editor.definition();assert.notEqual(variant.id,original.id);assert.equal(variant.animation,original.animation);assert.equal(variant.visual.palette,3);
    assert.match(f.field('status').textContent,/Shared HeroWalk/);
    f.field('variant-color2').value='41';f.field('variant-color2').onchange();
    assert.equal(f.studio.assets().palettes.find(palette=>palette.name==='SP3').colors[2],41);
    assert.deepEqual(f.studio.assets().palettes.find(palette=>palette.name==='SP0').colors,old);
    assert.equal(JSON.stringify(f.studio.assets().chr_bank),chr);assert.equal(JSON.stringify(f.studio.assets().animations),frames);assert.equal(JSON.stringify(f.studio.assets().metasprites),metas);
    assert.ok(Buffer.byteLength(variant.name)<=120);
});

test('fully occupied sprite palettes cannot be overwritten by automatic color variant creation',()=>{
    const f=fixture();f.window.projectManager.assets.metasprites[0].tiles[0].attr=3;
    f.window.projectManager.assets.metasprites[1].tiles[0].attr=1;f.window.projectManager.assets.metasprites[2].tiles[0].attr=2;
    const before=JSON.stringify(f.studio.assets().palettes),count=f.studio.design.entities.length;
    f.studio.entityEditor.colorVariant();assert.equal(f.studio.design.entities.length,count);assert.equal(JSON.stringify(f.studio.assets().palettes),before);
    assert.match(f.field('status').textContent,/All four sprite palettes/);
});

test('palette variant edits cannot mutate a different project during switching',()=>{
    const f=fixture();f.field('entity-palette').value='0';f.field('entity-palette').onchange();
    const palettes=JSON.stringify(f.window.projectManager.assets.palettes),count=f.studio.design.entities.length;
    f.window.projectManager.currentProject='different';f.field('variant-color2').value='41';f.field('variant-color2').onchange();f.studio.entityEditor.colorVariant();
    assert.equal(JSON.stringify(f.window.projectManager.assets.palettes),palettes);assert.equal(f.studio.design.entities.length,count);
});

test('visual facing and mirror controls snapshot without copying frames and allocate only player facing state',()=>{
    const f=fixture();f.studio.selectedScene='meadow';f.studio.render();
    f.field('entity-palette').value='2';f.field('entity-palette').onchange();
    for(const id of ['entity-flip_x','entity-flip_y','entity-face_movement','player-face_movement']) {f.field(id).checked=true;f.field(id).onchange();}
    const data=f.studio.getData();assert.equal(data.entities[0].visual.palette,2);assert.equal(data.entities[0].visual.flip_x,true);assert.equal(data.entities[0].visual.flip_y,true);
    assert.equal(data.player_visual.face_movement,true);assert.match(f.field('budget').textContent,/45\/560 bytes/);
    assert.equal(f.studio.assets().animations.length,3);assert.equal(f.studio.assets().metasprites.length,4);
    f.studio.load('adventure',StudioStarter.build().design);assert.equal(f.field('entity-flip_x').checked,false);assert.equal(f.field('player-face_movement').checked,false);
});

test('spawner settings capture immediately and budgets reserve empty pool capacity',()=>{
    const f=fixture(),{editor,spawner}=addSpawner(f);
    assert.equal(f.field('spawner-fields').hidden,false);assert.equal(f.field('entity-animation').disabled,true);
    assert.match(f.field('budget').textContent,/6\/16 runtime slots.*24\/64 sprites.*90\/560 bytes/);
    for(const [id,value] of [['spawner-capacity',3],['spawner-interval',45],['spawner-delay',12],['spawner-total',9]]) {
        f.field(id).value=String(value);f.field(id).dispatchEvent(new f.Event('input'));
    }
    const data=f.studio.getData().entities.find(entity=>entity.id===spawner).spawner;
    assert.equal(data.capacity,3);assert.equal(data.interval_frames,45);assert.equal(data.initial_delay_frames,12);assert.equal(data.total,9);
    assert.match(f.field('budget').textContent,/7\/16 runtime slots.*28\/64 sprites.*100\/560 bytes/);
    assert.equal(editor.instance().patrol_end_x,null);
});

test('spawner enemy references protect deletion and retain changed missing targets for repair',()=>{
    const f=fixture(),{editor,target,spawner}=addSpawner(f);
    f.field('entity').value=target;f.field('entity').onchange();editor.removeDefinition();
    assert.ok(f.studio.design.entities.some(entity=>entity.id===target));assert.match(f.field('status').textContent,/spawner.*reference/);
    f.field('entity-hitbox-x').value='4';f.field('entity-hitbox-x').dispatchEvent(new f.Event('input'));
    f.field('entity-hitbox-width').value='8';f.field('entity-hitbox-width').dispatchEvent(new f.Event('input'));
    f.field('entity').value=spawner;f.field('entity').onchange();
    assert.equal(f.field('entity-hitbox-width').disabled,true);assert.equal(f.field('entity-hitbox-width').value,8);
    const marker=f.field('placement').children.at(-1);assert.equal(marker.style.left,`${52/256*100}%`);
    f.field('entity').value=target;f.field('entity').onchange();f.field('entity-behavior').value='decoration';f.field('entity-behavior').onchange();
    f.field('entity').value=spawner;f.field('entity').onchange();assert.match(f.field('spawner-target').children[0].textContent,/Missing reference/);
    assert.equal(editor.definition().spawner.entity,target);
});

test('spawn count assertions preserve zero and removed emitters and follow expected scene scope',()=>{
    const f=fixture(),{editor,instance}=addSpawner(f);
    let row=f.field('tests-steps').children[0];
    let input=row.children.find(child=>child.textContent.includes(`expected spawns Enemy nest (${instance})`)).children[0];
    input.value='0';input.dispatchEvent(new f.Event('input'));assert.equal(f.studio.getData().tests[0].steps[0].expected_spawns[instance],0);
    editor.removeInstance();row=f.field('tests-steps').children[0];
    input=row.children.find(child=>child.textContent.includes(`Missing spawner (${instance})`)).children[0];
    assert.equal(input.value,0);input.value='';input.dispatchEvent(new f.Event('input'));assert.deepEqual(Object.keys(f.studio.getData().tests[0].steps[0].expected_spawns),[]);
    editor.place();f.studio.design.tests[0].steps[0].expected_scene='title';f.studio.renderTests();
    assert.ok(f.field('tests-steps').children[0].children.every(child=>!child.textContent.includes('expected spawns')));
});

test('spawner conditions stay scene scoped and namespace separate from variable references',()=>{
    const f=fixture(),{instance}=addSpawner(f),events=f.studio.eventEditor;events.addEvent();f.field('condition-add').click();
    let source=f.field('event-conditions').children[0].children[0].children[0];source.value=`spawn:${instance}`;source.onchange();
    assert.equal(events.event().conditions[0].source.kind,'spawned');assert.equal(events.event().conditions[0].source.instance,instance);
    f.field('event-scene').value='title';f.field('event-scene').onchange();source=f.field('event-conditions').children[0].children[0].children[0];
    assert.match(source.children[0].textContent,/Missing reference/);assert.equal(events.event().conditions[0].source.instance,instance);
    f.studio.load('adventure',StudioStarter.build().design);assert.equal(f.field('spawner-fields').hidden,true);assert.equal(f.studio.design.entities.some(entity=>entity.behavior==='spawner'),false);
});

test('starter creates independent editable art, audio, map and game-flow data', () => {
    const first = StudioStarter.build(), second = StudioStarter.build();
    first.assets.chr_bank[0] = 99; first.design.scenes[0].name = 'Changed';
    assert.equal(second.assets.chr_bank[0],0); assert.equal(second.design.scenes[0].name,'Title');
    assert.equal(second.assets.chr_bank.length,8192);
    assert.equal(second.assets.nametables.length,3);
    assert.equal(second.assets.animations[0].frames.length,2);
    assert.equal(second.design.scenes[0].transitions[0].target,'meadow');
});

test('combat enables with enemies, captures settings and cannot disable referenced behavior',()=>{
    const f=fixture(),editor=f.studio.entityEditor;
    editor.addDefinition();f.field('entity-behavior').value='enemy';f.field('entity-behavior').onchange();
    assert.equal(f.studio.getData().combat.health,3);assert.equal(f.field('combat-enabled').checked,true);
    assert.equal(f.field('enemy-fields').hidden,false);
    f.field('combat-health').value='9';f.field('combat-health').dispatchEvent(new f.Event('input'));
    f.field('enemy-health').value='2';f.field('enemy-health').dispatchEvent(new f.Event('input'));
    assert.equal(f.studio.getData().combat.health,9);assert.equal(editor.definition().enemy.health,2);
    f.field('combat-enabled').checked=false;f.field('combat-enabled').onchange();
    assert.equal(f.field('combat-enabled').checked,true);assert.match(f.field('status').textContent,/enemy definitions/);
    f.studio.selectedScene='meadow';f.studio.render();editor.place();
    assert.equal(editor.instance().patrol_end_x,null);assert.equal(f.field('patrol-field').hidden,false);
    f.field('instance-end').value='';f.field('instance-end').dispatchEvent(new f.Event('input'));
    assert.equal(editor.instance().patrol_end_x,null);
    assert.match(f.field('budget').textContent,/20\/64 sprites.*61\/560 bytes/);
});

test('facing attacks and pursuit settings persist, reset with project loads and share the facing byte',()=>{
    const f=fixture(),editor=f.studio.entityEditor;
    editor.addDefinition();f.field('entity-behavior').value='enemy';f.field('entity-behavior').onchange();
    assert.equal(f.field('combat-mode').value,'all_directions');assert.equal(f.field('enemy-motion').value,'patrol');
    assert.equal(f.field('enemy-range').disabled,true);
    f.studio.selectedScene='meadow';f.studio.render();editor.place();
    const ram=()=>f.studio.eventEditor.constructor.ramBytes(f.studio.design);
    const base=ram();
    f.field('combat-mode').value='facing';f.field('combat-mode').onchange();
    assert.equal(ram(),base+1);
    f.field('player-face_movement').checked=true;f.field('player-face_movement').onchange();
    assert.equal(ram(),base+1);
    f.field('enemy-motion').value='pursue';f.field('enemy-motion').onchange();
    assert.equal(f.field('enemy-range').disabled,false);
    f.field('enemy-range').value='48';f.field('enemy-range').dispatchEvent(new f.Event('input'));
    f.field('instance-end').value='96';f.field('instance-end').dispatchEvent(new f.Event('input'));
    const data=f.studio.getData();assert.equal(data.combat.attack_mode,'facing');
    assert.equal(data.entities.at(-1).enemy.motion,'pursue');assert.equal(data.entities.at(-1).enemy.detection_range,48);
    assert.equal(data.scenes[1].instances.at(-1).patrol_end_x,96);
    f.studio.load('adventure',data);assert.equal(f.field('combat-mode').value,'facing');
    editor.definitionId=data.entities.at(-1).id;editor.render();assert.equal(f.field('enemy-motion').value,'pursue');
    f.studio.load('adventure',StudioStarter.build().design);assert.equal(f.field('combat-mode').value,'all_directions');
    assert.equal(f.field('enemy-motion').value,'patrol');assert.equal(f.field('enemy-range').value,64);
});

test('attack cooldown edits persist and immediately update the optional two-byte RAM budget',()=>{
    const f=fixture();f.field('combat-enabled').checked=true;f.field('combat-enabled').onchange();
    const ram=()=>f.studio.eventEditor.constructor.ramBytes(f.studio.design),base=ram();
    assert.equal(f.field('combat-cooldown').value,0);
    f.field('combat-cooldown').value='12';f.field('combat-cooldown').dispatchEvent(new f.Event('input'));
    assert.equal(f.studio.getData().combat.attack_cooldown_frames,12);assert.equal(ram(),base+2);
    assert.match(f.field('budget').textContent,new RegExp(`${base+2}/560 bytes`));
    f.studio.load('adventure',f.studio.getData());assert.equal(f.field('combat-cooldown').value,12);
    f.field('combat-cooldown').value='0';f.field('combat-cooldown').dispatchEvent(new f.Event('input'));
    assert.equal(ram(),base);assert.equal(f.studio.getData().combat.attack_cooldown_frames,0);
    const legacy=f.studio.getData();delete legacy.combat.attack_cooldown_frames;
    f.studio.load('adventure',legacy);assert.equal(f.field('combat-cooldown').value,0);
});

test('collision edits and markers use offsets without moving art or placement origins',()=>{
    const f=fixture();f.studio.selectedScene='meadow';f.studio.render();
    for(const [id,value] of [['entity-hitbox-x',4],['entity-hitbox-width',8],['player-hitbox-y',8],['player-hitbox-height',8]]) {
        f.field(id).value=String(value);f.field(id).dispatchEvent(new f.Event('input'));
    }
    const snapshot=f.studio.getData();assert.equal(snapshot.entities[0].hitbox.width,8);
    assert.equal(snapshot.scenes[1].instances[0].x,80);assert.equal(snapshot.player_hitbox.y,8);
    const spawn=f.field('placement').children[1],coin=f.field('placement').children[2];
    assert.equal(spawn.style.top,`${136/240*100}%`);assert.equal(spawn.style.height,`${8/240*100}%`);
    assert.equal(coin.style.left,`${84/256*100}%`);assert.equal(coin.style.width,`${8/256*100}%`);
    f.studio.load('adventure',StudioStarter.build().design);
    assert.equal(f.field('entity-hitbox-width').value,16);assert.equal(f.field('player-hitbox-y').value,0);
    assert.equal(f.studio.getData().combat,undefined);
});

test('combat assertions preserve zero and missing state while restart and health rules are editable',()=>{
    const f=fixture();f.field('combat-enabled').checked=true;f.field('combat-enabled').onchange();
    let row=f.field('tests-steps').children[0];let health=row.children[7].children[0],defeated=row.children[8].children[0];
    health.value='0';health.dispatchEvent(new f.Event('input'));defeated.value='1';defeated.dispatchEvent(new f.Event('input'));
    f.field('combat-enabled').checked=false;f.field('combat-enabled').onchange();
    row=f.field('tests-steps').children[0];assert.equal(row.children[7].children[0].value,0);
    assert.equal(f.studio.getData().tests[0].steps[0].expected_health,0);
    const editor=f.studio.eventEditor;editor.addEvent();f.field('condition-add').click();
    let source=f.field('event-conditions').children[0].children[0].children[0];
    source.value='health';source.onchange();assert.equal(editor.event().conditions[0].source.kind,'health');
    const action=f.field('event-actions').children[0].children[0].children[0];action.value='restart';action.onchange();
    assert.equal(editor.event().actions[0].kind,'restart');
    health=f.field('tests-steps').children[0].children[7].children[0];health.value='';health.dispatchEvent(new f.Event('input'));
    assert.equal(f.studio.getData().tests[0].steps[0].expected_health,null);
});

test('variable and event edits capture unblurred values and retain stable references on rename', () => {
    const f=fixture(),editor=f.studio.eventEditor;
    editor.addVariable();const id=f.studio.design.variables[0].id;
    const variableRow=f.field('variables').children[0];
    const name=variableRow.children[0].children[0],initial=variableRow.children[1].children[0];
    name.value='Score';name.dispatchEvent(new f.Event('input'));initial.value='60000';initial.dispatchEvent(new f.Event('input'));
    assert.match(f.field('tests-steps').children[0].children[7].textContent,/expected variable Score/);
    editor.addEvent();f.field('event-name').value='Title bonus';f.field('event-name').dispatchEvent(new f.Event('input'));
    f.field('event-trigger').value='pressed';f.field('event-trigger').onchange();
    f.field('event-button').value='a';f.field('event-button').onchange();
    let actionRow=f.field('event-actions').children[0];
    actionRow.children[0].children[0].value='add';actionRow.children[0].children[0].onchange();
    actionRow=f.field('event-actions').children[0];const value=actionRow.children[2].children[0];value.value='5535';value.dispatchEvent(new f.Event('input'));
    f.field('condition-add').click();const conditionRow=f.field('event-conditions').children[0];
    const threshold=conditionRow.children[2].children[0];threshold.value='60000';threshold.dispatchEvent(new f.Event('input'));
    const snapshot=f.studio.getData();
    assert.equal(snapshot.variables[0].name,'Score');assert.equal(snapshot.variables[0].initial,60000);
    assert.equal(snapshot.events[0].name,'Title bonus');assert.equal(snapshot.events[0].actions[0].variable,id);
    assert.equal(snapshot.events[0].actions[0].value,5535);assert.equal(snapshot.events[0].conditions[0].value,60000);
    const renamed=f.field('variables').children[0].children[0].children[0];renamed.value='Points';renamed.dispatchEvent(new f.Event('input'));renamed.dispatchEvent(new f.Event('change'));
    assert.equal(f.studio.getData().events[0].conditions[0].source.variable,id);
    assert.equal(f.field('event-actions').children[0].children[1].children[0].children[0].textContent,'Points');
});

test('event and action priority controls preserve rule identity and timer repetition policy', () => {
    const f=fixture(),editor=f.studio.eventEditor;editor.addVariable();editor.addEvent();const first=editor.event().id;editor.addEvent();const second=editor.event().id;
    f.field('event-up').click();assert.equal(f.studio.design.events[0].id,second);assert.equal(editor.event().id,second);
    f.field('event-down').click();assert.equal(f.studio.design.events[0].id,first);
    f.field('event-trigger').value='timer';f.field('event-trigger').onchange();assert.equal(editor.event().once_per_visit,true);
    f.field('event-frames').value='24';f.field('event-frames').dispatchEvent(new f.Event('input'));
    f.field('event-once').checked=false;f.field('event-once').onchange();assert.equal(editor.event().once_per_visit,false);
    f.field('action-add').click();const row=f.field('event-actions').children[1];
    row.children[0].children[0].value='scene';row.children[0].children[0].onchange();
    const sceneRow=f.field('event-actions').children[1];sceneRow.children[2].click();
    assert.equal(editor.event().actions[0].kind,'scene');assert.equal(editor.event().actions[1].kind,'set');
    assert.equal(editor.event().trigger.frames,24);assert.match(f.field('budget').textContent,/49\/560 bytes/);
});

test('variable deletion protects rule references and missing scene references remain visible for repair', () => {
    const f=fixture(),editor=f.studio.eventEditor;editor.addVariable();editor.addEvent();
    f.field('variables').children[0].children[2].click();
    assert.equal(f.studio.design.variables.length,1);assert.match(f.field('status').textContent,/event references/);
    f.studio.removeScene();assert.equal(editor.event().scene,'title');
    assert.match(f.field('event-scene').children[0].textContent,/Missing reference: title/);
    f.field('event-scene').value='meadow';f.field('event-scene').onchange();assert.equal(editor.event().scene,'meadow');
    const kind=f.field('event-actions').children[0].children[0].children[0];kind.value='stop_music';kind.onchange();
    editor.removeVariable(f.studio.design.variables[0].id);assert.equal(f.studio.design.variables.length,0);
});

test('variable assertions distinguish zero and blank and retain missing IDs instead of erasing coverage', () => {
    const f=fixture(),editor=f.studio.eventEditor;editor.addVariable();const id=f.studio.design.variables[0].id;
    const row=f.field('tests-steps').children[0],input=row.children[7].children[0];
    input.value='0';input.dispatchEvent(new f.Event('input'));assert.equal(f.studio.currentTest().steps[0].expected_variables[id],0);
    input.value='';input.dispatchEvent(new f.Event('input'));assert.equal(Object.hasOwn(f.studio.currentTest().steps[0].expected_variables,id),false);
    input.value='5';input.dispatchEvent(new f.Event('input'));editor.removeVariable(id);
    const missing=f.field('tests-steps').children[0].children[7];assert.match(missing.textContent,/Missing:/);assert.equal(missing.children[0].value,5);
    assert.equal(f.studio.currentTest().steps[0].expected_variables[id],5);
});

test('legacy and switched projects get independent event collections and no stale rule selection', () => {
    const f=fixture(),editor=f.studio.eventEditor;editor.addVariable();editor.addEvent();
    const old=f.studio.getData(),legacy=StudioStarter.build().design;f.studio.load('other',legacy);
    assert.equal(f.studio.design.variables.length,0);assert.equal(f.studio.design.events.length,0);assert.equal(editor.event(),undefined);
    assert.equal(old.events.length,1);f.studio.enable();assert.equal(f.studio.design.events.length,0);
});

test('renaming a variable cannot rename a scene option with the same ID', () => {
    const f=fixture(),editor=f.studio.eventEditor;editor.addVariable();f.studio.design.variables[0].id='title';editor.addEvent();
    editor.event().actions=[{kind:'scene',scene:'title'}];editor.render();
    const name=f.field('variables').children[0].children[0].children[0];name.value='Points';name.dispatchEvent(new f.Event('input'));
    assert.equal(f.field('event-actions').children[0].children[1].children[0].children[0].textContent,'Title');
});

test('scene removal repairs the start node and incoming links while renaming retains stable IDs', () => {
    const f = fixture();
    f.field('scene-name').value = 'New title'; f.studio.applyFields();
    assert.equal(f.studio.design.start_scene,'title');
    assert.equal(f.studio.design.scenes[1].transitions[1].target,'title');
    f.studio.removeScene();
    assert.equal(f.studio.design.start_scene,'meadow');
    assert.ok(f.studio.design.scenes.every(scene=>scene.transitions.every(edge=>edge.target!=='title')));
    f.studio.addScene(); assert.equal(f.studio.selectedScene,'scene-1');
    f.studio.addTransition(); assert.equal(f.studio.design.scenes.at(-1).transitions.length,1);
});

test('graph labels are text and stale project ownership never supplies a design snapshot', () => {
    const f = fixture();
    f.studio.design.scenes[0].name = '<img src=x onerror=bad>'; f.studio.renderGraph();
    const card = f.field('graph').children[1];
    assert.ok(card.textContent.includes('<img src=x onerror=bad>'));
    assert.equal(card.children.length,0);
    f.window.projectManager.currentProject = 'other';
    assert.equal(f.studio.getData(),null);
});

test('starter creation preserves a project switch while the POST is pending', async () => {
    const response = deferred(), payloads = [];
    const f = fixture((url,options) => { payloads.push(JSON.parse(options.body)); return response.promise; });
    f.field('new-name').value = 'new-adventure';
    const create = f.studio.createStarter(); await new Promise(resolve=>setImmediate(resolve));
    f.window.projectManager.loadRevision++; f.window.projectManager.currentProject = 'other';
    response.resolve({ok:true}); await create;
    assert.deepEqual(f.calls,[]);
    assert.equal(payloads[0].name,'new-adventure');
    assert.equal(payloads[0].design.scenes.length,3);
});

test('smoke test discards obsolete responses and preview provides a separate source harness', async () => {
    const response = deferred();
    const f = fixture(()=>response.promise);
    const test = f.studio.test(); await new Promise(resolve=>setImmediate(resolve));
    f.studio.changed();
    response.resolve({ok:true,json:async()=>({frames:120,scene_name:'Title',rom_bytes:139280})}); await test;
    assert.ok(f.field('status').textContent.startsWith('Unsaved'));
    let options; f.window.editor = {runEmulator(value) { options = value; }};
    f.studio.run(); assert.equal(options.sourceOverride,StudioStarter.mainSource);
});

test('Inspect does not switch source or navigate after an edit, project reload or file switch during save', async () => {
    for (const change of [
        f => f.studio.changed(),
        f => { f.window.projectManager.currentProject = 'other'; },
        f => { f.window.projectManager.loadRevision++; },
        f => { f.window.projectManager.fileLoadRevision++; }
    ]) {
        const f = fixture(), save = deferred();
        let loads = 0, navigations = 0;
        f.window.projectManager.saveCurrentProject = () => save.promise;
        f.window.projectManager.loadFile = async () => { loads++; return true; };
        f.window.addEventListener('navigate-to', () => { navigations++; });
        const inspect = f.studio.inspectCode();
        change(f); save.resolve(true); await inspect;
        assert.equal(loads,0);
        assert.equal(navigations,0);
    }
});

test('Inspect discards navigation when project ownership changes during file loading', async () => {
    const f = fixture(), loaded = deferred();
    f.window.projectManager.loadFile = () => loaded.promise;
    let navigations = 0;
    f.window.addEventListener('navigate-to', () => { navigations++; });
    const inspect = f.studio.inspectCode();
    await new Promise(resolve=>setImmediate(resolve));
    f.window.projectManager.loadRevision++;
    loaded.resolve(true); await inspect;
    assert.equal(navigations,0);
});

test('Studio shows the backend validation error belonging to the current project', async () => {
    const f = fixture();
    f.window.projectManager.saveCurrentProject = async () => false;
    f.window.projectManager.lastSaveError = {project:'adventure',message:"Scene 'Meadow' refers to missing music"};
    await f.studio.save();
    assert.match(f.field('status').textContent,/Meadow.*missing music/);
});

test('typing updates design snapshots before blur without rebuilding the focused input', () => {
    const f = fixture();
    const input = f.field('scene-name');
    input.value = 'Village'; input.dispatchEvent(new f.Event('input'));
    assert.equal(f.studio.getData().scenes[0].name,'Village');
    assert.equal(f.field('scene-name'),input);
    const revision = f.studio.revision;
    input.dispatchEvent(new f.Event('change'));
    assert.equal(f.studio.revision,revision,'blur must not invalidate the same snapshot again');
});

test('Save captures unblurred inspector values and Preview reserves its updated design first', async () => {
    const f = fixture();
    let saved;
    f.window.projectManager.saveCurrentProject = async () => { saved = f.studio.getData(); return true; };
    f.field('scene-name').value = 'Village'; f.field('x').value = '48';
    await f.studio.save();
    assert.equal(saved.scenes[0].name,'Village'); assert.equal(saved.scenes[0].spawn_x,48);
    let preview;
    f.window.editor = {runEmulator() { preview = f.studio.getData(); }};
    f.field('scene-name').value = 'Harbor'; f.studio.run();
    assert.equal(preview.scenes[0].name,'Harbor');
});

test('controller scenarios edit and serialize holds, duration and assertions without blur', () => {
    const f=fixture(); f.studio.addTest();f.studio.addTestStep();
    const row=f.field('tests-steps').children[0];
    const frames=row.children[0].children[0];frames.value='25';frames.dispatchEvent(new f.Event('input'));
    const expectedX=row.children[2].children[0];expectedX.value='0';expectedX.dispatchEvent(new f.Event('input'));
    const start=row.children[4].children[2].children[0];start.checked=true;start.onchange();
    const data=f.studio.getData(),test=data.tests.find(test=>test.id===f.studio.selectedTest);
    assert.equal(test.steps.length,2);assert.equal(test.steps[0].frames,25);assert.equal(test.steps[0].expected_x,0);
    assert.deepEqual(Array.from(test.steps[0].buttons),['start']);
});

test('controller failures are shown with step details and obsolete case responses are discarded', async () => {
    const response=deferred();const f=fixture(()=>response.promise);
    const run=f.studio.runSavedTest();await new Promise(resolve=>setImmediate(resolve));
    response.resolve({ok:true,json:async()=>({passed:false,frames:12,cpu_cycles:300000,scope:'saved controller scenario',message:'Step 1: expected title',
        observations:[{step:1,frames:12,passed:false,state:{scene_name:'Meadow',x:32,y:128}}]})});
    await run;assert.match(f.field('tests-result').textContent,/FAIL.*Enter gameplay/);
    assert.match(f.field('tests-result').textContent,/Step 1: FAIL/);assert.equal(f.field('tests-result').dataset.error,'true');
    const late=deferred();f.context.fetch=()=>late.promise;
    const second=f.studio.runSavedTest();await new Promise(resolve=>setImmediate(resolve));
    f.field('tests-select').value='round-trip';f.field('tests-select').onchange();
    late.resolve({ok:true,json:async()=>({passed:true,frames:12,cpu_cycles:3,scope:'old'})});await second;
    assert.ok(f.field('tests-result').textContent.startsWith('Run the selected scenario'));
});

test('removing saved tests retains remaining scenarios and permits an empty test suite', () => {
    const f=fixture(); f.studio.removeTest();
    assert.equal(f.studio.selectedTest,'round-trip');
    assert.equal(f.studio.design.tests.length,2);
    assert.ok(f.studio.design.tests.every(test=>test.id!=='enter-play'));
    f.studio.removeTest();f.studio.removeTest();
    assert.equal(f.studio.getData().tests.length,0);
    assert.equal(f.studio.currentTest(),undefined);
    assert.match(f.field('tests-result').textContent,/Add a scenario/);
});

test('entity edits and exact placement snapshot immediately without blur or changing stable references', () => {
    const f=fixture(), editor=f.studio.entityEditor;
    f.field('entity-name').value='Gold reward';f.field('entity-name').dispatchEvent(new f.Event('input'));
    assert.equal(f.studio.getData().entities[0].name,'Gold reward');
    assert.equal(f.studio.getData().scenes[1].instances[0].entity,'coin');
    f.studio.selectedScene='meadow';f.studio.render();
    f.field('instance-x').value='73';f.field('instance-x').dispatchEvent(new f.Event('input'));
    assert.equal(f.studio.getData().scenes[1].instances[0].x,73);
    assert.match(f.field('budget').textContent,/16\/64 sprites.*44\/560 bytes/);
    f.field('placement').onclick({clientX:101,clientY:103});
    assert.equal(editor.instance().x,96);assert.equal(editor.instance().y,96);
    const door=f.field('placement').children[3];door.onclick({stopPropagation(){}});
    assert.equal(editor.instance().entity,'door');assert.equal(f.field('entity-name').value,'Room doorway');
    assert.equal(f.field('place-type').value,'door');
});

test('definition deletion protects placed instances and placement IDs are unique between rooms', () => {
    const f=fixture(), editor=f.studio.entityEditor;
    editor.removeDefinition();assert.equal(f.studio.design.entities.length,3);
    assert.match(f.field('status').textContent,/placed instances/);
    editor.addDefinition();const id=editor.definitionId;
    assert.equal(f.field('place-type').value,id);
    f.field('place-type').value=id;editor.place();
    const first=editor.instanceId;f.studio.selectedScene='meadow';f.studio.render();
    f.field('place-type').value=id;editor.place();assert.notEqual(editor.instanceId,first);
    editor.removeInstance();editor.removeDefinition();assert.ok(f.studio.design.entities.some(item=>item.id===id));
    f.studio.selectedScene='title';f.studio.render();editor.removeInstance();editor.removeDefinition();
    assert.ok(f.studio.design.entities.every(item=>item.id!==id));
});

test('missing door links remain visible after removing a destination and scene switch cannot edit another room', () => {
    const f=fixture();f.studio.selectedScene='ruins';f.studio.removeScene();
    const door=f.studio.design.scenes[1].instances[1];assert.equal(door.target_scene,'ruins');
    f.studio.selectedScene='meadow';f.studio.render();f.field('instance').value=door.id;f.field('instance').onchange();
    assert.match(f.field('instance-target').children[0].textContent,/Missing reference: ruins/);
    f.studio.selectedScene='title';f.studio.render();f.field('instance-x').value='55';f.field('instance-x').dispatchEvent(new f.Event('input'));
    assert.equal(door.x,128);assert.equal(f.studio.entityEditor.instance(),undefined);
});

test('old design loads with empty entity collections and connecting a project does not import starter references', () => {
    const f=fixture(), legacy=StudioStarter.build().design;
    delete legacy.entities;for(const scene of legacy.scenes)delete scene.instances;
    f.studio.load('adventure',legacy);assert.equal(f.studio.getData().entities.length,0);
    f.studio.addScene();assert.equal(f.studio.design.scenes.at(-1).instances.length,0);
    f.studio.enable();assert.equal(f.studio.design.entities.length,0);assert.equal(f.studio.design.scenes[0].instances.length,0);
});

test('scenario entity count assertions distinguish zero from no assertion in saved snapshots', () => {
    const f=fixture();const row=f.field('tests-steps').children[0];
    const pickups=row.children[5].children[0],active=row.children[6].children[0];
    pickups.value='0';pickups.dispatchEvent(new f.Event('input'));
    active.value='2';active.dispatchEvent(new f.Event('input'));
    let step=f.studio.getData().tests[0].steps[0];assert.equal(step.expected_pickups,0);assert.equal(step.expected_active_entities,2);
    pickups.value='';pickups.dispatchEvent(new f.Event('input'));step=f.studio.getData().tests[0].steps[0];assert.equal(step.expected_pickups,null);
});

test('placement preview preserves palette RGB components in the actual pixel buffer', () => {
    const f=fixture(), create=f.document.createElement;let pixels;
    f.document.createElement=tag=>{
        const element=create(tag);
        if(tag==='canvas')element.context={
            createImageData:(width,height)=>({data:new Uint8ClampedArray(width*height*4)}),
            putImageData:image=>{pixels=image.data;}
        };
        return element;
    };
    f.window.paletteEditor={nesPalette:Array(64).fill('123456')};
    f.studio.entityEditor.drawPlacement();
    assert.deepEqual(Array.from(pixels.slice(232*256*4,232*256*4+4)),[18,52,86,255]);
});
