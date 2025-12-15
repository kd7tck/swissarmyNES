
class CHREditor {
    constructor() {
        this.container = document.getElementById('chr-editor-root');
        this.canvas = null;
        this.ctx = null;
        this.assets = null;
        this.currentTileIndex = 0;
        this.currentColorIndex = 1; // 0-3
        this.scale = 20; // Scale 8x8 up to 160x160

        // Palette for rendering (RGB strings)
        // Default to grayscale if no palette loaded
        this.renderPalette = ['#000000', '#666666', '#aaaaaa', '#ffffff'];

        // Listen for project load
        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));

        // Listen for palette selection changes (if we implement that event)
        // For now, we might just assume the first sub-palette or let user pick.
        window.addEventListener('palette-changed', () => {
             this.updateRenderPalette();
             this.render();
        });

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        // Controls Area
        const controls = document.createElement('div');
        controls.className = 'chr-controls';

        // Tile Navigation
        const btnPrev = document.createElement('button');
        btnPrev.textContent = '<';
        btnPrev.onclick = () => this.setTile(this.currentTileIndex - 1);

        this.lblTile = document.createElement('span');
        this.lblTile.textContent = 'Tile $00';

        const btnNext = document.createElement('button');
        btnNext.textContent = '>';
        btnNext.onclick = () => this.setTile(this.currentTileIndex + 1);

        controls.appendChild(btnPrev);
        controls.appendChild(this.lblTile);
        controls.appendChild(btnNext);
        this.container.appendChild(controls);

        // Editor Canvas
        this.canvas = document.createElement('canvas');
        this.canvas.width = 8 * this.scale;
        this.canvas.height = 8 * this.scale;
        this.canvas.className = 'chr-canvas';
        this.ctx = this.canvas.getContext('2d');

        // Disable smoothing for pixel art
        this.ctx.imageSmoothingEnabled = false;

        // Mouse Events
        let isDrawing = false;

        const drawPixel = (e) => {
            const rect = this.canvas.getBoundingClientRect();
            const x = Math.floor((e.clientX - rect.left) / this.scale);
            const y = Math.floor((e.clientY - rect.top) / this.scale);

            if (x >= 0 && x < 8 && y >= 0 && y < 8) {
                this.updatePixel(x, y, this.currentColorIndex);
            }
        };

        this.canvas.addEventListener('mousedown', (e) => {
            isDrawing = true;
            drawPixel(e);
        });

        window.addEventListener('mouseup', () => {
            isDrawing = false;
        });

        this.canvas.addEventListener('mousemove', (e) => {
            if (isDrawing) drawPixel(e);
        });

        this.container.appendChild(this.canvas);

        // Color Picker (0-3)
        const colorPicker = document.createElement('div');
        colorPicker.className = 'chr-color-picker';
        this.colorButtons = [];

        for (let i = 0; i < 4; i++) {
            const btn = document.createElement('div');
            btn.className = 'color-btn';
            btn.dataset.index = i;
            if (i === this.currentColorIndex) btn.classList.add('selected');

            btn.onclick = () => {
                this.currentColorIndex = i;
                this.updateColorSelection();
            };

            this.colorButtons.push(btn);
            colorPicker.appendChild(btn);
        }

        this.container.appendChild(colorPicker);

        this.render();
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        // Validate CHR bank size
        if (!this.assets.chr_bank || this.assets.chr_bank.length === 0) {
            // Create default 4KB CHR if missing
            this.assets.chr_bank = new Array(4096).fill(0);
        } else {
            // Ensure it's an array (it might come in as an object if serialized weirdly, but usually array)
            // If it's a regular array from JSON, it's fine.
        }

        // Try to sync palette from assets (take the first sub-palette if available)
        this.updateRenderPalette();

        this.currentTileIndex = 0;
        this.render();
    }

    updateRenderPalette() {
        if (this.assets && this.assets.palettes && this.assets.palettes.length > 0) {
            // Use SP0 or BG0? Let's use the first one found.
            const pal = this.assets.palettes[0];
            if (pal && window.paletteEditor && window.paletteEditor.nesPalette) {
                this.renderPalette = pal.colors.map(c => '#' + window.paletteEditor.nesPalette[c & 0x3F]);
            }
        }
        // Update color buttons UI
        this.colorButtons.forEach((btn, i) => {
            btn.style.backgroundColor = this.renderPalette[i];
        });
    }

    setTile(index) {
        if (index < 0) index = 255;
        if (index > 255) index = 0;
        this.currentTileIndex = index;
        this.lblTile.textContent = 'Tile $' + index.toString(16).toUpperCase().padStart(2, '0');
        this.render();
    }

    updateColorSelection() {
        this.colorButtons.forEach((btn, i) => {
            if (i === this.currentColorIndex) btn.classList.add('selected');
            else btn.classList.remove('selected');
        });
    }

    // Convert (x,y) to CHR format and update
    updatePixel(x, y, color) {
        if (!this.assets || !this.assets.chr_bank) return;

        const tileOffset = this.currentTileIndex * 16;

        // CHR Format:
        // Byte y (0-7): Bit 0 of row y
        // Byte y+8 (8-15): Bit 1 of row y
        // Bit 7 is leftmost pixel (x=0)

        const bitMask = 1 << (7 - x);
        const lowByteIdx = tileOffset + y;
        const highByteIdx = tileOffset + y + 8;

        // Clear the bit in both planes
        this.assets.chr_bank[lowByteIdx] &= ~bitMask;
        this.assets.chr_bank[highByteIdx] &= ~bitMask;

        // Set bit 0 if color has bit 0 set
        if (color & 1) {
            this.assets.chr_bank[lowByteIdx] |= bitMask;
        }

        // Set bit 1 if color has bit 1 set
        if (color & 2) {
            this.assets.chr_bank[highByteIdx] |= bitMask;
        }

        this.render();
    }

    render() {
        if (!this.ctx) return;

        // Clear
        this.ctx.fillStyle = '#000'; // Background
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        if (!this.assets || !this.assets.chr_bank) return;

        const tileOffset = this.currentTileIndex * 16;

        // Draw 8x8
        for (let y = 0; y < 8; y++) {
            const lowByte = this.assets.chr_bank[tileOffset + y];
            const highByte = this.assets.chr_bank[tileOffset + y + 8];

            for (let x = 0; x < 8; x++) {
                const bitMask = 1 << (7 - x);
                const bit0 = (lowByte & bitMask) ? 1 : 0;
                const bit1 = (highByte & bitMask) ? 1 : 0;
                const colorVal = bit0 + (bit1 << 1); // 0-3

                this.ctx.fillStyle = this.renderPalette[colorVal];
                this.ctx.fillRect(x * this.scale, y * this.scale, this.scale, this.scale);

                // Grid lines (optional, maybe faint)
                this.ctx.strokeStyle = '#444';
                this.ctx.lineWidth = 1;
                this.ctx.strokeRect(x * this.scale, y * this.scale, this.scale, this.scale);
            }
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('chr-editor-root')) {
        window.chrEditor = new CHREditor();
    }
});
