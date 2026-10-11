// Authoring stays in project metadata; generated SwissBASIC owns runtime behavior.
class StudioEntities {
    constructor(studio) {
        this.studio = studio; this.field = studio.field; this.reset();
        this.field('entity-add').onclick = () => this.addDefinition();
        this.field('entity-remove').onclick = () => this.removeDefinition();
        this.field('place').onclick = () => this.place();
        this.field('instance-remove').onclick = () => this.removeInstance();
        this.field('entity').onchange = () => {
            this.definitionId = this.field('entity').value; this.field('place-type').value=this.definitionId; this.render();
        };
        this.field('instance').onchange = () => {
            this.instanceId = this.field('instance').value; this.selectInstanceDefinition(); this.render();
        };
        this.field('entity-name').addEventListener('input', () => {
            const entity = this.definition(); if (!entity) return;
            entity.name = this.field('entity-name').value; studio.changed();
            for (const id of ['entity','place-type']) for (const option of this.field(id).children) {
                if (option.value === entity.id) option.textContent = entity.name;
            }
            this.drawPlacement();
        });
        for (const [id,key] of [['entity-animation','animation'],['entity-behavior','behavior']]) {
            this.field(id).onchange = () => {
                const entity = this.definition(); if (!entity) return;
                entity[key] = this.field(id).value;
                // Keep valid existing references; supply initial settings only when absent.
                for (const scene of studio.design.scenes) for (const instance of scene.instances) {
                    if (instance.entity !== entity.id) continue;
                    if (entity.behavior === 'door') instance.target_scene ||= studio.design.scenes.find(item=>item.id!==scene.id)?.id || scene.id;
                    if (entity.behavior === 'patrol') instance.patrol_end_x ??= Math.min(240,instance.x+32);
                }
                studio.changed(); this.render();
            };
        }
        for (const [id,key] of [['instance-x','x'],['instance-y','y'],['instance-end','patrol_end_x']]) {
            this.field(id).addEventListener('input', () => {
                const instance = this.instance(); if (!instance) return;
                instance[key] = Number(this.field(id).value); studio.changed(); this.drawPlacement();
            });
        }
        this.field('instance-target').onchange = () => {
            const instance = this.instance(); if (instance) { instance.target_scene = this.field('instance-target').value; studio.changed(); }
        };
        this.field('placement').onclick = event => {
            const instance = this.instance(); if (!instance) return;
            const bounds = this.field('placement').getBoundingClientRect();
            instance.x = Math.max(0,Math.min(240,Math.floor((event.clientX-bounds.left)/bounds.width*256/8)*8));
            instance.y = Math.max(0,Math.min(224,Math.floor((event.clientY-bounds.top)/bounds.height*240/8)*8));
            studio.changed(); this.render();
        };
    }

    reset() { this.definitionId = null; this.instanceId = null; this.field('place-type').value=''; }
    scene() { return this.studio.design?.scenes.find(scene=>scene.id===this.studio.selectedScene); }
    definition() { return this.studio.design?.entities.find(entity=>entity.id===this.definitionId); }
    instance() { return this.scene()?.instances.find(instance=>instance.id===this.instanceId); }
    selectInstanceDefinition() {
        if (this.instance()) { this.definitionId=this.instance().entity; this.field('place-type').value=this.definitionId; }
    }
    id(prefix,items) { let n=1; while(items.some(item=>item.id===`${prefix}-${n}`))n++; return `${prefix}-${n}`; }

    addDefinition() {
        const design = this.studio.design; if (!design || design.entities.length >= 32) return;
        const animation = this.studio.assets().animations?.[0]?.name;
        if (!animation) { this.studio.status('Create an animation in Sprites before adding an entity.',true); return; }
        const id = this.id('entity',design.entities);
        design.entities.push({id,name:`Entity ${design.entities.length+1}`,animation,behavior:'decoration'});
        this.definitionId=id; this.field('place-type').value=id; this.studio.changed(); this.render();
    }

