class SpriteEditor {
    constructor() {
        this.container = document.getElementById('sprite-editor');
        this.metasprites = [];
        this.animations = [];
        this.currentMetasprite = null;
        this.currentAnimation = null;
        this.selectedTileIndex = 0;
        this.selectedAttr = 0;
        this.zoom = 2;

        this.initUI();
        this.bindEvents();
    }

    initUI() {
        this.container.innerHTML = `
            <div class="editor-layout">
                <div class="sidebar">
                    <div class="panel-section">
                        <h3>Metasprites</h3>
                        <div class="list-controls">
                            <button id="btn-add-metasprite">New</button>
                            <button id="btn-del-metasprite" class="danger">Del</button>
                        </div>
                        <ul id="metasprite-list" class="item-list"></ul>
                    </div>
                    <div class="panel-section">
                        <h3>Animations</h3>
                        <div class="list-controls">
                            <button id="btn-add-animation">New</button>
                            <button id="btn-del-animation" class="danger">Del</button>
                        </div>
                        <ul id="animation-list" class="item-list"></ul>
                    </div>
                </div>

                <div class="main-area">
                    <div id="metasprite-editor-view" style="display:none; height: 100%; display: flex; flex-direction: column;">
                        <div class="toolbar">
                            <label>Name: <input type="text" id="metasprite-name"></label>
                            <label>Palette:
                                <select id="sprite-attr-select">
                                    <option value="0">Pal 0</option>
                                    <option value="1">Pal 1</option>
                                    <option value="2">Pal 2</option>
                                    <option value="3">Pal 3</option>
                                </select>
                            </label>
                            <span class="info">Click to place tile. Right-click to remove.</span>
                        </div>
                        <div class="canvas-wrapper" style="flex:1; background: #333; overflow: auto; position: relative;">
                            <canvas id="metasprite-canvas" width="512" height="512" style="cursor: crosshair;"></canvas>
                        </div>
                    </div>

                    <div id="animation-editor-view" style="display:none; height: 100%; flex-direction: column;">
                        <div class="toolbar">
                            <label>Name: <input type="text" id="anim-name"></label>
                            <label>Loop: <input type="checkbox" id="anim-loop"></label>
                            <button id="btn-play-anim">Play</button>
                        </div>
                        <div class="timeline-area" style="flex: 1; padding: 10px;">
                            <h4>Frames</h4>
                            <ul id="anim-frame-list" class="frame-list"></ul>
                            <div class="frame-controls">
                                <label>Metasprite: <select id="anim-frame-meta"></select></label>
                                <label>Duration: <input type="number" id="anim-frame-dur" min="1" max="255" value="10"></label>
                                <button id="btn-add-frame">Add Frame</button>
                                <button id="btn-update-frame">Update</button>
                                <button id="btn-del-frame" class="danger">Remove</button>
                            </div>
                            <div class="preview-area" style="margin-top: 20px; border: 1px solid #555; width: 256px; height: 256px; background: #000; position: relative;">
                                <canvas id="anim-preview-canvas" width="256" height="256"></canvas>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="sidebar right-sidebar">
                    <h3>CHR Bank</h3>
                    <canvas id="sprite-chr-picker" width="128" height="128" style="border: 1px solid #555; cursor: pointer;"></canvas>
                    <div id="selected-tile-preview">Selected: 00</div>
                </div>
            </div>
        `;
    }

    bindEvents() {
        document.getElementById('btn-add-metasprite').onclick = () => this.addMetasprite();
        document.getElementById('btn-del-metasprite').onclick = () => this.deleteMetasprite();
        document.getElementById('metasprite-name').onchange = (e) => this.renameMetasprite(e.target.value);
        document.getElementById('sprite-attr-select').onchange = (e) => this.selectedAttr = parseInt(e.target.value);

        document.getElementById('btn-add-animation').onclick = () => this.addAnimation();
        document.getElementById('btn-del-animation').onclick = () => this.deleteAnimation();
        document.getElementById('anim-name').onchange = (e) => this.renameAnimation(e.target.value);
        document.getElementById('anim-loop').onchange = (e) => {
            if(this.currentAnimation) this.currentAnimation.does_loop = e.target.checked;
        };

        const msCanvas = document.getElementById('metasprite-canvas');
        msCanvas.onmousedown = (e) => this.handleCanvasClick(e);
        msCanvas.oncontextmenu = (e) => { e.preventDefault(); this.handleCanvasClick(e, true); };

        const chrCanvas = document.getElementById('sprite-chr-picker');
        chrCanvas.onclick = (e) => this.handleChrPick(e);

        document.getElementById('btn-add-frame').onclick = () => this.addFrame();
        document.getElementById('btn-del-frame').onclick = () => this.deleteFrame();
        document.getElementById('btn-update-frame').onclick = () => this.updateFrame();
        document.getElementById('btn-play-anim').onclick = () => this.playAnimation();

        // Listen for global events
        window.addEventListener('chr-changed', () => this.renderChrPicker());
        window.addEventListener('palette-changed', () => {
            this.renderChrPicker();
            this.renderMetasprite();
        });
    }

