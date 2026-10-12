// Rules remain metadata until the server validates and compiles the ordinary source.
class StudioEvents {
    constructor(studio) {
        this.studio=studio;this.field=studio.field;this.reset();
        this.field('variable-add').onclick=()=>this.addVariable();
        this.field('events-add').onclick=()=>this.addEvent();
        this.field('events-select').onchange=()=>{this.eventId=this.field('events-select').value;this.render();};
        this.field('event-name').addEventListener('input',()=>{
            const event=this.event();if(!event)return;event.name=this.field('event-name').value;studio.changed();this.eventOptions();
        });
        this.field('event-scene').onchange=()=>{const event=this.event();if(event){event.scene=this.field('event-scene').value;studio.changed();this.render();}};
        this.field('event-trigger').onchange=()=>{
            const event=this.event();if(!event)return;const kind=this.field('event-trigger').value;
            event.trigger=kind==='timer' ? {kind,frames:120} : ['pressed','held'].includes(kind) ? {kind,button:'a'} : {kind};
            event.once_per_visit=kind==='timer';studio.changed();this.render();
        };
        this.field('event-button').onchange=()=>{const event=this.event();if(event){event.trigger.button=this.field('event-button').value;studio.changed();}};
        this.field('event-frames').addEventListener('input',()=>{const event=this.event();if(event){event.trigger.frames=Number(this.field('event-frames').value);studio.changed();}});
        this.field('event-once').onchange=()=>{const event=this.event();if(event){event.once_per_visit=this.field('event-once').checked;studio.changed();}};
        this.field('event-remove').onclick=()=>{
            const design=studio.design;if(!this.event())return;design.events=design.events.filter(event=>event.id!==this.eventId);this.eventId=null;this.update();
        };
        this.field('event-up').onclick=()=>this.moveEvent(-1);this.field('event-down').onclick=()=>this.moveEvent(1);
        this.field('condition-add').onclick=()=>{
            const event=this.event();if(!event || event.conditions.length>=8)return;
            const variable=studio.design.variables[0];
            event.conditions.push({source:variable ? {kind:'variable',variable:variable.id} : {kind:'pickups'},comparison:'greater_equal',value:1});this.update();
        };
        this.field('action-add').onclick=()=>{const event=this.event();if(event && event.actions.length<8){event.actions.push(this.defaultAction('set'));this.update();}};
    }

    reset() { this.eventId=null; }
    event() { return this.studio.design?.events.find(event=>event.id===this.eventId); }
    id(prefix,items) { let index=1;while(items.some(item=>item.id===`${prefix}-${index}`))index++;return `${prefix}-${index}`; }
    update() { this.studio.changed();this.render();this.studio.entityEditor.drawPlacement(); }
    static ramBytes(design) {
        const slots=Math.max(0,...design.scenes.map(scene=>StudioEntities.slots(design,scene)));
        const spawners=Math.max(0,...design.scenes.map(scene=>StudioEntities.spawners(design,scene).length));
        return 9+(design.animation ? 5 : 0)+(slots ? 3+slots*9 : 0)+(design.variables?.length || 0)*2+
            (design.events?.length ? 1+(slots ? 0 : 1)+design.events.length : 0)+(design.combat ? 4+slots : 0)+(spawners ? spawners*4+5 : 0)+((design.animation && design.player_visual?.face_movement) || design.combat?.attack_mode==='facing' ? 1 : 0)+(design.combat?.attack_cooldown_frames>0 ? 2 : 0);
    }

    addVariable() {
        const design=this.studio.design;if(!design || design.variables.length>=32)return;
        let index=1;while(design.variables.some(variable=>variable.name.toLowerCase()===`variable${index}`))index++;
        design.variables.push({id:this.id('variable',design.variables),name:`Variable${index}`,initial:0});this.update();this.studio.renderTests();
    }

    removeVariable(id) {
        const design=this.studio.design;
        if(design.events.some(event=>event.conditions.some(condition=>condition.source.kind==='variable' && condition.source.variable===id) ||
            event.actions.some(action=>['set','add','subtract'].includes(action.kind) && action.variable===id))) {
            this.studio.status('Remove this variable’s event references before removing it.',true);return;
        }
        // Saved assertions retain missing IDs so deleting a variable cannot erase coverage.
        design.variables=design.variables.filter(variable=>variable.id!==id);this.update();this.studio.renderTests();
    }

    defaultAction(kind) {
        const design=this.studio.design;
        if(['set','add','subtract'].includes(kind))return design.variables.length ? {kind,variable:design.variables[0].id,value:kind==='set' ? 0 : 1} : {kind,variable:'',value:0};
        if(kind==='scene')return {kind,scene:this.studio.selectedScene};
        if(kind==='music')return {kind,track:0};if(kind==='sfx')return {kind,effect:0};return {kind};
    }

    addEvent() {
        const design=this.studio.design;if(!design || design.events.length>=32)return;
        const event={id:this.id('event',design.events),name:`Event ${design.events.length+1}`,scene:this.studio.selectedScene,
            trigger:{kind:'enter'},once_per_visit:false,conditions:[],actions:[this.defaultAction(design.variables.length ? 'set' : 'stop_music')]};
        design.events.push(event);this.eventId=event.id;this.update();
    }