    removeDefinition() {
        const design=this.studio.design, entity=this.definition(); if (!entity)return;
        if (design.scenes.some(scene=>scene.instances.some(instance=>instance.entity===entity.id))) {
            this.studio.status('Remove this definition’s placed instances before removing the definition.',true); return;
        }
        design.entities=design.entities.filter(item=>item.id!==entity.id); this.definitionId=null;
        this.studio.changed(); this.render();
    }

    place() {
        const scene=this.scene(), design=this.studio.design;
        const entity=design?.entities.find(item=>item.id===this.field('place-type').value);
        if (!scene || !entity) { this.studio.status('Add an entity definition before placing it.',true); return; }
        if (scene.instances.length>=16) { this.studio.status('A scene supports at most 16 instances.',true); return; }
        const id=this.id('instance',design.scenes.flatMap(item=>item.instances));
        const instance={id,entity:entity.id,x:48,y:128,target_scene:null,patrol_end_x:null};
        if(entity.behavior==='door')instance.target_scene=design.scenes.find(item=>item.id!==scene.id)?.id || scene.id;
        if(entity.behavior==='patrol')instance.patrol_end_x=80;
        scene.instances.push(instance);this.instanceId=id;this.selectInstanceDefinition();this.studio.changed();this.render();
    }

    removeInstance() {
        const scene=this.scene();if(!this.instance())return;
        scene.instances=scene.instances.filter(item=>item.id!==this.instanceId);this.instanceId=null;
        this.studio.changed();this.render();
    }

    render() {
        const design=this.studio.design,scene=this.scene();if(!design || !scene)return;
        const entities=design.entities,assets=this.studio.assets();
        if(!this.definition())this.definitionId=entities[0]?.id || null;
        if(!this.instance())this.instanceId=scene.instances[0]?.id || null;
        this.studio.options('entity',entities.map(entity=>({value:entity.id,label:entity.name})),this.definitionId ?? undefined);
        this.studio.options('place-type',entities.map(entity=>({value:entity.id,label:entity.name})),this.field('place-type').value || this.definitionId || undefined);
        const entity=this.definition();this.field('entity-name').value=entity?.name || '';
        this.studio.options('entity-animation',(assets.animations || []).map(item=>({value:item.name,label:item.name})),entity?.animation);
        this.field('entity-behavior').value=entity?.behavior || 'decoration';
        for(const id of ['entity-name','entity-animation','entity-behavior','entity-remove'])this.field(id).disabled=!entity;
        this.studio.options('instance',scene.instances.map((item,index)=>({value:item.id,label:`${index+1}: ${entities.find(entity=>entity.id===item.entity)?.name || 'Missing entity'} (${item.id})`})),this.instanceId ?? undefined);
        const instance=this.instance(),behavior=entities.find(item=>item.id===instance?.entity)?.behavior;
        this.field('instance-x').value=instance?.x ?? '';this.field('instance-y').value=instance?.y ?? '';
        this.field('instance-end').value=instance?.patrol_end_x ?? '';
        this.studio.options('instance-target',design.scenes.map(item=>({value:item.id,label:item.name})),instance?.target_scene || undefined);
        this.field('door-field').hidden=behavior!=='door';this.field('patrol-field').hidden=behavior!=='patrol';
        for(const id of ['instance-x','instance-y','instance-end','instance-target','instance-remove'])this.field(id).disabled=!instance;
        this.drawPlacement();
    }