    loadData(assets) {
        this.metasprites = assets.metasprites || [];
        this.animations = assets.animations || [];

        // Ensure valid data structures
        this.metasprites.forEach(ms => {
            if(!ms.tiles) ms.tiles = [];
        });
        this.animations.forEach(anim => {
            if(!anim.frames) anim.frames = [];
            if(anim.does_loop === undefined) anim.does_loop = true;
        });

        this.renderMetaspriteList();
        this.renderAnimationList();
        this.renderChrPicker();

        if(this.metasprites.length > 0) this.selectMetasprite(this.metasprites[0]);
    }

    getData() {
        return {
            metasprites: this.metasprites,
            animations: this.animations
        };
    }

    renderMetaspriteList() {
        const list = document.getElementById('metasprite-list');
        list.innerHTML = '';
        this.metasprites.forEach(ms => {
            const li = document.createElement('li');
            li.textContent = ms.name;
            li.onclick = () => this.selectMetasprite(ms);
            if(this.currentMetasprite === ms) li.classList.add('selected');
            list.appendChild(li);
        });
    }

    renderAnimationList() {
        const list = document.getElementById('animation-list');
        list.innerHTML = '';
        this.animations.forEach(anim => {
            const li = document.createElement('li');
            li.textContent = anim.name;
            li.onclick = () => this.selectAnimation(anim);
            if(this.currentAnimation === anim) li.classList.add('selected');
            list.appendChild(li);
        });
    }

    selectMetasprite(ms) {
        this.currentMetasprite = ms;
        this.currentAnimation = null;

        document.getElementById('metasprite-editor-view').style.display = 'flex';
        document.getElementById('animation-editor-view').style.display = 'none';

        document.getElementById('metasprite-name').value = ms.name;

        this.renderMetaspriteList();
        this.renderAnimationList(); // To clear selection
        this.renderMetasprite();
    }

    selectAnimation(anim) {
        this.currentAnimation = anim;
        this.currentMetasprite = null;

        document.getElementById('metasprite-editor-view').style.display = 'none';
        document.getElementById('animation-editor-view').style.display = 'flex';

        document.getElementById('anim-name').value = anim.name;
        document.getElementById('anim-loop').checked = anim.does_loop;

        // Populate Metasprite Select
        const sel = document.getElementById('anim-frame-meta');
        sel.innerHTML = '';
        this.metasprites.forEach(ms => {
            const opt = document.createElement('option');
            opt.value = ms.name;
            opt.textContent = ms.name;
            sel.appendChild(opt);
        });

        this.renderMetaspriteList(); // To clear selection
        this.renderAnimationList();
        this.renderFrameList();
    }

    addMetasprite() {
        const name = prompt("Metasprite Name:", "NewSprite");
        if(name) {
            const newMs = { name: name, tiles: [] };
            this.metasprites.push(newMs);
            this.selectMetasprite(newMs);
        }
    }

    deleteMetasprite() {
        if(this.currentMetasprite && confirm("Delete Metasprite?")) {
            const idx = this.metasprites.indexOf(this.currentMetasprite);
            this.metasprites.splice(idx, 1);
            this.currentMetasprite = null;
            this.renderMetaspriteList();
            document.getElementById('metasprite-editor-view').style.display = 'none';
        }
    }

    renameMetasprite(name) {
        if(this.currentMetasprite) {
            this.currentMetasprite.name = name;
            this.renderMetaspriteList();
        }
    }

    handleChrPick(e) {
        const rect = e.target.getBoundingClientRect();
        const x = Math.floor((e.clientX - rect.left) / 8); // 8px tiles, scale 1 (image is 128x128)
        const y = Math.floor((e.clientY - rect.top) / 8);
        if(x >= 0 && x < 16 && y >= 0 && y < 16) {
            this.selectedTileIndex = y * 16 + x;
            document.getElementById('selected-tile-preview').textContent = `Selected: $${this.selectedTileIndex.toString(16).toUpperCase().padStart(2,'0')}`;
            this.renderChrPicker(); // To show selection
        }
    }

