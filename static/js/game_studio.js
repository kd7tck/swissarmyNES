class GameStudio {
    constructor() {
        this.root = document.getElementById('studio');
        this.design = null;
        this.projectName = null;
        this.selectedScene = null;
        this.revision = 0;
        if (!this.root) return;
        this.field = id => document.getElementById(`studio-${id}`);
        this.entityEditor = new StudioEntities(this);
        this.eventEditor = new StudioEvents(this);
        this.field('create').onclick = () => this.createStarter();
        this.field('enable').onclick = () => this.enable();
        this.field('save').onclick = () => this.save();
        this.field('run').onclick = () => this.run();
        this.field('test').onclick = () => this.test();
        this.field('code').onclick = () => this.inspectCode();
        this.field('add-scene').onclick = () => this.addScene();
        this.field('remove-scene').onclick = () => this.removeScene();
        this.field('add-transition').onclick = () => this.addTransition();
        this.field('tests-add').onclick = () => this.addTest();
        this.field('tests-add-step').onclick = () => this.addTestStep();
        this.field('tests-remove').onclick = () => this.removeTest();
        this.field('tests-run').onclick = () => this.runSavedTest();
        this.field('tests-select').onchange = () => {
            this.selectedTest = this.field('tests-select').value; this.renderTests();
            this.field('tests-result').textContent = 'Run the selected scenario to obtain current results.';
            this.field('tests-result').dataset.error = 'false';
        };
        this.field('tests-name').addEventListener('input', () => {
            const test = this.currentTest();
            if (test) {
                test.name = this.field('tests-name').value;
                for (const option of this.field('tests-select').children) if (option.value === test.id) option.textContent = test.name;
                this.changed();
            }
        });
        for (const id of ['title','brief','start','animation','scene-name','map','x','y','movement','music','sfx']) {
            this.field(id).addEventListener('change', () => this.applyFields());
        }
        for (const id of ['title','brief','scene-name','x','y']) {
            // Keep snapshots current while typing, without replacing the focused form controls.
            this.field(id).addEventListener('input', () => this.applyFields(false));
        }
        window.addEventListener('project-ready', event => this.load(event.detail.name, event.detail.design));
        window.addEventListener('studio-opened', () => this.render());
        this.load(window.projectManager?.currentProject, window.projectManager?.design);
    }

    getData() {
        return this.projectName === window.projectManager?.currentProject && this.design ? JSON.parse(JSON.stringify(this.design)) : null;
    }

    assets() {
        const assets = {...window.projectManager?.assets};
        if (window.audioTracker) Object.assign(assets, window.audioTracker.getData());
        if (window.sfxEditor) assets.sound_effects = window.sfxEditor.getData();
        if (window.worldEditor) assets.world = window.worldEditor.getData();
        if (window.spriteEditor) Object.assign(assets, window.spriteEditor.getData());
        return assets;
    }

    load(name, design) {
        this.projectName = name || null;
        this.design = design ? JSON.parse(JSON.stringify(design)) : null;
        if (this.design) {
            this.design.tests ||= []; this.design.entities ||= [];
            this.design.variables ||= []; this.design.events ||= [];
            for (const event of this.design.events) event.conditions ||= [];
            for (const scene of this.design.scenes) scene.instances ||= [];
        }
        this.entityEditor?.reset();
        this.eventEditor?.reset();
        this.selectedTest = this.design?.tests[0]?.id || null;
        this.selectedScene = this.design?.start_scene || null;
        this.revision++;
        if (this.root) {
            this.field('tests-result').textContent = 'Run a scenario to obtain current results.';
            this.field('tests-result').dataset.error = 'false';
            this.status('Choose a starter or connect your existing project.'); this.render();
        }
    }

    status(message, error = false) {
        this.field('status').textContent = message;
        this.field('status').dataset.error = String(error);
    }

    changed() {
        this.revision++;
        window.dispatchEvent(new CustomEvent('game-design-changed'));
        this.status('Unsaved design changes. Save, preview, or smoke test to build the current flow.');
        this.field('tests-result').textContent = 'Design changed. Run the scenario again to obtain current results.';
        this.field('tests-result').dataset.error = 'false';
        this.renderGraph();
    }

    options(id, choices, selected) {
        const input = this.field(id);
        input.innerHTML = '';
        if (selected !== undefined && !choices.some(choice => String(choice.value) === String(selected))) {
            choices = [{value:selected,label:`Missing reference: ${selected}`}, ...choices];
        }
        for (const choice of choices) {
            const option = document.createElement('option');
            option.value = String(choice.value); option.textContent = choice.label; input.appendChild(option);
        }
        input.value = String(selected ?? choices[0]?.value ?? '');
    }

    render() {
        if (!this.root) return;
        this.field('workspace').hidden = !this.design;
        this.field('enable').hidden = !!this.design || !this.projectName;
        this.field('project').textContent = this.projectName || 'No project loaded';
        if (!this.design) return;
        const assets = this.assets(), scenes = this.design.scenes;
        this.field('title').value = this.design.title;
        this.field('brief').value = this.design.core_loop;
        this.options('start', scenes.map(scene => ({value:scene.id,label:scene.name})), this.design.start_scene);
        this.options('animation', [{value:'',label:'No player animation'}, ...(assets.animations || []).map(item => ({value:item.name,label:item.name}))], this.design.animation || '');
        const scene = scenes.find(item => item.id === this.selectedScene) || scenes[0];
        this.selectedScene = scene.id;
        this.field('scene-name').value = scene.name;
        this.field('x').value = scene.spawn_x; this.field('y').value = scene.spawn_y;
        this.field('movement').checked = scene.movement;
        const maps = [], world = assets.world;
        for (let cell = 0; cell < (world?.data.length || 0); cell++) {
            const map = assets.nametables?.[world.data[cell]];
            if (map) maps.push({value:`${cell % world.width},${Math.floor(cell / world.width)}`,label:`${map.name} (${cell % world.width}, ${Math.floor(cell / world.width)})`});
        }
        this.options('map', maps, `${scene.map_col},${scene.map_row}`);
        for (const [field, items] of [['music',assets.audio_tracks], ['sfx',assets.sound_effects]]) {
            this.options(field, [{value:'',label:field === 'music' ? 'Silence' : 'No entry effect'}, ...(items || []).map((item,index) => ({value:index,label:item.name || `${field} ${index}`}))], scene[field] ?? '');
        }
        this.renderTransitions(scene);
        this.renderGraph();
        this.field('summary').textContent = `${scenes.length} scenes · ${scenes.reduce((sum,item) => sum + item.transitions.length,0)} transitions · ${assets.nametables?.length || 0} maps · ${assets.animations?.length || 0} animations · ${assets.audio_tracks?.length || 0} music tracks · ${assets.sound_effects?.length || 0} effects`;
        this.renderTests();
        this.entityEditor.render();
        this.eventEditor.render();
    }

    applyFields(render = true) {
        if (!this.design) return;
        const previous = JSON.stringify(this.design);
        const scene = this.design.scenes.find(item => item.id === this.selectedScene);
        this.design.title = this.field('title').value;
        this.design.core_loop = this.field('brief').value;
        this.design.start_scene = this.field('start').value;
        this.design.animation = this.field('animation').value || null;
        scene.name = this.field('scene-name').value;
        [scene.map_col, scene.map_row] = this.field('map').value.split(',').map(Number);
        scene.spawn_x = Number(this.field('x').value); scene.spawn_y = Number(this.field('y').value);
        scene.movement = this.field('movement').checked;
        scene.music = this.field('music').value === '' ? null : Number(this.field('music').value);
        scene.sfx = this.field('sfx').value === '' ? null : Number(this.field('sfx').value);
        if (previous !== JSON.stringify(this.design)) {
            this.changed();
            if (!render) this.entityEditor.drawPlacement();
        }
        if (render) this.render();
    }

    renderGraph() {
        const graph = this.field('graph'); graph.innerHTML = '';
        if (!this.design) return;
        const width = 760, positions = this.design.scenes.map((_,i) => ({x:20 + i % 3 * 250,y:20 + Math.floor(i / 3) * 150}));
        const height = Math.max(170, Math.ceil(positions.length / 3) * 150 + 20);
        graph.style.width = `${width}px`; graph.style.height = `${height}px`;
        const svg = document.createElementNS('http://www.w3.org/2000/svg','svg');
        svg.setAttribute('viewBox',`0 0 ${width} ${height}`); svg.setAttribute('aria-hidden','true');
        svg.classList.add('studio-connections'); graph.appendChild(svg);
        this.design.scenes.forEach((scene,index) => {
            const point = positions[index];
            for (const edge of scene.transitions) {
                const targetIndex = this.design.scenes.findIndex(item => item.id === edge.target);
                if (targetIndex < 0) continue;
                const target = positions[targetIndex], path = document.createElementNS('http://www.w3.org/2000/svg','path');
                path.setAttribute('d', `M ${point.x + 105} ${point.y + 98} C ${point.x + 105} ${point.y + 140}, ${target.x + 105} ${target.y - 15}, ${target.x + 105} ${target.y}`);
                svg.appendChild(path);
            }
            const button = document.createElement('button'); button.className = 'studio-scene';
            button.style.left = `${point.x}px`; button.style.top = `${point.y}px`;
            button.dataset.selected = String(scene.id === this.selectedScene);
            button.textContent = `${scene.id === this.design.start_scene ? '▶ ' : ''}${scene.name}\nMap ${scene.map_col},${scene.map_row}\n${scene.movement ? 'Explore' : 'Screen'} · ${scene.transitions.length} exits`;
            button.onclick = () => { this.applyFields(false); this.selectedScene = scene.id; this.render(); };
            graph.appendChild(button);
        });
    }

    renderTransitions(scene) {
        const container = this.field('transitions'); container.innerHTML = '';
        scene.transitions.forEach((edge,index) => {
            const row = document.createElement('div'); row.className = 'studio-transition';
            const trigger = document.createElement('select'), target = document.createElement('select'), frames = document.createElement('input');
            trigger.setAttribute('aria-label', `Transition ${index + 1} trigger`);
            target.setAttribute('aria-label', `Transition ${index + 1} destination`);
            frames.setAttribute('aria-label', `Transition ${index + 1} timer frames`);
            for (const key of ['start','select','a','b','up','down','left','right','timer']) {
                const option = document.createElement('option'); option.value = key; option.textContent = key === 'timer' ? 'After timer' : `Press ${key.toUpperCase()}`; trigger.appendChild(option);
            }
            for (const item of this.design.scenes) {
                const option = document.createElement('option'); option.value = item.id; option.textContent = `→ ${item.name}`; target.appendChild(option);
            }
            trigger.value = edge.trigger; target.value = edge.target;
            frames.type = 'number'; frames.min = 1; frames.max = 65535; frames.value = edge.frames || 120; frames.hidden = edge.trigger !== 'timer';
            const update = () => { edge.trigger = trigger.value; edge.target = target.value; edge.frames = Number(frames.value); this.changed(); this.renderTransitions(scene); };
            trigger.onchange = update; target.onchange = update; frames.onchange = update;
            const remove = document.createElement('button'); remove.textContent = 'Remove';
            remove.onclick = () => { scene.transitions.splice(index,1); this.changed(); this.render(); };
            row.append(trigger,target,frames,remove); container.appendChild(row);
        });
    }

    enable() {
        if (!this.projectName) return;
        const starter = StudioStarter.build();
        this.design = {...starter.design,title:this.projectName,animation:this.assets().animations?.[0]?.name || null,
            start_scene:'scene-1',tests:[],entities:[],variables:[],events:[],scenes:[{...starter.design.scenes[0],id:'scene-1',name:'Scene 1',transitions:[],instances:[]}]};
        this.selectedScene = 'scene-1'; this.changed(); this.render();
    }

    addScene() {
        if (!this.design || this.design.scenes.length >= 255) return;
        let index = 1; while (this.design.scenes.some(scene => scene.id === `scene-${index}`)) index++;
        this.design.scenes.push({id:`scene-${index}`,name:`Scene ${index}`,map_col:0,map_row:0,spawn_x:32,spawn_y:128,movement:true,music:null,sfx:null,transitions:[],instances:[]});
        this.selectedScene = `scene-${index}`; this.changed(); this.render();
    }

    removeScene() {
        if (!this.design || this.design.scenes.length <= 1) { this.status('Keep at least one scene.',true); return; }
        const removed = this.selectedScene;
        this.design.scenes = this.design.scenes.filter(scene => scene.id !== removed);
        for (const scene of this.design.scenes) scene.transitions = scene.transitions.filter(edge => edge.target !== removed);
        if (this.design.start_scene === removed) this.design.start_scene = this.design.scenes[0].id;
        this.selectedScene = this.design.scenes[0].id; this.changed(); this.render();
    }

    addTransition() {
        const scene = this.design?.scenes.find(item => item.id === this.selectedScene);
        if (!scene) return;
        const trigger = ['start','select','a','b','up','down','left','right','timer'].find(key => !scene.transitions.some(edge => edge.trigger === key));
        if (!trigger) { this.status('This scene already uses every trigger.',true); return; }
        scene.transitions.push({trigger,target:this.design.scenes.find(item => item.id !== scene.id)?.id || scene.id,frames:120});
        this.changed(); this.render();
    }

    async save() {
        if (!this.design) return false;
        this.applyFields(false);
        const project = this.projectName, revision = this.revision;
        const saved = await window.projectManager.saveCurrentProject(true);
        const error = window.projectManager.lastSaveError;
        if (project === this.projectName && revision === this.revision) this.status(saved ? 'Design and assets saved.' : error?.project === project ? error.message : 'Save failed. Resolve the reported reference or source error.',!saved);
        if (saved) await window.projectManager.refreshFiles();
        return saved;
    }

    run() {
        if (this.design) {
            this.applyFields(false);
            window.editor.runEmulator({sourceOverride:StudioStarter.mainSource});
        }
    }

    async inspectCode() {
        this.applyFields(false);
        const manager = window.projectManager, project = this.projectName, revision = this.revision;
        const loadRevision = manager.loadRevision, fileRevision = manager.fileLoadRevision;
        const current = () => manager === window.projectManager && project === this.projectName &&
            project === manager.currentProject && revision === this.revision && loadRevision === manager.loadRevision;
        if (!current() || !await this.save() || !current() || fileRevision !== manager.fileLoadRevision) return;
        // Inspect is an intent to change views; edits/switches during either await supersede it.
        if (await manager.loadFile('game_flow.swiss') && current()) window.dispatchEvent(new CustomEvent('navigate-to',{detail:'code'}));
    }

    async test() {
        this.applyFields(false);
        const project = this.projectName, revision = this.revision;
        if (!await this.save() || project !== this.projectName || revision !== this.revision) return;
        this.status('Compiling and executing 120 frames in the production NES core…');
        try {
            const response = await fetch(`/api/projects/${project}/flow/test`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({frames:120})});
            if (!response.ok) throw new Error(await response.text());
            const report = await response.json();
            if (project !== this.projectName || revision !== this.revision) return;
            this.status(`PASS · ${report.frames} frames · ${report.scene_name} · ${report.rom_bytes.toLocaleString()} ROM bytes · generated flow boot. Controller scenarios are a separate test.`);
        } catch (error) {
            if (project === this.projectName && revision === this.revision) this.status(error.message,true);
        }
    }

    async createStarter() {
        const manager = window.projectManager, name = this.field('new-name').value.trim();
        if (!/^[a-z0-9_-]{1,64}$/.test(name)) { this.status('Use 1–64 lowercase letters, numbers, underscores or hyphens.',true); return; }
        const context = manager.loadRevision;
        if (!await manager.saveCurrentProject(true)) return;
        try {
            const response = await fetch('/api/projects',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({name,...StudioStarter.build()})});
            if (!response.ok) throw new Error(await response.text());
            await manager.refreshProjects();
            // Creating a project must not undo a project switch while the request was in flight.
            if (context === manager.loadRevision) await manager.loadProject(name);
            this.status(`Created ${name}. Press START in preview, B to change rooms, SELECT to return home.`);
        } catch (error) { this.status(error.message,true); }
    }

    currentTest() {
        return this.design?.tests.find(test => test.id === this.selectedTest);
    }

    addTest() {
        if (!this.design || this.design.tests.length >= 32) return;
        let index = 1; while (this.design.tests.some(test => test.id === `test-${index}`)) index++;
        const test = {id:`test-${index}`,name:`Test ${index}`,steps:[{frames:60,buttons:[],expected_scene:this.design.start_scene}]};
        this.design.tests.push(test); this.selectedTest = test.id; this.changed(); this.renderTests();
    }

    addTestStep() {
        const test = this.currentTest();
        if (!test || test.steps.length >= 64) return;
        test.steps.push({frames:1,buttons:[],expected_scene:null,expected_x:null,expected_y:null});
        this.changed(); this.renderTests();
    }

    removeTest() {
        if (!this.currentTest()) return;
        this.design.tests = this.design.tests.filter(test => test.id !== this.selectedTest);
        this.selectedTest = this.design.tests[0]?.id || null;
        this.changed(); this.renderTests();
    }

    renderTests() {
        const tests = this.design.tests;
        if (!tests.some(test => test.id === this.selectedTest)) this.selectedTest = tests[0]?.id || null;
        this.options('tests-select', tests.map(test => ({value:test.id,label:test.name})),this.selectedTest ?? undefined);
        const test = this.currentTest(), container = this.field('tests-steps');
        this.field('tests-name').value = test?.name || '';
        container.innerHTML = '';
        if (!test) { this.field('tests-result').textContent = 'Add a scenario to automate controller input and state assertions.'; return; }
        test.steps.forEach((step,index) => {
            const row = document.createElement('div'); row.className = 'studio-test-step';
            const number = (label,value,update,max) => {
                const wrapper = document.createElement('label'); wrapper.textContent = `Step ${index + 1} ${label}`;
                const input = document.createElement('input'); input.type='number'; input.min=label==='frames' ? 1 : 0; input.max=max; input.value=value ?? '';
                input.addEventListener('input',()=>{update(input.value);this.changed();}); wrapper.appendChild(input); row.appendChild(wrapper);
            };
            number('frames',step.frames,value=>{step.frames=Number(value);},590);
            const wrapper = document.createElement('label'); wrapper.textContent=`Step ${index + 1} expected scene`;
            const expected = document.createElement('select');
            const choices = [{id:'',name:'No scene assertion'},...this.design.scenes];
            if (step.expected_scene && !this.design.scenes.some(scene=>scene.id===step.expected_scene)) choices.push({id:step.expected_scene,name:`Missing scene: ${step.expected_scene}`});
            for (const scene of choices) { const option=document.createElement('option');option.value=scene.id;option.textContent=scene.name;expected.appendChild(option); }
            expected.value=step.expected_scene || ''; expected.onchange=()=>{step.expected_scene=expected.value || null;this.changed();};
            wrapper.appendChild(expected); row.appendChild(wrapper);
            number('expected X',step.expected_x,value=>{step.expected_x=value==='' ? null : Number(value);},65535);
            number('expected Y',step.expected_y,value=>{step.expected_y=value==='' ? null : Number(value);},65535);
            const buttons=document.createElement('div'); buttons.className='studio-test-buttons';
            for (const key of ['a','b','start','select','up','down','left','right']) {
                const label=document.createElement('label'), input=document.createElement('input'); input.type='checkbox';input.checked=(step.buttons || []).includes(key);
                input.setAttribute('aria-label',`Step ${index + 1} hold ${key.toUpperCase()}`);
                input.onchange=()=>{step.buttons=(step.buttons || []).filter(value=>value!==key);if(input.checked)step.buttons.push(key);this.changed();};
                label.appendChild(input);const text=document.createElement('span');text.textContent=key.toUpperCase();label.appendChild(text);buttons.appendChild(label);
            }
            row.appendChild(buttons);
            number('expected pickups',step.expected_pickups,value=>{step.expected_pickups=value==='' ? null : Number(value);},65535);
            number('expected active entities',step.expected_active_entities,value=>{step.expected_active_entities=value==='' ? null : Number(value);},16);
            this.eventEditor.renderAssertions(row,step,index);
            const remove=document.createElement('button');remove.textContent=`Remove step ${index + 1}`;
            remove.onclick=()=>{if(test.steps.length>1){test.steps.splice(index,1);this.changed();this.renderTests();}};
            row.appendChild(remove);container.appendChild(row);
        });
    }

    async runSavedTest() {
        this.applyFields(false);
        const test=this.currentTest(); if(!test)return;
        const project=this.projectName,revision=this.revision,id=test.id,loadRevision=window.projectManager.loadRevision;
        const current=()=>project===this.projectName && project===window.projectManager.currentProject && revision===this.revision && id===this.selectedTest && loadRevision===window.projectManager.loadRevision;
        if(!await this.save() || !current())return;
        const output=this.field('tests-result'); output.textContent=`Running ${test.name}…`;output.dataset.error='false';
        try {
            const response=await fetch(`/api/projects/${project}/flow/test`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({test_id:id})});
            if(!response.ok)throw new Error(await response.text());
            const report=await response.json();if(!current())return;
            const lines=(report.observations || []).map(item=>`Step ${item.step}: ${item.passed ? 'PASS' : 'FAIL'} · ${item.state.scene_name} · X=${item.state.x} Y=${item.state.y} · pickups=${item.state.pickups ?? 0} active=${item.state.active_entities ?? 0}${Object.entries(item.state.variables || {}).map(([id,value])=>` · ${this.design.variables.find(variable=>variable.id===id)?.name || id}=${value}`).join('')} · frame ${item.frames}`);
            output.textContent=`${report.passed ? 'PASS' : 'FAIL'} · ${test.name} · ${report.frames} frames · ${report.cpu_cycles.toLocaleString()} CPU cycles\n${lines.join('\n')}${report.message ? '\n'+report.message : ''}\nScope: ${report.scope}`;
            output.dataset.error=String(!report.passed);
        } catch(error) {if(current()){output.textContent=error.message;output.dataset.error='true';}}
    }
}
window.GameStudio = GameStudio;
