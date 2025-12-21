class WorldEditor {
    constructor() {
        this.container = document.getElementById('world-editor-root');
        this.assets = null;
        this.selectedNtIndex = -1; // -1 means no selection (or Erase if we add specific mode)
        this.scale = 1; // 1 pixel per tile (32x30 per screen)

        // Listeners
        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));
        window.addEventListener('chr-changed', () => this.render()); // Re-render if tiles change (if we render tiles)
        window.addEventListener('palette-changed', () => this.render());

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        // Sidebar
        const sidebar = document.createElement('div');
        sidebar.className = 'world-sidebar';

        const h3 = document.createElement('h3');
        h3.textContent = 'Nametables';
        sidebar.appendChild(h3);

        const help = document.createElement('p');
        help.textContent = 'Select a nametable to place it on the world grid. Right-click grid to remove.';
        help.style.fontSize = '12px';
        help.style.color = '#888';
        sidebar.appendChild(help);

        this.listEl = document.createElement('ul');
        this.listEl.className = 'world-nt-list';
        sidebar.appendChild(this.listEl);

        this.container.appendChild(sidebar);

        // Main Area
        const main = document.createElement('div');
        main.className = 'world-main';

        // Toolbar
        const toolbar = document.createElement('div');
        toolbar.className = 'world-toolbar';

        const lblSize = document.createElement('span');
        lblSize.textContent = 'World Size: ';
        toolbar.appendChild(lblSize);

        this.inputW = document.createElement('input');
        this.inputW.type = 'number';
        this.inputW.value = 16;
        this.inputW.style.width = '50px';
        this.inputW.onchange = () => this.resizeWorld();
        toolbar.appendChild(this.inputW);

        const lblX = document.createElement('span');
        lblX.textContent = 'x';
        toolbar.appendChild(lblX);

        this.inputH = document.createElement('input');
        this.inputH.type = 'number';
        this.inputH.value = 16;
        this.inputH.style.width = '50px';
        this.inputH.onchange = () => this.resizeWorld();
        toolbar.appendChild(this.inputH);

        main.appendChild(toolbar);

        // Canvas Wrapper
        const wrapper = document.createElement('div');
        wrapper.className = 'world-canvas-wrapper';

        this.canvas = document.createElement('canvas');
        this.canvas.className = 'world-canvas';
        // Events
        this.canvas.addEventListener('mousedown', (e) => this.onMouseDown(e));
        this.canvas.addEventListener('mousemove', (e) => this.onMouseMove(e));
        this.canvas.addEventListener('contextmenu', (e) => { e.preventDefault(); }); // Prevent menu on right click

        wrapper.appendChild(this.canvas);
        main.appendChild(wrapper);

        this.container.appendChild(main);

        this.ctx = this.canvas.getContext('2d');

        this.isDrawing = false;
        window.addEventListener('mouseup', () => this.isDrawing = false);
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.world) {
            this.assets.world = {
                width: 16,
                height: 16,
                data: new Array(16*16).fill(0xFF)
            };
        }

        // Update UI inputs
        this.inputW.value = this.assets.world.width;
        this.inputH.value = this.assets.world.height;

        this.renderList();
        this.render();
    }

    resizeWorld() {
        if (!this.assets || !this.assets.world) return;

        const newW = parseInt(this.inputW.value) || 16;
        const newH = parseInt(this.inputH.value) || 16;

        if (newW === this.assets.world.width && newH === this.assets.world.height) return;

        // Resize data array, preserving content where possible
        const newData = new Array(newW * newH).fill(0xFF);

        for(let y=0; y < Math.min(newH, this.assets.world.height); y++) {
            for(let x=0; x < Math.min(newW, this.assets.world.width); x++) {
                const oldIdx = y * this.assets.world.width + x;
                const newIdx = y * newW + x;
                newData[newIdx] = this.assets.world.data[oldIdx];
            }
        }

        this.assets.world.width = newW;
        this.assets.world.height = newH;
        this.assets.world.data = newData;

        this.render();
    }

    renderList() {
        this.listEl.innerHTML = '';
        if (!this.assets || !this.assets.nametables) return;

        this.assets.nametables.forEach((nt, i) => {
            const li = document.createElement('li');
            li.textContent = nt.name || `Nametable ${i}`;
            li.onclick = () => {
                this.selectedNtIndex = i;
                this.highlightSelection();
            };
            this.listEl.appendChild(li);
        });

        this.highlightSelection();
    }

    highlightSelection() {
        Array.from(this.listEl.children).forEach((li, i) => {
            if (i === this.selectedNtIndex) li.className = 'selected';
            else li.className = '';
        });
    }

    onMouseDown(e) {
        this.isDrawing = true;
        this.handleMouse(e);
    }

    onMouseMove(e) {
        if (this.isDrawing) {
            this.handleMouse(e);
        }
    }

    handleMouse(e) {
        if (!this.assets || !this.assets.world) return;

        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        // 1 Screen = 32x30 pixels
        const gridX = Math.floor(x / 32);
        const gridY = Math.floor(y / 30);

        if (gridX >= 0 && gridX < this.assets.world.width &&
            gridY >= 0 && gridY < this.assets.world.height) {

            const idx = gridY * this.assets.world.width + gridX;

            if (e.button === 2 || e.buttons === 2) {
                // Right click = Erase
                this.assets.world.data[idx] = 0xFF;
            } else if (this.selectedNtIndex >= 0) {
                this.assets.world.data[idx] = this.selectedNtIndex;
            }

            this.render();
        }
    }

    render() {
        if (!this.assets || !this.assets.world) return;

        const w = this.assets.world.width;
        const h = this.assets.world.height;

        // 32x30 pixels per screen
        this.canvas.width = w * 32;
        this.canvas.height = h * 30;

        // Fill background
        this.ctx.fillStyle = '#111';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        const imgData = this.ctx.createImageData(this.canvas.width, this.canvas.height);
        const buf = new Uint32Array(imgData.data.buffer);

        const nametables = this.assets.nametables;
        const chrBank = this.assets.chr_bank; // Optional: For detailed rendering

        // Render each screen
        for (let wy = 0; wy < h; wy++) {
            for (let wx = 0; wx < w; wx++) {
                const ntIdx = this.assets.world.data[wy * w + wx];
                if (ntIdx === 0xFF || !nametables || !nametables[ntIdx]) {
                    // Empty or invalid: draw checkerboard or outline?
                    // Background is already dark.
                    // Draw a faint outline maybe?
                    this.drawOutline(wx, wy);
                    continue;
                }

                const nt = nametables[ntIdx];

                // Draw tiles
                for (let row = 0; row < 30; row++) {
                    for (let col = 0; col < 32; col++) {
                        const tileIdx = nt.data[row * 32 + col];

                        // Pixel coord on canvas
                        const px = wx * 32 + col;
                        const py = wy * 30 + row;
                        const pIdx = py * this.canvas.width + px;

                        // Simple Grayscale based on tile ID for now
                        // To make it look nice, we could check density of tile in CHR?
                        // Too expensive to count bits.
                        // Just use tileIdx as color?
                        // tileIdx is 0-255.
                        // Map to grayscale.
                        // ABGR format for Uint32 Little Endian

                        const val = tileIdx;
                        // Greenish tint
                        // A=255, B=val/2, G=val, R=val/2
                        buf[pIdx] = 0xFF000000 | ((val >> 1) << 16) | (val << 8) | (val >> 1);
                    }
                }

                // Draw Attribute overlay? (skip for now)
            }
        }

        this.ctx.putImageData(imgData, 0, 0);

        // Draw Grid Lines on top
        this.ctx.strokeStyle = 'rgba(255, 255, 255, 0.3)';
        this.ctx.beginPath();
        for (let x = 0; x <= w; x++) {
            this.ctx.moveTo(x * 32, 0);
            this.ctx.lineTo(x * 32, h * 30);
        }
        for (let y = 0; y <= h; y++) {
            this.ctx.moveTo(0, y * 30);
            this.ctx.lineTo(w * 32, y * 30);
        }
        this.ctx.stroke();
    }

    drawOutline(wx, wy) {
        // Since we are using putImageData, we can't easily draw vector lines inside the loop efficiently
        // unless we manipulate pixels.
        // We handle grid lines after the loop.
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('world-editor-root')) {
        window.worldEditor = new WorldEditor();
    }
});