    renderChrPicker() {
        const canvas = document.getElementById('sprite-chr-picker');
        const ctx = canvas.getContext('2d');
        const img = window.chrEditor ? window.chrEditor.getCanvas() : null; // Hack to get CHR image from other editor

        ctx.fillStyle = '#000';
        ctx.fillRect(0,0,128,128);

        if(img) {
            ctx.drawImage(img, 0, 0);
        }

        // Draw selection
        const x = (this.selectedTileIndex % 16) * 8;
        const y = Math.floor(this.selectedTileIndex / 16) * 8;
        ctx.strokeStyle = 'red';
        ctx.lineWidth = 1;
        ctx.strokeRect(x, y, 8, 8);
    }

    handleCanvasClick(e, isRightClick = false) {
        if(!this.currentMetasprite) return;

        const canvas = document.getElementById('metasprite-canvas');
        const rect = canvas.getBoundingClientRect();
        const scale = 2; // We are drawing scaled
        const cx = (e.clientX - rect.left);
        const cy = (e.clientY - rect.top);

        // Canvas center is 256, 256
        // Relative coordinates: (cx - 256) / scale, (cy - 256) / scale
        // Snap to 8 grid

        // Let's assume grid is aligned to center.
        // Center is (0,0).
        // Mouse X in canvas space is cx.
        // Effective Canvas is 512x512. Center is 256,256.
        // Rel X = (cx - 256) / this.zoom

        const zoom = 2; // Drawing zoom
        // Visual click pos
        const rawX = (cx / (rect.width/512)); // map to canvas pixel space
        const rawY = (cy / (rect.height/512));

        // Center is 256,256
        const relX = Math.floor((rawX - 256) / (8 * zoom)) * 8;
        const relY = Math.floor((rawY - 256) / (8 * zoom)) * 8;

        // relX, relY are now snapped to 8px grid, relative to center.
        // Example: Center click -> 0,0.
        // Click 16px right -> 16,0.

        if (relX < -128 || relX > 127 || relY < -128 || relY > 127) return;

        if (isRightClick) {
            // Remove tile at this position
            this.currentMetasprite.tiles = this.currentMetasprite.tiles.filter(t => t.x !== relX || t.y !== relY);
        } else {
            // Add or update tile
            // Remove existing at same spot first
            this.currentMetasprite.tiles = this.currentMetasprite.tiles.filter(t => t.x !== relX || t.y !== relY);
            this.currentMetasprite.tiles.push({
                x: relX,
                y: relY,
                tile: this.selectedTileIndex,
                attr: this.selectedAttr
            });
        }
        this.renderMetasprite();
    }

    renderMetasprite() {
        if(!this.currentMetasprite) return;
        const canvas = document.getElementById('metasprite-canvas');
        const ctx = canvas.getContext('2d');
        const cx = 256;
        const cy = 256;
        const zoom = 2;

        ctx.fillStyle = '#222';
        ctx.fillRect(0,0,512,512);

        // Draw Crosshair
        ctx.strokeStyle = '#444';
        ctx.beginPath();
        ctx.moveTo(0, cy); ctx.lineTo(512, cy);
        ctx.moveTo(cx, 0); ctx.lineTo(cx, 512);
        ctx.stroke();

        // Draw Tiles
        // We need the CHR bank.
        // We can get tile data from window.chrEditor.chr_bank if available, or ask app.
        // Assuming window.chrEditor.renderTileToCanvas exists or we duplicate logic.
        // `renderTile` logic is reusable.

        const palette = this.getPalette(this.selectedAttr); // For preview, use selected attr or sprite's attr

        this.currentMetasprite.tiles.forEach(tile => {
            const screenX = cx + (tile.x * zoom);
            const screenY = cy + (tile.y * zoom);

            this.drawTile(ctx, tile.tile, screenX, screenY, zoom, tile.attr);
        });
    }

    getPalette(attrIdx) {
        // Get Sprite Palettes from project
        if(window.projectManager && window.projectManager.assets) {
            // Palettes 4-7 are sprites
            const palIdx = 4 + attrIdx;
            const palettes = window.projectManager.assets.palettes;
            if(palettes && palettes[palIdx]) {
                // Convert NES colors to RGB
                return palettes[palIdx].colors.map(c => window.paletteEditor ? window.paletteEditor.getHexColor(c) : '#FFF');
            }
        }
        return ['#00000000', '#F00', '#0F0', '#00F']; // Default debug
    }