    moveEvent(direction) {
        const events=this.studio.design.events,index=events.findIndex(event=>event.id===this.eventId),target=index+direction;
        if(index<0 || target<0 || target>=events.length)return;
        [events[index],events[target]]=[events[target],events[index]];this.update();
    }

    eventOptions() {
        const design=this.studio.design;
        this.studio.options('events-select',design.events.map((event,index)=>({value:event.id,label:`${index+1}. ${event.name} · ${design.scenes.find(scene=>scene.id===event.scene)?.name || 'Missing scene'}`})),this.eventId ?? undefined);
    }

    // Labels also identify dynamically authored controls to keyboard and screen-reader users.
    select(row,label,choices,value,change) {
        const wrapper=document.createElement('label');wrapper.textContent=label;
        const input=document.createElement('select');input.setAttribute('aria-label',label);
        if(!choices.some(choice=>String(choice.value)===String(value)))choices=[{value,label:`Missing reference: ${value}`},...choices];
        for(const choice of choices){const option=document.createElement('option');option.value=String(choice.value);option.textContent=choice.label;input.appendChild(option);}
        input.value=String(value);input.onchange=()=>{change(input.value);this.update();};wrapper.appendChild(input);row.appendChild(wrapper);return input;
    }

    number(row,label,value,change,max=65535) {
        const wrapper=document.createElement('label');wrapper.textContent=label;const input=document.createElement('input');
        input.type='number';input.min=0;input.max=max;input.value=value;input.setAttribute('aria-label',label);
        input.addEventListener('input',()=>{change(Number(input.value));this.studio.changed();});wrapper.appendChild(input);row.appendChild(wrapper);return input;
    }

    remove(row,label,action) { const button=document.createElement('button');button.textContent=label;button.onclick=()=>{if(action()!==false)this.update();};row.appendChild(button); }

    refreshVariableLabels(variable) {
        // Rename referenced labels without replacing the variable input while someone types.
        for(const [id,column,value] of [['event-conditions',0,`var:${variable.id}`],['event-actions',1,variable.id]]) {
            for(const [index,row] of Array.from(this.field(id).children).entries()) {
                if(id==='event-actions' && !['set','add','subtract'].includes(this.event()?.actions[index]?.kind))continue;
                for(const option of row.children[column]?.children[0]?.children || []) {
                    if(option.value===value)option.textContent=variable.name;
                }
            }
        }
        this.studio.renderTests();
    }

