class ScreenEditor {
    constructor() {
        this.container = document.getElementById('screen-editor-root');
        this.assets = null;
        this.currentScreenIndex = -1;
        this.selectedMetatileIndex = 0;
        this.scale = 2; // 2x Zoom (256x240 -> 512x480)

        // Bind methods
        this.onProjectLoaded = this.onProjectLoaded.bind(this);
        this.render = this.render.bind(this);

        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));
        // Global refresh event
        window.addEventListener('chr-changed', this.render);
        window.addEventListener('palette-changed', this.render);
        // We might want a 'metatiles-changed' event later

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        // --- Left Panel: List ---
        const leftPanel = document.createElement('div');
        leftPanel.className = 'screen-list-panel';

        const listHeader = document.createElement('div');
        listHeader.className = 'screen-list-header';
        const title = document.createElement('span');
        title.textContent = 'Screens';
        listHeader.appendChild(title);

        const btnAdd = document.createElement('button');
        btnAdd.textContent = '+';
        btnAdd.title = 'Add Screen';
        btnAdd.onclick = () => this.addScreen();
        listHeader.appendChild(btnAdd);

        const btnDel = document.createElement('button');
        btnDel.textContent = '-';
        btnDel.title = 'Delete Screen';
        btnDel.onclick = () => this.deleteScreen();
        listHeader.appendChild(btnDel);

        leftPanel.appendChild(listHeader);

        this.listEl = document.createElement('ul');
        this.listEl.className = 'screen-list';
        leftPanel.appendChild(this.listEl);

        this.container.appendChild(leftPanel);

        // --- Main Area ---
        const mainArea = document.createElement('div');
        mainArea.className = 'screen-editor-main';

        // Toolbar
        const toolbar = document.createElement('div');
        toolbar.className = 'screen-toolbar';

        this.nameInput = document.createElement('input');
        this.nameInput.type = 'text';
        this.nameInput.placeholder = 'Screen Name';
        this.nameInput.onchange = (e) => this.renameScreen(e.target.value);
        toolbar.appendChild(this.nameInput);

        const lblInfo = document.createElement('span');
        lblInfo.textContent = ' 16x15 Metatiles (256x240 px)';
        lblInfo.style.color = '#888';
        lblInfo.style.fontSize = '12px';
        toolbar.appendChild(lblInfo);

        mainArea.appendChild(toolbar);

        // Workspace
        const workspace = document.createElement('div');
        workspace.className = 'screen-workspace';

        // Canvas Container
        const canvasContainer = document.createElement('div');
        canvasContainer.className = 'screen-canvas-container';

        this.canvas = document.createElement('canvas');
        this.canvas.className = 'screen-canvas';
        this.canvas.width = 256 * this.scale;
        this.canvas.height = 240 * this.scale;

        // Mouse Interaction
        this.isDrawing = false;
        this.canvas.addEventListener('mousedown', (e) => {
            this.isDrawing = true;
            this.handleDraw(e);
        });
        this.canvas.addEventListener('mousemove', (e) => {
            if (this.isDrawing) this.handleDraw(e);
        });
        window.addEventListener('mouseup', () => {
            this.isDrawing = false;
        });

        canvasContainer.appendChild(this.canvas);
        workspace.appendChild(canvasContainer);

        // Palette (Metatiles)
        const palettePanel = document.createElement('div');
        palettePanel.className = 'metatile-palette';

        const palHeader = document.createElement('div');
        palHeader.className = 'metatile-palette-header';
        palHeader.textContent = 'Metatiles';
        palettePanel.appendChild(palHeader);

        this.paletteGrid = document.createElement('div');
        this.paletteGrid.className = 'metatile-palette-grid';
        palettePanel.appendChild(this.paletteGrid);

        workspace.appendChild(palettePanel);
        mainArea.appendChild(workspace);

        this.container.appendChild(mainArea);

        this.ctx = this.canvas.getContext('2d');
        this.ctx.imageSmoothingEnabled = false;
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.screens) {
            this.assets.screens = [];
        }
        this.renderList();
        this.renderPalette();

        if (this.assets.screens.length > 0) {
            this.selectScreen(0);
        } else {
            this.currentScreenIndex = -1;
            this.render();
        }
    }

    addScreen() {
        if (!this.assets) return;
        const newScreen = {
            name: `Screen ${this.assets.screens.length}`,
            data: Array(16 * 15).fill(0) // Fill with metatile 0
        };
        this.assets.screens.push(newScreen);
        this.renderList();
        this.selectScreen(this.assets.screens.length - 1);
    }

    deleteScreen() {
        if (!this.assets || this.currentScreenIndex < 0) return;
        if (!confirm('Delete this screen?')) return;

        this.assets.screens.splice(this.currentScreenIndex, 1);
        this.renderList();
        this.currentScreenIndex = -1;
        if (this.assets.screens.length > 0) {
            this.selectScreen(0);
        } else {
            this.render();
        }
    }

    renameScreen(newName) {
        if (!this.assets || this.currentScreenIndex < 0) return;
        this.assets.screens[this.currentScreenIndex].name = newName;
        this.renderList();
    }

    selectScreen(index) {
        this.currentScreenIndex = index;
        const screen = this.assets.screens[index];
        this.nameInput.value = screen.name;

        // Highlight in list
        Array.from(this.listEl.children).forEach((li, i) => {
            if (i === index) li.classList.add('active');
            else li.classList.remove('active');
        });

        this.render();
    }

    renderList() {
        this.listEl.innerHTML = '';
        if (!this.assets || !this.assets.screens) return;

        this.assets.screens.forEach((s, i) => {
            const li = document.createElement('li');
            li.className = 'screen-list-item';

            const nameSpan = document.createElement('span');
            nameSpan.textContent = s.name;
            li.appendChild(nameSpan);

            li.onclick = () => this.selectScreen(i);
            if (i === this.currentScreenIndex) li.classList.add('active');
            this.listEl.appendChild(li);
        });
    }

    renderPalette() {
        this.paletteGrid.innerHTML = '';
        if (!this.assets || !this.assets.metatiles) return;

        this.assets.metatiles.forEach((m, i) => {
            const swatch = document.createElement('div');
            swatch.className = 'metatile-swatch';
            if (i === this.selectedMetatileIndex) swatch.classList.add('selected');

            swatch.onclick = () => {
                this.selectedMetatileIndex = i;
                this.renderPalette(); // Update selection
            };

            // Render metatile preview into swatch
            const cvs = document.createElement('canvas');
            cvs.width = 16;
            cvs.height = 16;
            const ctx = cvs.getContext('2d');
            this.drawMetatileToContext(ctx, 0, 0, m, 1);

            swatch.style.backgroundImage = `url(${cvs.toDataURL()})`;
            swatch.style.backgroundSize = 'contain';

            this.paletteGrid.appendChild(swatch);
        });
    }

    handleDraw(e) {
        if (!this.assets || this.currentScreenIndex < 0) return;

        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        // Convert to Metatile Grid Coords
        const mx = Math.floor(x / (16 * this.scale));
        const my = Math.floor(y / (16 * this.scale));

        if (mx >= 0 && mx < 16 && my >= 0 && my < 15) {
            const idx = my * 16 + mx;
            this.assets.screens[this.currentScreenIndex].data[idx] = this.selectedMetatileIndex;
            this.render();
        }
    }

    render() {
        // Render Palette (in case metatiles changed)
        this.renderPalette();

        if (!this.ctx) return;
        this.ctx.fillStyle = '#000';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        if (!this.assets || this.currentScreenIndex < 0) return;

        const screen = this.assets.screens[this.currentScreenIndex];
        const metatiles = this.assets.metatiles || [];

        // Draw Grid
        for(let row=0; row<15; row++) {
            for(let col=0; col<16; col++) {
                const idx = row * 16 + col;
                const metaIdx = screen.data[idx];
                if (metaIdx < metatiles.length) {
                    const metatile = metatiles[metaIdx];
                    this.drawMetatileToContext(
                        this.ctx,
                        col * 16 * this.scale,
                        row * 16 * this.scale,
                        metatile,
                        this.scale
                    );
                }
            }
        }

        // Optional: Draw Grid lines
        this.ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        this.ctx.beginPath();
        for(let i=0; i<=16; i++) {
            this.ctx.moveTo(i * 16 * this.scale, 0);
            this.ctx.lineTo(i * 16 * this.scale, 15 * 16 * this.scale);
        }
        for(let i=0; i<=15; i++) {
            this.ctx.moveTo(0, i * 16 * this.scale);
            this.ctx.lineTo(16 * 16 * this.scale, i * 16 * this.scale);
        }
        this.ctx.stroke();
    }

    drawMetatileToContext(ctx, x, y, metatile, scale) {
        // Reuse logic from MetatileEditor/CHREditor to draw tiles
        // We need Palette Colors
        let palette = ['#000', '#555', '#aaa', '#fff'];
        if (this.assets.palettes && this.assets.palettes[metatile.attr]) {
             if (window.paletteEditor && window.paletteEditor.nesPalette) {
                 palette = this.assets.palettes[metatile.attr].colors.map(c => '#' + window.paletteEditor.nesPalette[c & 0x3F]);
             }
        }

        const chrBank = this.assets.chr_bank;

        // Draw 4 tiles
        for(let i=0; i<4; i++) {
            const tileIdx = metatile.tiles[i];
            const tx = (i % 2) * 8;
            const ty = Math.floor(i / 2) * 8;
            this.drawTile(ctx, x + tx * scale, y + ty * scale, tileIdx, chrBank, palette, scale);
        }
    }

    drawTile(ctx, x, y, tileIdx, chrBank, palette, scale) {
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

                // Optimization: Don't draw transparent pixels if we want layering, but here background is black anyway
                ctx.fillStyle = palette[colorVal];
                ctx.fillRect(
                    x + px * scale,
                    y + py * scale,
                    scale, scale
                );
            }
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('screen-editor-root')) {
        window.screenEditor = new ScreenEditor();
    }
});