    drawPlacement() {
        const scene=this.scene(), design=this.studio.design;if(!scene)return;
        const assets=this.studio.assets(), stage=this.field('placement');stage.innerHTML='';
        const canvas=document.createElement('canvas');canvas.width=256;canvas.height=240;canvas.setAttribute('aria-hidden','true');stage.appendChild(canvas);
        this.drawMap(canvas,scene,assets);
        const spawn=document.createElement('span');spawn.className='studio-spawn';spawn.textContent='P';
        spawn.style.left=`${scene.spawn_x/256*100}%`;spawn.style.top=`${scene.spawn_y/240*100}%`;stage.appendChild(spawn);
        scene.instances.forEach((instance,index)=>{
            const entity=design.entities.find(item=>item.id===instance.entity),marker=document.createElement('button');
            marker.className='studio-instance';marker.textContent=String(index+1);marker.dataset.behavior=entity?.behavior;
            marker.dataset.selected=String(instance.id===this.instanceId);
            marker.setAttribute('aria-label',`Select instance ${index+1}: ${entity?.name || 'Missing entity'}`);
            marker.title=`${entity?.name || instance.entity} · ${instance.x}, ${instance.y}`;
            marker.style.left=`${instance.x/256*100}%`;marker.style.top=`${instance.y/240*100}%`;
            marker.onclick=event=>{event.stopPropagation();this.instanceId=instance.id;this.selectInstanceDefinition();this.render();};stage.appendChild(marker);
        });
        const sprites=name=>{
            const animation=assets.animations?.find(item=>item.name===name);
            return animation?.frames.length ? Math.max(...animation.frames.map(frame=>assets.metasprites?.find(item=>item.name===frame.metasprite)?.tiles.length || 0)) : 0;
        };
        const count=(design.animation ? sprites(design.animation) : 0)+scene.instances.reduce((sum,item)=>sum+sprites(design.entities.find(entity=>entity.id===item.entity)?.animation),0);
        const ram=StudioEvents.ramBytes(design);
        this.field('budget').textContent=`${scene.name}: ${scene.instances.length}/16 instances · up to ${count}/64 sprites · generated variables ${ram}/560 bytes. Eight sprites per scanline is a separate hardware limit. Custom code and actual ROM capacity are checked by the compiler.`;
        this.field('budget').dataset.error=String(count>64);
    }

    drawMap(canvas,scene,assets) {
        const ctx=canvas.getContext('2d');if(!ctx)return;
        const world=assets.world,map=assets.nametables?.[world?.data[scene.map_row*world.width+scene.map_col]];
        const pixels=ctx.createImageData(256,240),chr=assets.chr_bank || [];
        const colors=window.paletteEditor?.nesPalette || Array(64).fill('545454');
        const tile=(id,x,y,palette,bank,attr=0,sprite=false)=>{
            for(let py=0;py<8;py++)for(let px=0;px<8;px++){
                const bit=attr&64 ? px : 7-px,row=attr&128 ? 7-py : py;
                const color=((chr[bank+id*16+row]>>bit)&1)|(((chr[bank+id*16+row+8]>>bit)&1)<<1);
                if(sprite && !color)continue;
                const dx=x+px,dy=y+py;if(dx<0 || dx>=256 || dy<0 || dy>=240)continue;
                const rgb=parseInt(colors[(palette?.[color] ?? 15)&63],16),offset=(dy*256+dx)*4;
                // ImageData clamps rather than truncates; isolate each RGB byte first.
                pixels.data[offset]=(rgb>>16)&255;pixels.data[offset+1]=(rgb>>8)&255;pixels.data[offset+2]=rgb&255;pixels.data[offset+3]=255;
            }
        };
        for(let y=0;y<30;y++)for(let x=0;x<32;x++){
            const attr=map?.attrs[Math.floor(y/4)*8+Math.floor(x/4)] || 0;
            const palette=(attr>>((Math.floor(y%4/2)*2+Math.floor(x%4/2))*2))&3;
            tile(map?.data[y*32+x] || 0,x*8,y*8,assets.palettes?.[palette]?.colors,4096);
        }
        for(const instance of scene.instances){
            const entity=this.studio.design.entities.find(item=>item.id===instance.entity);
            const animation=assets.animations?.find(item=>item.name===entity?.animation);
            const sprite=assets.metasprites?.find(item=>item.name===animation?.frames[0]?.metasprite);
            for(const part of sprite?.tiles || []){
                const palettes=(assets.palettes || []).filter(item=>item.name.startsWith('SP'));
                tile(part.tile,instance.x+part.x,instance.y+part.y,palettes[part.attr&3]?.colors,0,part.attr,true);
            }
        }
        ctx.putImageData(pixels,0,0);
    }
}
window.StudioEntities=StudioEntities;
