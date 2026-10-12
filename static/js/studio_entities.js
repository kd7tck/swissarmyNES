// Authoring stays in project metadata; generated SANBasic owns runtime behavior.
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
        for(const owner of ['player','entity']) for(const key of ['x','y','width','height']) {
            this.field(`${owner}-hitbox-${key}`).addEventListener('input',()=>{
                const target=owner==='player' ? studio.design : this.definition();if(!target)return;
                const property=owner==='player' ? 'player_hitbox' : 'hitbox';
                target[property] ||= StudioEntities.defaultHitbox();
                target[property][key]=Number(this.field(`${owner}-hitbox-${key}`).value);
                studio.changed();this.drawPlacement();
            });
        }
        for(const owner of ['player','entity']) {
            this.field(`${owner}-palette`).onchange=()=>{
                const target=owner==='player' ? studio.design : this.definition();if(!target)return;
                const property=owner==='player' ? 'player_visual' : 'visual';target[property] ||= {};
                target[property].palette=this.field(`${owner}-palette`).value==='' ? null : Number(this.field(`${owner}-palette`).value);
                studio.changed();this.render();
            };
            for(const key of ['flip_x','flip_y','face_movement'])this.field(`${owner}-${key}`).onchange=()=>{
                const target=owner==='player' ? studio.design : this.definition();if(!target)return;
                const property=owner==='player' ? 'player_visual' : 'visual';target[property] ||= {};
                target[property][key]=this.field(`${owner}-${key}`).checked;studio.changed();this.drawPlacement();
            };
        }
        this.field('entity-color-variant').onclick=()=>this.colorVariant();
        for(let index=1;index<=3;index++)this.field(`variant-color${index}`).onchange=()=>{
            if(studio.projectName!==window.projectManager?.currentProject)return;
            const palette=this.spritePalette(this.definition()?.visual?.palette);if(!palette)return;
            palette.colors[index]=Number(this.field(`variant-color${index}`).value);studio.changed();this.paletteChanged();
        };
        this.field('combat-enabled').onchange=()=>{
            if(!studio.design)return;
            if(!this.field('combat-enabled').checked && studio.design.entities.some(entity=>['enemy','spawner'].includes(entity.behavior))) {
                this.field('combat-enabled').checked=true;studio.status('Change or remove enemy definitions and spawners before disabling combat.',true);return;
            }
            studio.design.combat=this.field('combat-enabled').checked ? this.defaultCombat() : null;
            studio.changed();this.render();studio.renderTests();
        };
        for(const [id,key] of [['combat-health','health'],['combat-immunity','invulnerability_frames'],['combat-damage','attack_damage'],['combat-reach','attack_reach'],['combat-cooldown','attack_cooldown_frames']]) {
            this.field(id).addEventListener('input',()=>{if(studio.design?.combat){studio.design.combat[key]=Number(this.field(id).value);studio.changed();if(key==='attack_cooldown_frames')this.drawPlacement();}});
        }
        for(const [id,key] of [['combat-button','attack_button'],['combat-death','death_scene'],['combat-mode','attack_mode']]) {
            this.field(id).onchange=()=>{if(studio.design?.combat){studio.design.combat[key]=this.field(id).value;studio.changed();this.drawPlacement();}};
        }
        this.field('enemy-motion').onchange=()=>{
            const entity=this.definition();if(!entity)return;entity.enemy ||= {health:1,damage:1};
            entity.enemy.motion=this.field('enemy-motion').value;studio.changed();this.render();
        };
        this.field('enemy-range').addEventListener('input',()=>{
            const entity=this.definition();if(!entity)return;entity.enemy ||= {health:1,damage:1};
            entity.enemy.detection_range=Number(this.field('enemy-range').value);studio.changed();
        });
        for(const key of ['health','damage']) this.field(`enemy-${key}`).addEventListener('input',()=>{
            const entity=this.definition();if(!entity)return;entity.enemy ||= {health:1,damage:1};
            entity.enemy[key]=Number(this.field(`enemy-${key}`).value);studio.changed();
        });
        this.field('spawner-target').onchange=()=>{
            const entity=this.definition();if(!entity?.spawner)return;entity.spawner.entity=this.field('spawner-target').value;
            studio.changed();this.render();studio.eventEditor.render();studio.renderTests();
        };
        for(const [id,key] of [['spawner-capacity','capacity'],['spawner-interval','interval_frames'],['spawner-delay','initial_delay_frames'],['spawner-total','total']]) {
            this.field(id).addEventListener('input',()=>{const entity=this.definition();if(!entity?.spawner)return;
                entity.spawner[key]=Number(this.field(id).value);studio.changed();this.drawPlacement();
            });
        }
        for (const [id,key] of [['entity-animation','animation'],['entity-behavior','behavior']]) {
            this.field(id).onchange = () => {
                const entity = this.definition(); if (!entity) return;
                entity[key] = this.field(id).value;
                if(['enemy','spawner'].includes(entity.behavior))studio.design.combat ||= this.defaultCombat();
                if(entity.behavior==='spawner')entity.spawner ||= {entity:studio.design.entities.find(item=>item.behavior==='enemy')?.id || '',capacity:2,interval_frames:60,initial_delay_frames:0,total:0};
                // Keep valid existing references; supply initial settings only when absent.
                for (const scene of studio.design.scenes) for (const instance of scene.instances) {
                    if (instance.entity !== entity.id) continue;
                    if (entity.behavior === 'door') instance.target_scene ||= studio.design.scenes.find(item=>item.id!==scene.id)?.id || scene.id;
                    if (entity.behavior === 'patrol') instance.patrol_end_x ??= Math.min(240,instance.x+32);
                }
                studio.changed(); this.render();studio.eventEditor.render();studio.renderTests();
            };
        }
        for (const [id,key] of [['instance-x','x'],['instance-y','y'],['instance-end','patrol_end_x']]) {
            this.field(id).addEventListener('input', () => {
                const instance = this.instance(); if (!instance) return;
                instance[key] = key==='patrol_end_x' && this.field(id).value==='' ? null : Number(this.field(id).value); studio.changed(); this.drawPlacement();
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
    static defaultHitbox() { return {x:0,y:0,width:16,height:16}; }
    static slots(design,scene) {
        return (scene.instances?.length || 0)+(scene.instances || []).reduce((sum,instance)=>{
            const entity=design.entities.find(item=>item.id===instance.entity);
            return sum+(entity?.behavior==='spawner' ? Number(entity.spawner?.capacity || 0) : 0);
        },0);
    }
    static spawners(design,scene) {
        return (scene?.instances || []).filter(instance=>design.entities.find(entity=>entity.id===instance.entity)?.behavior==='spawner');
    }
    drawingDefinition(entity) { return entity?.behavior==='spawner' ? this.studio.design.entities.find(item=>item.id===entity.spawner?.entity) : entity; }
    spritePalette(slot) { return slot==null ? null : this.studio.assets().palettes?.find(palette=>palette.name===`SP${slot}`); }
    paletteChanged() {window.paletteEditor?.render();window.dispatchEvent(new CustomEvent('palette-changed'));this.drawPlacement();}
    colorVariant() {
        if(this.studio.projectName!==window.projectManager?.currentProject)return;
        const design=this.studio.design,entity=this.definition();if(!entity || entity.behavior==='spawner' || design.entities.length>=32)return;
        const assets=this.studio.assets();
        const used=new Set((assets.metasprites || []).flatMap(sprite=>sprite.tiles.map(tile=>tile.attr&3)));
        for(const item of design.entities)if(item.visual?.palette!=null)used.add(item.visual.palette);
        if(design.player_visual?.palette!=null)used.add(design.player_visual.palette);
        const slot=[0,1,2,3].find(slot=>!used.has(slot));
        if(slot==null){this.studio.status('All four sprite palettes are used. Choose an existing palette or free one in Graphics before creating a color variant.',true);return;}
        const animation=assets.animations.find(animation=>animation.name===entity.animation);
        const sprite=assets.metasprites.find(sprite=>sprite.name===animation?.frames[0]?.metasprite);
        const base=this.spritePalette(entity.visual?.palette ?? (sprite?.tiles[0]?.attr&3));
        let palette=this.spritePalette(slot);
        if(!base){this.studio.status('Create the source sprite palette in Graphics before creating a color variant.',true);return;}
        if(!palette){palette={name:`SP${slot}`,colors:base.colors.slice()};window.projectManager.assets.palettes.push(palette);window.paletteEditor?.onProjectLoaded?.(window.projectManager.assets);}
        const clone=JSON.parse(JSON.stringify(entity));clone.id=this.id('entity',design.entities);clone.name=`Color variant ${design.entities.length+1}`;
        clone.visual={...clone.visual,palette:slot};palette.colors=base.colors.slice();
        design.entities.push(clone);this.definitionId=clone.id;this.field('place-type').value=clone.id;
        this.studio.changed();this.paletteChanged();this.render();
        this.studio.status(`Shared ${entity.animation} and its tile patterns. Edit SP${slot} colors below; no new animation frames or CHR tiles were created.`);
    }
    defaultCombat() { return {health:3,invulnerability_frames:30,attack_button:'b',attack_damage:1,attack_reach:8,attack_cooldown_frames:0,death_scene:this.studio.design.scenes[0].id}; }
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
        if(design.entities.some(item=>item.behavior==='spawner' && item.spawner?.entity===entity.id)) {
            this.studio.status('Change the spawner’s enemy reference before removing this definition.',true);return;
        }
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
        scene.instances.push(instance);this.instanceId=id;this.selectInstanceDefinition();this.studio.changed();this.render();this.studio.eventEditor.render();this.studio.renderTests();
    }

    removeInstance() {
        const scene=this.scene();if(!this.instance())return;
        scene.instances=scene.instances.filter(item=>item.id!==this.instanceId);this.instanceId=null;
        this.studio.changed();this.render();this.studio.eventEditor.render();this.studio.renderTests();
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
        for(const owner of ['player','entity']) {
            const box=(owner==='player' ? design.player_hitbox : this.drawingDefinition(entity)?.hitbox) || StudioEntities.defaultHitbox();
            for(const key of ['x','y','width','height']) {
                this.field(`${owner}-hitbox-${key}`).value=box[key];
                this.field(`${owner}-hitbox-${key}`).disabled=owner==='entity' && (!entity || entity.behavior==='spawner');
            }
        }
        this.field('enemy-fields').hidden=entity?.behavior!=='enemy';
        for(const key of ['health','damage'])this.field(`enemy-${key}`).value=entity?.enemy?.[key] ?? 1;
        this.field('enemy-motion').value=entity?.enemy?.motion ?? 'patrol';
        this.field('enemy-range').value=entity?.enemy?.detection_range ?? 64;
        this.field('enemy-range').disabled=entity?.behavior!=='enemy' || entity?.enemy?.motion!=='pursue';
        for(const owner of ['player','entity']) {
            const visual=(owner==='player' ? design.player_visual : this.drawingDefinition(entity)?.visual) || {};
            const disabled=owner==='entity' && (!entity || entity.behavior==='spawner');
            this.field(`${owner}-palette`).value=visual.palette ?? '';this.field(`${owner}-palette`).disabled=disabled;
            for(const key of ['flip_x','flip_y','face_movement']) {this.field(`${owner}-${key}`).checked=!!visual[key];this.field(`${owner}-${key}`).disabled=disabled;}
        }
        this.field('entity-color-variant').disabled=!entity || entity.behavior==='spawner' || design.entities.length>=32;
        const palette=this.spritePalette(entity?.visual?.palette);
        const colors=window.paletteEditor?.nesPalette || [];
        for(let index=1;index<=3;index++) {
            this.studio.options(`variant-color${index}`,Array.from({length:64},(_,color)=>({value:color,label:`$${color.toString(16).padStart(2,'0').toUpperCase()}${colors[color] ? ` #${colors[color]}` : ''}`})),palette?.colors[index]);
            this.field(`variant-color${index}`).disabled=!palette || entity?.behavior==='spawner';
        }
        this.field('spawner-fields').hidden=entity?.behavior!=='spawner';
        this.studio.options('spawner-target',entities.filter(item=>item.behavior==='enemy').map(item=>({value:item.id,label:item.name})),entity?.spawner?.entity);
        for(const [id,key] of [['spawner-capacity','capacity'],['spawner-interval','interval_frames'],['spawner-delay','initial_delay_frames'],['spawner-total','total']])this.field(id).value=entity?.spawner?.[key] ?? '';
        this.field('combat-enabled').checked=!!design.combat;
        this.field('combat-fields').hidden=!design.combat;
        this.field('combat-mode').value=design.combat?.attack_mode ?? 'all_directions';
        this.field('combat-mode').disabled=!design.combat;
        for(const [id,key] of [['combat-health','health'],['combat-immunity','invulnerability_frames'],['combat-damage','attack_damage'],['combat-reach','attack_reach'],['combat-button','attack_button'],['combat-cooldown','attack_cooldown_frames']]) {
            this.field(id).value=design.combat?.[key] ?? '';this.field(id).disabled=!design.combat;
        }
        this.field('combat-cooldown').value=design.combat?.attack_cooldown_frames ?? 0;
        this.studio.options('combat-death',design.scenes.map(item=>({value:item.id,label:item.name})),design.combat?.death_scene);
        for(const id of ['entity-name','entity-animation','entity-behavior','entity-remove'])this.field(id).disabled=!entity;
        this.field('entity-animation').disabled=!entity || entity.behavior==='spawner';
        this.studio.options('instance',scene.instances.map((item,index)=>({value:item.id,label:`${index+1}: ${entities.find(entity=>entity.id===item.entity)?.name || 'Missing entity'} (${item.id})`})),this.instanceId ?? undefined);
        const instance=this.instance(),behavior=entities.find(item=>item.id===instance?.entity)?.behavior;
        this.field('instance-x').value=instance?.x ?? '';this.field('instance-y').value=instance?.y ?? '';
        this.field('instance-end').value=instance?.patrol_end_x ?? '';
        this.studio.options('instance-target',design.scenes.map(item=>({value:item.id,label:item.name})),instance?.target_scene || undefined);
        this.field('door-field').hidden=behavior!=='door';this.field('patrol-field').hidden=!['patrol','enemy','spawner'].includes(behavior);
        for(const id of ['instance-x','instance-y','instance-end','instance-target','instance-remove'])this.field(id).disabled=!instance;
        this.drawPlacement();
    }

    drawPlacement() {
        const scene=this.scene(), design=this.studio.design;if(!scene)return;
        const assets=this.studio.assets(), stage=this.field('placement');stage.innerHTML='';
        const canvas=document.createElement('canvas');canvas.width=256;canvas.height=240;canvas.setAttribute('aria-hidden','true');stage.appendChild(canvas);
        this.drawMap(canvas,scene,assets);
        const spawn=document.createElement('span');spawn.className='studio-spawn';spawn.textContent='P';
        const position=(marker,x,y,box)=>{
            marker.style.left=`${(x+box.x)/256*100}%`;marker.style.top=`${(y+box.y)/240*100}%`;
            marker.style.width=`${box.width/256*100}%`;marker.style.height=`${box.height/240*100}%`;
        };
        position(spawn,scene.spawn_x,scene.spawn_y,design.player_hitbox || StudioEntities.defaultHitbox());stage.appendChild(spawn);
        scene.instances.forEach((instance,index)=>{
            const entity=design.entities.find(item=>item.id===instance.entity),marker=document.createElement('button');
            marker.className='studio-instance';marker.textContent=String(index+1);marker.dataset.behavior=entity?.behavior;
            marker.dataset.selected=String(instance.id===this.instanceId);
            marker.setAttribute('aria-label',`Select instance ${index+1}: ${entity?.name || 'Missing entity'}`);
            marker.title=`${entity?.name || instance.entity} · ${instance.x}, ${instance.y}`;
            position(marker,instance.x,instance.y,this.drawingDefinition(entity)?.hitbox || StudioEntities.defaultHitbox());
            marker.onclick=event=>{event.stopPropagation();this.instanceId=instance.id;this.selectInstanceDefinition();this.render();};stage.appendChild(marker);
        });
        const sprites=name=>{
            const animation=assets.animations?.find(item=>item.name===name);
            return animation?.frames.length ? Math.max(...animation.frames.map(frame=>assets.metasprites?.find(item=>item.name===frame.metasprite)?.tiles.length || 0)) : 0;
        };
        const count=(design.animation ? sprites(design.animation) : 0)+scene.instances.reduce((sum,item)=>{
            const entity=design.entities.find(entity=>entity.id===item.entity);
            return sum+sprites(this.drawingDefinition(entity)?.animation)*(entity?.behavior==='spawner' ? Number(entity.spawner?.capacity || 0) : 1);
        },0);
        const slots=StudioEntities.slots(design,scene);
        const ram=StudioEvents.ramBytes(design);
        this.field('budget').textContent=`${scene.name}: ${scene.instances.length}/16 instances · ${slots}/16 runtime slots including spawn pools · up to ${count}/64 sprites · generated variables ${ram}/560 bytes. Eight sprites per scanline is a separate hardware limit. Custom code and actual ROM capacity are checked by the compiler.`;
        this.field('budget').dataset.error=String(count>64 || slots>16 || ram>560);
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
            const entity=this.drawingDefinition(this.studio.design.entities.find(item=>item.id===instance.entity));
            const animation=assets.animations?.find(item=>item.name===entity?.animation);
            const sprite=assets.metasprites?.find(item=>item.name===animation?.frames[0]?.metasprite);
            const visual=entity?.visual || {};
            const xs=(sprite?.tiles || []).map(part=>part.x),ys=(sprite?.tiles || []).map(part=>part.y);
            const pivotX=xs.length ? Math.min(...xs)+Math.max(...xs) : 0,pivotY=ys.length ? Math.min(...ys)+Math.max(...ys) : 0;
            for(const part of sprite?.tiles || []){
                const palettes=(assets.palettes || []).filter(item=>item.name.startsWith('SP'));
                const attr=(visual.palette!=null ? (part.attr&252)|visual.palette : part.attr)^(visual.flip_x ? 64 : 0)^(visual.flip_y ? 128 : 0);
                tile(part.tile,instance.x+(visual.flip_x ? pivotX-part.x : part.x),instance.y+(visual.flip_y ? pivotY-part.y : part.y),palettes[attr&3]?.colors,0,attr,true);
            }
        }
        ctx.putImageData(pixels,0,0);
    }
}
window.StudioEntities=StudioEntities;