    render() {
        const design=this.studio.design;if(!design)return;
        const variables=this.field('variables');variables.innerHTML='';
        design.variables.forEach(variable=>{
            const row=document.createElement('div');row.className='studio-rule-row';
            const wrapper=document.createElement('label');wrapper.textContent=`Variable name (${variable.id})`;
            const name=document.createElement('input');name.value=variable.name;name.maxLength=32;name.setAttribute('aria-label',`Variable name (${variable.id})`);
            name.addEventListener('input',()=>{variable.name=name.value;this.studio.changed();this.refreshVariableLabels(variable);});wrapper.appendChild(name);row.appendChild(wrapper);
            this.number(row,`Initial value (${variable.id})`,variable.initial,value=>{variable.initial=value;});
            this.remove(row,`Remove variable ${variable.id}`,()=>{this.removeVariable(variable.id);return false;});variables.appendChild(row);
        });
        if(!this.event())this.eventId=design.events[0]?.id || null;this.eventOptions();
        const event=this.event();this.field('event-name').value=event?.name || '';
        this.studio.options('event-scene',design.scenes.map(scene=>({value:scene.id,label:scene.name})),event?.scene);
        this.field('event-trigger').value=event?.trigger.kind || 'enter';this.field('event-button').value=event?.trigger.button || 'a';
        this.field('event-frames').value=event?.trigger.frames || 120;this.field('event-once').checked=!!event?.once_per_visit;
        this.field('event-button-field').hidden=!['pressed','held'].includes(event?.trigger.kind);
        this.field('event-timer-field').hidden=event?.trigger.kind!=='timer';
        for(const id of ['event-name','event-scene','event-trigger','event-button','event-frames','event-once','event-remove','event-up','event-down','condition-add','action-add'])this.field(id).disabled=!event;
        this.field('event-conditions').innerHTML='';this.field('event-actions').innerHTML='';if(!event)return;
        const vars=design.variables.map(variable=>({value:variable.id,label:variable.name}));
        event.conditions.forEach((condition,index)=>{
            const row=document.createElement('div');row.className='studio-rule-row';const prefix=`Condition ${index+1}`;
            const sources=[...vars.map(variable=>({value:`var:${variable.value}`,label:variable.label})),...['pickups','x','y','ticks','health','defeated'].map(kind=>({value:kind,label:{pickups:'Pickup count',x:'Player X',y:'Player Y',ticks:'Scene timer',health:'Player health',defeated:'Enemies defeated'}[kind]}))];
            const scene=design.scenes.find(scene=>scene.id===event.scene);
            sources.push(...StudioEntities.spawners(design,scene).map(instance=>({value:`spawn:${instance.id}`,label:`${design.entities.find(entity=>entity.id===instance.entity)?.name} (${instance.id}) spawn count`})));
            const value=condition.source.kind==='variable' ? `var:${condition.source.variable}` : condition.source.kind==='spawned' ? `spawn:${condition.source.instance}` : condition.source.kind;
            this.select(row,`${prefix} source`,sources,value,value=>{condition.source=value.startsWith('var:') ? {kind:'variable',variable:value.slice(4)} : value.startsWith('spawn:') ? {kind:'spawned',instance:value.slice(6)} : {kind:value};});
            this.select(row,`${prefix} comparison`,['equal','not_equal','less','less_equal','greater','greater_equal'].map((value,index)=>({value,label:['=','≠','<','≤','>','≥'][index]})),condition.comparison,value=>{condition.comparison=value;});
            this.number(row,`${prefix} value`,condition.value,value=>{condition.value=value;});this.remove(row,`Remove condition ${index+1}`,()=>event.conditions.splice(index,1));this.field('event-conditions').appendChild(row);
        });
        event.actions.forEach((action,index)=>{
            const row=document.createElement('div');row.className='studio-rule-row';const prefix=`Action ${index+1}`;
            this.select(row,`${prefix} kind`,['set','add','subtract','scene','music','sfx','stop_music','restart'].map((value,index)=>({value,label:['Set variable','Add to variable','Subtract from variable','Change scene','Play music','Play SFX','Stop music','Restart game'][index]})),action.kind,value=>{event.actions[index]=this.defaultAction(value);});
            if(['set','add','subtract'].includes(action.kind)){
                this.select(row,`${prefix} variable`,vars,action.variable,value=>{action.variable=value;});this.number(row,`${prefix} value`,action.value,value=>{action.value=value;});
            } else if(action.kind==='scene')this.select(row,`${prefix} destination`,design.scenes.map(scene=>({value:scene.id,label:scene.name})),action.scene,value=>{action.scene=value;});
            else if(['music','sfx'].includes(action.kind)){
                const key=action.kind==='music' ? 'track' : 'effect',assets=this.studio.assets();
                this.select(row,`${prefix} ${key}`,((action.kind==='music' ? assets.audio_tracks : assets.sound_effects) || []).map((item,index)=>({value:index,label:item.name || `${key} ${index}`})),action[key],value=>{action[key]=Number(value);});
            }
            for(const [direction,label] of [[-1,'earlier'],[1,'later']]){
                const button=document.createElement('button');button.textContent=`Move action ${index+1} ${label}`;
                button.disabled=index+direction<0 || index+direction>=event.actions.length;
                button.onclick=()=>{const target=index+direction;[event.actions[index],event.actions[target]]=[event.actions[target],event.actions[index]];this.update();};row.appendChild(button);
            }
            this.remove(row,`Remove action ${index+1}`,()=>event.actions.splice(index,1));this.field('event-actions').appendChild(row);
        });
    }

    renderAssertions(row,step,index) {
        const design=this.studio.design,expectations=step.expected_variables || {};
        const scene=design.scenes.find(scene=>scene.id===(step.expected_scene || this.studio.selectedScene));
        const spawns=step.expected_spawns || {};
        const spawnerIds=[...new Set([...StudioEntities.spawners(design,scene).map(instance=>instance.id),...Object.keys(spawns)])];
        for(const id of spawnerIds) {
            const instance=design.scenes.flatMap(scene=>scene.instances).find(instance=>instance.id===id);
            const entity=design.entities.find(entity=>entity.id===instance?.entity && entity.behavior==='spawner');
            const label=`Step ${index+1} expected spawns ${entity?.name || 'Missing spawner'} (${id})`;
            const wrapper=document.createElement('label');wrapper.textContent=label;const input=document.createElement('input');
            input.type='number';input.min=0;input.max=65535;input.value=spawns[id] ?? '';input.setAttribute('aria-label',label);
            input.addEventListener('input',()=>{step.expected_spawns ||= {};if(input.value==='')delete step.expected_spawns[id];else step.expected_spawns[id]=Number(input.value);this.studio.changed();});wrapper.appendChild(input);row.appendChild(wrapper);
        }
        const ids=[...new Set([...design.variables.map(variable=>variable.id),...Object.keys(expectations)])];
        for(const id of ids){
            const variable=design.variables.find(variable=>variable.id===id),label=`Step ${index+1} expected variable ${variable?.name || `Missing: ${id}`}`;
            const wrapper=document.createElement('label');wrapper.textContent=label;const input=document.createElement('input');
            input.type='number';input.min=0;input.max=65535;input.value=expectations[id] ?? '';input.setAttribute('aria-label',label);
            input.addEventListener('input',()=>{step.expected_variables ||= {};if(input.value==='')delete step.expected_variables[id];else step.expected_variables[id]=Number(input.value);this.studio.changed();});wrapper.appendChild(input);row.appendChild(wrapper);
        }
    }
}
window.StudioEvents=StudioEvents;
