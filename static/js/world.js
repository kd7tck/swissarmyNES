class WorldEditor {
    constructor() {
        this.container = document.getElementById('world-editor-root');
        this.canvas = null;
        this.ctx = null;
        this.assets = null;
        this.world = null;

        this.scale = 0.25; // 64x60 pixels per map
        this.mapWidth = 256;
        this.mapHeight = 240;

        this.selectedNametableIndex = -1; // -1 means "erase" mode

        this.mapThumbnails = new Map(); // cache: index -> HTMLCanvasElement

        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));
        window.addEventListener('palette-changed', () => this.invalidateAll());
        window.addEventListener('chr-changed', () => this.invalidateAll());
        window.addEventListener('nametable-changed', (e) => this.invalidateMap(e.detail.index));
        window.addEventListener('metatile-changed', () => this.invalidateAll());

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';
        this.container.className = 'world-container';

        // Layout: Sidebar (List of maps), Main (Canvas)
        const layout = document.createElement('div');
        layout.className = 'world-layout';

        // Sidebar
        const sidebar = document.createElement('div');
        sidebar.className = 'world-sidebar';

        const header = document.createElement('div');
        header.style.padding = '10px';
        header.innerHTML = '<h3>World Maps</h3><p style="font-size: 12px; color: #888;">Select a map to paint on the grid.</p>';
        sidebar.appendChild(header);

        this.mapList = document.createElement('ul');
        this.mapList.className = 'map-list';
        sidebar.appendChild(this.mapList);

        layout.appendChild(sidebar);

        // Canvas Area
        const wrapper = document.createElement('div');
        wrapper.className = 'world-canvas-wrapper';

        this.canvas = document.createElement('canvas');
        this.canvas.className = 'world-canvas';
        this.ctx = this.canvas.getContext('2d');
        this.ctx.imageSmoothingEnabled = false;

        wrapper.appendChild(this.canvas);
        layout.appendChild(wrapper);

        this.container.appendChild(layout);

        // Events
        this.canvas.addEventListener('mousedown', (e) => this.handleMouse(e));
        this.canvas.addEventListener('mousemove', (e) => {
            if (e.buttons === 1) this.handleMouse(e);
        });

        // Prevent context menu on canvas
        this.canvas.addEventListener('contextmenu', e => e.preventDefault());
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.world) {
            this.assets.world = {
                width: 8,
                height: 8,
                data: new Array(64).fill(-1)
            };
        }
        this.world = this.assets.world;

        // Validation
        const size = this.world.width * this.world.height;
        if (this.world.data.length < size) {
            const oldData = this.world.data;
            this.world.data = new Array(size).fill(-1);
            for(let i=0; i<oldData.length && i<size; i++) {
                this.world.data[i] = oldData[i];
            }
        }

        // Setup Canvas Size
        this.canvas.width = this.world.width * this.mapWidth * this.scale;
        this.canvas.height = this.world.height * this.mapHeight * this.scale;

        this.invalidateAll();
        this.renderMapList();
        this.render();
    }

    renderMapList() {
        this.mapList.innerHTML = '';

        // Eraser
        const liEmpty = document.createElement('li');
        liEmpty.textContent = '(Eraser)';
        liEmpty.onclick = () => {
            this.selectedNametableIndex = -1;
            this.highlightMapList();
        };
        this.mapList.appendChild(liEmpty);

        if (this.assets && this.assets.nametables) {
            this.assets.nametables.forEach((nt, idx) => {
                const li = document.createElement('li');
                li.textContent = nt.name || `Nametable ${idx}`;
                li.onclick = () => {
                    this.selectedNametableIndex = idx;
                    this.highlightMapList();
                };
                this.mapList.appendChild(li);
            });
        }
        this.highlightMapList();
    }

    highlightMapList() {
        const items = this.mapList.querySelectorAll('li');
        items.forEach((li, idx) => {
            const isSelected = (idx === 0 && this.selectedNametableIndex === -1) ||
                               (idx > 0 && this.selectedNametableIndex === idx - 1);
            if (isSelected) li.classList.add('active');
            else li.classList.remove('active');
        });
    }

    invalidateAll() {
        this.mapThumbnails.clear();
        this.render();
    }

    invalidateMap(index) {
        this.mapThumbnails.delete(index);
        this.render();
    }

    handleMouse(e) {
        if (!this.world) return;

        const rect = this.canvas.getBoundingClientRect();
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;

        const cw = this.mapWidth * this.scale;
        const ch = this.mapHeight * this.scale;

        const gx = Math.floor(mx / cw);
        const gy = Math.floor(my / ch);

        if (gx >= 0 && gx < this.world.width && gy >= 0 && gy < this.world.height) {
            const idx = gy * this.world.width + gx;
            if (this.world.data[idx] !== this.selectedNametableIndex) {
                this.world.data[idx] = this.selectedNametableIndex;
                this.render();
            }
        }
    }

    getMapThumbnail(index) {
        if (index < 0) return null;
        if (this.mapThumbnails.has(index)) {
            return this.mapThumbnails.get(index);
        }

        const offCanvas = document.createElement('canvas');
        offCanvas.width = this.mapWidth;
        offCanvas.height = this.mapHeight;
        const ctx = offCanvas.getContext('2d');

        const nt = this.assets.nametables[index];
        if (!nt) {
            // Invalid index?
            return null;
        }

        const chrBank = this.assets.chr_bank;

        // Build palette
        const subPalettes = [];
        if (window.paletteEditor && this.assets.palettes) {
             for(let i=0; i<4; i++) {
                 let colors = ['#000000', '#666666', '#aaaaaa', '#ffffff'];
                 if (this.assets.palettes && this.assets.palettes[i]) {
                     colors = this.assets.palettes[i].colors.map(c => {
                        if (window.paletteEditor && window.paletteEditor.nesPalette) {
                            return '#' + window.paletteEditor.nesPalette[c & 0x3F];
                        }
                        return '#fff';
                     });
                 }
                 subPalettes.push(colors);
             }
        } else {
             for(let i=0; i<4; i++) subPalettes.push(['#000', '#555', '#aaa', '#fff']);
        }

        // Draw Tiles
        // We iterate 32x30
        for (let r = 0; r < 30; r++) {
            for (let c = 0; c < 32; c++) {
                const tileIdx = nt.data[r * 32 + c];

                // Attribute
                const attrX = Math.floor(c / 4);
                const attrY = Math.floor(r / 4);
                const attrIdx = attrY * 8 + attrX;
                let palIdx = 0;
                if (nt.attrs && attrIdx < nt.attrs.length) {
                    const byte = nt.attrs[attrIdx];
                    const isRight = (c % 4) >= 2;
                    const isBottom = (r % 4) >= 2;
                    let shift = 0;
                    if (isRight) shift += 2;
                    if (isBottom) shift += 4;
                    palIdx = (byte >> shift) & 0x03;
                }

                this.drawTileToCtx(ctx, c * 8, r * 8, tileIdx, chrBank, subPalettes[palIdx]);
            }
        }

        this.mapThumbnails.set(index, offCanvas);
        return offCanvas;
    }

    drawTileToCtx(ctx, x, y, tileIdx, chrBank, palette) {
        if (!chrBank) return;
        const tileOffset = tileIdx * 16;
        for (let py = 0; py < 8; py++) {
            const lowByte = chrBank[tileOffset + py];
            const highByte = chrBank[tileOffset + py + 8];
            for (let px = 0; px < 8; px++) {
                const bitMask = 1 << (7 - px);
                const bit0 = (lowByte & bitMask) ? 1 : 0;
                const bit1 = (highByte & bitMask) ? 1 : 0;
                const colorVal = bit0 + (bit1 << 1);
                ctx.fillStyle = palette[colorVal];
                ctx.fillRect(x + px, y + py, 1, 1);
            }
        }
    }

    render() {
        if (!this.ctx || !this.world) return;

        // Clear
        this.ctx.fillStyle = '#111';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        const cw = this.mapWidth * this.scale;
        const ch = this.mapHeight * this.scale;

        // Draw grid
        this.ctx.strokeStyle = '#444';
        this.ctx.lineWidth = 1;

        for (let r = 0; r < this.world.height; r++) {
            for (let c = 0; c < this.world.width; c++) {
                const idx = r * this.world.width + c;
                const mapIdx = this.world.data[idx];

                const x = c * cw;
                const y = r * ch;

                if (mapIdx >= 0) {
                    const thumb = this.getMapThumbnail(mapIdx);
                    if (thumb) {
                        this.ctx.drawImage(thumb, 0, 0, this.mapWidth, this.mapHeight, x, y, cw, ch);
                    } else {
                        // Fallback text
                        this.ctx.fillStyle = '#222';
                        this.ctx.fillRect(x, y, cw, ch);
                        this.ctx.fillStyle = '#fff';
                        this.ctx.font = '10px sans-serif';
                        this.ctx.fillText(`Map ${mapIdx}`, x + 5, y + 15);
                    }
                } else {
                    // Empty
                    this.ctx.strokeRect(x, y, cw, ch);
                }
            }
        }

        // Draw grid lines on top
        this.ctx.beginPath();
        for (let c = 0; c <= this.world.width; c++) {
            this.ctx.moveTo(c * cw, 0);
            this.ctx.lineTo(c * cw, this.canvas.height);
        }
        for (let r = 0; r <= this.world.height; r++) {
            this.ctx.moveTo(0, r * ch);
            this.ctx.lineTo(this.canvas.width, r * ch);
        }
        this.ctx.stroke();
    }

    getData() {
        return this.world;
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('world-editor-root')) {
        window.worldEditor = new WorldEditor();
    }
});