    drawTile(ctx, tileIdx, x, y, scale, attr) {
        // This duplicates some logic from MapEditor/CHREditor but keeps it self contained for now
        // Ideally we have a shared Renderer service
        const bank = window.projectManager.assets.chr_bank;
        if(!bank) return;

        const offset = tileIdx * 16;
        const palette = this.getPalette(attr);

        for(let row=0; row<8; row++) {
            let p1 = bank[offset + row];
            let p2 = bank[offset + row + 8];
            for(let col=0; col<8; col++) {
                const bit = 7 - col;
                const lo = (p1 >> bit) & 1;
                const hi = (p2 >> bit) & 1;
                const val = (hi << 1) | lo;

                if(val !== 0) { // Transparent
                    ctx.fillStyle = palette[val];
                    ctx.fillRect(x + col * scale, y + row * scale, scale, scale);
                }
            }
        }
    }

    // Animation Logic
    addAnimation() {
        const name = prompt("Animation Name:", "NewAnim");
        if(name) {
            const newAnim = { name: name, frames: [], does_loop: true };
            this.animations.push(newAnim);
            this.selectAnimation(newAnim);
        }
    }

    deleteAnimation() {
        if(this.currentAnimation && confirm("Delete Animation?")) {
            const idx = this.animations.indexOf(this.currentAnimation);
            this.animations.splice(idx, 1);
            this.currentAnimation = null;
            this.renderAnimationList();
            document.getElementById('animation-editor-view').style.display = 'none';
        }
    }

    renameAnimation(name) {
        if(this.currentAnimation) {
            this.currentAnimation.name = name;
            this.renderAnimationList();
        }
    }

    renderFrameList() {
        const list = document.getElementById('anim-frame-list');
        list.innerHTML = '';
        if(!this.currentAnimation) return;

        this.currentAnimation.frames.forEach((frame, idx) => {
            const li = document.createElement('li');
            li.textContent = `${idx}: ${frame.metasprite} (${frame.duration})`;
            li.onclick = () => {
                this.selectedFrameIndex = idx;
                // Highlight
                Array.from(list.children).forEach(c => c.classList.remove('active'));
                li.classList.add('active');
                // Load into controls
                document.getElementById('anim-frame-meta').value = frame.metasprite;
                document.getElementById('anim-frame-dur').value = frame.duration;
            };
            list.appendChild(li);
        });
    }

    addFrame() {
        if(!this.currentAnimation) return;
        const meta = document.getElementById('anim-frame-meta').value;
        const dur = parseInt(document.getElementById('anim-frame-dur').value);
        if(!meta) { alert("Select a metasprite first"); return; }

        this.currentAnimation.frames.push({ metasprite: meta, duration: dur });
        this.renderFrameList();
    }

    updateFrame() {
        if(!this.currentAnimation || this.selectedFrameIndex === undefined) return;
        const meta = document.getElementById('anim-frame-meta').value;
        const dur = parseInt(document.getElementById('anim-frame-dur').value);

        this.currentAnimation.frames[this.selectedFrameIndex] = { metasprite: meta, duration: dur };
        this.renderFrameList();
    }

    deleteFrame() {
        if(!this.currentAnimation || this.selectedFrameIndex === undefined) return;
        this.currentAnimation.frames.splice(this.selectedFrameIndex, 1);
        this.selectedFrameIndex = undefined;
        this.renderFrameList();
    }

    playAnimation() {
        if(!this.currentAnimation || this.currentAnimation.frames.length === 0) return;

        let frameIdx = 0;
        let timer = 0;
        const canvas = document.getElementById('anim-preview-canvas');
        const ctx = canvas.getContext('2d');
        const cx = 128;
        const cy = 128;
        const zoom = 2;

        const loop = () => {
            // Draw current frame
            const frame = this.currentAnimation.frames[frameIdx];
            const ms = this.metasprites.find(m => m.name === frame.metasprite);

            // Clear
            ctx.fillStyle = '#000';
            ctx.fillRect(0,0,256,256);

            // Draw Metasprite
            if(ms) {
                // Reuse drawTile logic logic by mocking currentMetasprite?
                // Or extracting logic. I'll define a drawMetasprite helper.
                ms.tiles.forEach(tile => {
                    const screenX = cx + (tile.x * zoom);
                    const screenY = cy + (tile.y * zoom);
                    this.drawTile(ctx, tile.tile, screenX, screenY, zoom, tile.attr);
                });
            }

            timer++;
            if(timer >= frame.duration) {
                timer = 0;
                frameIdx++;
                if(frameIdx >= this.currentAnimation.frames.length) {
                    if(this.currentAnimation.does_loop) {
                        frameIdx = 0;
                    } else {
                        return; // Stop
                    }
                }
            }
            requestAnimationFrame(loop);
        };
        loop();
    }
}
