class MetatileEditor {
    constructor() {
        this.container = document.getElementById('metatile-editor-root');
        this.assets = null;
        this.currentMetatileIndex = -1;
        this.scale = 16; // 16x scale for 16x16 pixel metatile (2x2 tiles)

        // Event listeners
        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));
        window.addEventListener('chr-changed', () => this.render());
        window.addEventListener('palette-changed', () => this.render());

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        // Layout: Left (List), Right (Editor)
        this.container.style.display = 'flex';
        this.container.style.gap = '20px';
        this.container.style.height = '100%';

        // --- Left Column: List ---
        const leftCol = document.createElement('div');
        leftCol.style.width = '200px';
        leftCol.style.display = 'flex';
        leftCol.style.flexDirection = 'column';

        const listHeader = document.createElement('h3');
        listHeader.textContent = 'Metatiles';
        leftCol.appendChild(listHeader);

        const listToolbar = document.createElement('div');
        const btnAdd = document.createElement('button');
        btnAdd.textContent = '+';
        btnAdd.onclick = () => this.addMetatile();
        const btnDel = document.createElement('button');
        btnDel.textContent = '-';
        btnDel.onclick = () => this.deleteMetatile();
        listToolbar.appendChild(btnAdd);
        listToolbar.appendChild(btnDel);
        leftCol.appendChild(listToolbar);

        this.listEl = document.createElement('ul');
        this.listEl.className = 'item-list'; // Re-use generic list class if available or define
        this.listEl.style.flex = '1';
        this.listEl.style.overflowY = 'auto';
        this.listEl.style.border = '1px solid #444';
        leftCol.appendChild(this.listEl);

        this.container.appendChild(leftCol);

        // --- Right Column: Editor ---
        const rightCol = document.createElement('div');
        rightCol.style.flex = '1';

        const editorHeader = document.createElement('h3');
        editorHeader.textContent = 'Editor';
        rightCol.appendChild(editorHeader);

        // Name Input
        this.nameInput = document.createElement('input');
        this.nameInput.type = 'text';
        this.nameInput.placeholder = 'Metatile Name';
        this.nameInput.onchange = (e) => this.renameMetatile(e.target.value);
        this.nameInput.style.marginBottom = '10px';
        this.nameInput.style.width = '100%';
        rightCol.appendChild(this.nameInput);

        // Canvas for 2x2 tiles
        // 16x16 pixels. Scaled up.
        this.canvas = document.createElement('canvas');
        this.canvas.width = 16 * this.scale;
        this.canvas.height = 16 * this.scale;
        this.canvas.style.border = '1px solid #666';
        this.canvas.style.marginTop = '10px';
        this.canvas.style.cursor = 'pointer';

        // Mouse interaction to place tiles
        this.canvas.addEventListener('mousedown', (e) => this.onCanvasClick(e));

        rightCol.appendChild(this.canvas);

        // Palette Selector
        const palContainer = document.createElement('div');
        palContainer.style.marginTop = '10px';
        const lblPal = document.createElement('span');
        lblPal.textContent = 'Attribute (Palette): ';
        palContainer.appendChild(lblPal);

        this.palSelect = document.createElement('select');
        for(let i=0; i<4; i++) {
            const opt = document.createElement('option');
            opt.value = i;
            opt.textContent = `Palette ${i}`;
            this.palSelect.appendChild(opt);
        }
        this.palSelect.onchange = (e) => this.setPalette(parseInt(e.target.value));
        palContainer.appendChild(this.palSelect);
        rightCol.appendChild(palContainer);

        // Help Text
        const help = document.createElement('p');
        help.textContent = 'Click quadrants to assign the currently selected tile from the Graphics tab.';
        help.style.fontSize = '12px';
        help.style.color = '#888';
        rightCol.appendChild(help);

        this.container.appendChild(rightCol);

        this.ctx = this.canvas.getContext('2d');
        this.ctx.imageSmoothingEnabled = false;
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.metatiles) {
            this.assets.metatiles = [];
        }
        this.renderList();
        if (this.assets.metatiles.length > 0) {
            this.selectMetatile(0);
        } else {
            this.currentMetatileIndex = -1;
            this.render();
        }
    }

    addMetatile() {
        if (!this.assets) return;
        const newMeta = {
            name: `Meta ${this.assets.metatiles.length}`,
            tiles: [0, 0, 0, 0], // TopLeft, TopRight, BottomLeft, BottomRight
            attr: 0
        };
        this.assets.metatiles.push(newMeta);
        this.renderList();
        this.selectMetatile(this.assets.metatiles.length - 1);
    }

    deleteMetatile() {
        if (!this.assets || this.currentMetatileIndex < 0) return;
        if (!confirm('Delete this metatile? This will remove it from all maps.')) return;

        const deletedIndex = this.currentMetatileIndex;

        // Remove from list
        this.assets.metatiles.splice(deletedIndex, 1);

        // Fix up references in Nametables
        if (this.assets.nametables) {
            this.assets.nametables.forEach(nt => {
                if (nt.metatile_grid) {
                    for(let i=0; i<nt.metatile_grid.length; i++) {
                        const val = nt.metatile_grid[i];
                        if (val === deletedIndex) {
                            nt.metatile_grid[i] = -1; // Empty
                        } else if (val > deletedIndex) {
                            nt.metatile_grid[i] = val - 1; // Shift down
                        }
                    }
                }
            });
        }

        // Notify Map Editor to refresh
        // We trigger a global event because we don't have direct access
        // 'metatile-changed' is usually for content updates, but maybe we need 'metatiles-reindexed'
        // For now, we can rely on map editor refreshing if we trigger something generic or if the user clicks around.
        // Actually, let's trigger a nametable refresh for all nametables if possible, or just one.
        // Or we can assume MapEditor listens for 'project-loaded' or we can add a new event.
        window.dispatchEvent(new CustomEvent('metatile-changed', { detail: { index: -1 } })); // Force refresh

        this.renderList();
        this.currentMetatileIndex = -1;
        if (this.assets.metatiles.length > 0) {
            this.selectMetatile(0);
        } else {
            this.render();
        }
    }

    renameMetatile(newName) {
        if (!this.assets || this.currentMetatileIndex < 0) return;
        this.assets.metatiles[this.currentMetatileIndex].name = newName;
        this.renderList();
    }

    setPalette(idx) {
        if (!this.assets || this.currentMetatileIndex < 0) return;
        this.assets.metatiles[this.currentMetatileIndex].attr = idx;
        this.render();
        window.dispatchEvent(new CustomEvent('metatile-changed', { detail: { index: this.currentMetatileIndex } }));
    }

    selectMetatile(index) {
        this.currentMetatileIndex = index;
        const meta = this.assets.metatiles[index];
        this.nameInput.value = meta.name;
        this.palSelect.value = meta.attr;

        // Highlight in list
        Array.from(this.listEl.children).forEach((li, i) => {
            li.className = (i === index) ? 'selected' : '';
        });

        this.render();
    }

    renderList() {
        this.listEl.innerHTML = '';
        if (!this.assets || !this.assets.metatiles) return;

        this.assets.metatiles.forEach((m, i) => {
            const li = document.createElement('li');
            li.textContent = m.name;
            li.onclick = () => this.selectMetatile(i);
            if (i === this.currentMetatileIndex) li.className = 'selected';
            this.listEl.appendChild(li);
        });
    }

    onCanvasClick(e) {
        if (!this.assets || this.currentMetatileIndex < 0) return;
        // Require CHR editor for tile selection
        // We can access window.chrEditor if it exists

        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        // Determine quadrant (0-3)
        // Canvas is scaled.
        // The click coordinates are relative to the canvas size.
        // We divide by scale * 8 (each tile is 8 pixels)
        const scale = this.scale;
        const qx = Math.floor(x / (8 * scale));
        const qy = Math.floor(y / (8 * scale));

        // Index mapping:
        // 0 (TL), 1 (TR)
        // 2 (BL), 3 (BR)
        const idx = qy * 2 + qx;

        if (idx >= 0 && idx < 4) {
             let selectedTile = 0;
            if (window.chrEditor) {
                selectedTile = window.chrEditor.currentTileIndex;
            }
            this.assets.metatiles[this.currentMetatileIndex].tiles[idx] = selectedTile;
            this.render();
            window.dispatchEvent(new CustomEvent('metatile-changed', { detail: { index: this.currentMetatileIndex } }));
        }
    }

    render() {
        if (!this.ctx) return;
        // Clear
        this.ctx.fillStyle = '#000';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        if (!this.assets || this.currentMetatileIndex < 0) return;

        const meta = this.assets.metatiles[this.currentMetatileIndex];
        const chrBank = this.assets.chr_bank;

        // Determine palette colors
        let palette = ['#000', '#555', '#aaa', '#fff'];
        if (this.assets.palettes && this.assets.palettes[meta.attr]) {
             if (window.paletteEditor && window.paletteEditor.nesPalette) {
                 palette = this.assets.palettes[meta.attr].colors.map(c => '#' + window.paletteEditor.nesPalette[c & 0x3F]);
             }
        }

        // Draw 4 tiles
        for(let i=0; i<4; i++) {
            const tileIdx = meta.tiles[i];
            const tx = (i % 2) * 8;
            const ty = Math.floor(i / 2) * 8;
            this.drawTile(tx, ty, tileIdx, chrBank, palette);
        }

        // Draw Grid Cross
        this.ctx.strokeStyle = 'rgba(0, 255, 0, 0.5)';
        this.ctx.beginPath();
        this.ctx.moveTo(this.canvas.width / 2, 0);
        this.ctx.lineTo(this.canvas.width / 2, this.canvas.height);
        this.ctx.moveTo(0, this.canvas.height / 2);
        this.ctx.lineTo(this.canvas.width, this.canvas.height / 2);
        this.ctx.stroke();
    }

    drawTile(x, y, tileIdx, chrBank, palette) {
        if (!chrBank) return;
        const tileOffset = tileIdx * 16;
        const scale = this.scale;

        for (let py = 0; py < 8; py++) {
            const lowByte = chrBank[tileOffset + py];
            const highByte = chrBank[tileOffset + py + 8];

            for (let px = 0; px < 8; px++) {
                const bitMask = 1 << (7 - px);
                const bit0 = (lowByte & bitMask) ? 1 : 0;
                const bit1 = (highByte & bitMask) ? 1 : 0;
                const colorVal = bit0 + (bit1 << 1);

                this.ctx.fillStyle = palette[colorVal];
                this.ctx.fillRect(
                    (x + px) * scale,
                    (y + py) * scale,
                    scale, scale
                );
            }
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('metatile-editor-root')) {
        window.metatileEditor = new MetatileEditor();
    }
});
