// Original, editable source assets for a title/adventure/return flow. No binary download.
const StudioStarter = {
    mainSource: 'INCLUDE "game_flow.swiss"\n\nSUB Main()\n    GameFlowStart()\n    DO\n        WAIT_VBLANK\n        GameFlowUpdate()\n        \' Add custom game systems here. The flow has already read the controller.\n    LOOP WHILE 1\nEND SUB\n',
    build() {
        const glyphs = {
            A:[14,17,17,31,17,17,17], B:[30,17,17,30,17,17,30], C:[14,17,16,16,16,17,14],
            D:[30,17,17,17,17,17,30], E:[31,16,16,30,16,16,31], F:[31,16,16,30,16,16,16],
            G:[14,17,16,23,17,17,15], H:[17,17,17,31,17,17,17], I:[14,4,4,4,4,4,14],
            J:[7,2,2,2,18,18,12], K:[17,18,20,24,20,18,17], L:[16,16,16,16,16,16,31],
            M:[17,27,21,21,17,17,17], N:[17,25,21,19,17,17,17], O:[14,17,17,17,17,17,14],
            P:[30,17,17,30,16,16,16], Q:[14,17,17,17,21,18,13], R:[30,17,17,30,20,18,17],
            S:[15,16,16,14,1,1,30], T:[31,4,4,4,4,4,4], U:[17,17,17,17,17,17,14],
            V:[17,17,17,17,17,10,4], W:[17,17,17,21,21,21,10], X:[17,17,10,4,10,17,17],
            Y:[17,17,10,4,4,4,4], Z:[31,1,2,4,8,16,31],
            '0':[14,17,19,21,25,17,14], '1':[4,12,4,4,4,4,14], '2':[14,17,1,2,4,8,31],
            '3':[30,1,1,14,1,1,30], '4':[2,6,10,18,31,2,2], '5':[31,16,16,30,1,1,30],
            '6':[14,16,16,30,17,17,14], '7':[31,1,2,4,8,8,8], '8':[14,17,17,14,17,17,14],
            '9':[14,17,17,15,1,1,14], '-':[0,0,0,31,0,0,0], '_':[0,0,0,0,0,0,31],
            '!':[4,4,4,4,4,0,4], '?':[14,17,1,2,4,0,4], '.':[0,0,0,0,0,0,4]
        };
        const chr_bank = Array(8192).fill(0);
        const tile = (id, rows, color, bank) => {
            for (let y = 0; y < 8; y++) {
                chr_bank[bank + id * 16 + y] = color & 1 ? rows[y] || 0 : 0;
                chr_bank[bank + id * 16 + y + 8] = color & 2 ? rows[y] || 0 : 0;
            }
        };
        for (const [letter, rows] of Object.entries(glyphs)) tile(letter.charCodeAt(0), rows.map(row => row << 2), 1, 4096);
        tile(1, [255,129,165,129,165,129,129,255], 2, 4096);
        tile(2, [255,255,255,255,255,255,255,255], 1, 4096);
        const metasprites = [0, 1].map(frame => {
            const tiles = [0,1,2,3].map(index => {
                const id = 128 + frame * 4 + index;
                tile(id, [60,126,126,126,126,126,frame ? 102 : 60,frame ? 102 : 60], index < 2 ? 1 : 2, 0);
                return {x: index % 2 * 8, y: Math.floor(index / 2) * 8, tile: id, attr: 0};
            });
            return {name: `Hero${frame}`, tiles};
        });
        // Original 16x16 pickup and doorway art; all remains editable source data.
        for (const [name,base,rows,color] of [
            ['Coin',136,[0,24,60,126,126,60,24,0],3],
            ['Door',140,[255,129,129,129,129,129,129,129],2]
        ]) {
            metasprites.push({name,tiles:[0,1,2,3].map(index=>{
                tile(base+index,rows,color,0);
                return {x:index%2*8,y:Math.floor(index/2)*8,tile:base+index,attr:0};
            })});
        }
        const text = (data, x, y, value) => [...value].forEach((letter, index) => { data[y * 32 + x + index] = letter.charCodeAt(0); });
        const nametables = ['Title', 'Meadow', 'Ruins'].map((name, index) => {
            const data = Array(960).fill(0);
            for (let x = 0; x < 32; x++) data[29 * 32 + x] = 2;
            if (index) for (let y = 8; y < 22; y++) data[y * 32 + (index === 1 ? 20 : 12)] = 1;
            text(data, 1, 4, index ? 'ARROWS MOVE B NEXT' : 'PRESS START TO PLAY');
            text(data, 1, 6, index ? 'SELECT RETURNS HOME' : 'BUILD YOUR OWN ADVENTURE');
            return {name, data, attrs: Array(64).fill(0), metatile_grid: []};
        });
        const assets = {chr_bank, metasprites, nametables,
            palettes: [{name:'BG0',colors:[15,33,17,48]}, {name:'SP0',colors:[15,48,33,22]}],
            animations: [{name:'HeroWalk', does_loop:true, frames:[0,1].map(frame => ({metasprite:`Hero${frame}`,duration:8}))},
                {name:'CoinIdle',does_loop:true,frames:[{metasprite:'Coin',duration:8}]},
                {name:'DoorIdle',does_loop:true,frames:[{metasprite:'Door',duration:8}]}],
            world: {width:3,height:1,data:[0,1,2]}, metatiles:[], envelopes:[], samples:[],
            audio_tracks: [{name:'Adventure theme',channel:0,instrument:159,priority:1,
                notes:[36,40,43,48].map((pitch,col) => ({pitch,col,row:0,duration:24}))}],
            sound_effects: [{name:'Enter room',channel:1,priority:10,speed:3,does_loop:false,
                vol_sequence:[12,8,0],pitch_sequence:[0],duty_sequence:[1],arpeggio_sequence:[0,4,7]}]
        };
        const scene = (id,name,map_col,movement,transitions) => ({id,name,map_col,map_row:0,
            spawn_x:32,spawn_y:128,movement,music:movement ? 0 : null,sfx:movement ? 0 : null,transitions,instances:[]});
        const design = {version:1,title:'My NES Adventure',core_loop:'Explore rooms, discover obstacles, and build your own game systems.',
            start_scene:'title',animation:'HeroWalk',scenes:[
                scene('title','Title',0,false,[{trigger:'start',target:'meadow',frames:0}]),
                scene('meadow','Meadow',1,true,[{trigger:'b',target:'ruins',frames:0},{trigger:'select',target:'title',frames:0}]),
                scene('ruins','Ruins',2,true,[{trigger:'b',target:'meadow',frames:0},{trigger:'select',target:'title',frames:0}])
            ], entities:[
                {id:'coin',name:'Adventure coin',animation:'CoinIdle',behavior:'pickup'},
                {id:'door',name:'Room doorway',animation:'DoorIdle',behavior:'door'},
                {id:'scout',name:'Patrolling scout',animation:'HeroWalk',behavior:'patrol'}
            ], tests:[
                {id:'enter-play',name:'Enter gameplay',steps:[{frames:2,buttons:['start'],expected_scene:'meadow'}]},
                {id:'round-trip',name:'Room round trip',steps:[
                    {frames:2,buttons:['start'],expected_scene:'meadow'}, {frames:1,buttons:[]},
                    {frames:2,buttons:['b'],expected_scene:'ruins'}, {frames:1,buttons:[]},
                    {frames:2,buttons:['select'],expected_scene:'title'}]},
                {id:'wall-stop',name:'Player stops at the wall',steps:[
                    {frames:2,buttons:['start'],expected_scene:'meadow'}, {frames:1,buttons:[]},
                    {frames:200,buttons:['right'],expected_scene:'meadow',expected_x:144,expected_pickups:1,expected_active_entities:2}]}
            ]};
        design.scenes[1].instances=[
            {id:'meadow-coin',entity:'coin',x:80,y:128},
            {id:'meadow-door',entity:'door',x:128,y:128,target_scene:'ruins'},
            {id:'meadow-scout',entity:'scout',x:48,y:176,patrol_end_x:112}
        ];
        design.scenes[2].instances=[{id:'ruins-door',entity:'door',x:32,y:128,target_scene:'meadow'}];
        return {source: this.mainSource, assets, design};
    }
};
if (typeof window !== 'undefined') window.StudioStarter = StudioStarter;
if (typeof module !== 'undefined') module.exports = StudioStarter;
