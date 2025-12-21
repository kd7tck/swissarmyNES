
class CHREditor {
    constructor() {
        this.container = document.getElementById('chr-editor-root');
        this.canvas = null;
        this.ctx = null;
        this.assets = null;
        this.currentTileIndex = 0;
        this.currentColorIndex = 1; // 0-3
        this.scale = 20; // Scale 8x8 up to 160x160
        this.bankCanvas = null;
        this.bankCtx = null;

        // Palette for rendering (RGB strings)
        // Default to grayscale if no palette loaded
        this.renderPalette = ['#000000', '#666666', '#aaaaaa', '#ffffff'];

        // Listen for project load
        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));

        // Listen for palette selection changes
        window.addEventListener('palette-changed', () => {
             this.updateRenderPalette();
             this.render();
        });

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        // Controls Area (Prev/Next/Label)
        const controls = document.createElement('div');
        controls.className = 'chr-controls';

        const btnPrev = document.createElement('button');
        btnPrev.textContent = '<';
        btnPrev.onclick = () => this.setTile(this.currentTileIndex - 1);

        this.lblTile = document.createElement('span');
        this.lblTile.textContent = 'Tile $00';

        const btnNext = document.createElement('button');
        btnNext.textContent = '>';
        btnNext.onclick = () => this.setTile(this.currentTileIndex + 1);

        const btnBank = document.createElement('button');
        btnBank.textContent = 'Bank View';
        btnBank.onclick = () => this.toggleBankView();

        controls.appendChild(btnPrev);
        controls.appendChild(this.lblTile);
        controls.appendChild(btnNext);
        controls.appendChild(btnBank);
        this.container.appendChild(controls);

        // Tool Bar (Shift, Flip, Fill)
        const tools = document.createElement('div');
        tools.className = 'chr-tools';

        // Shift Tools
        const shifts = [
            { label: '↑', action: () => this.shiftTile(0, -1) },
            { label: '↓', action: () => this.shiftTile(0, 1) },
            { label: '←', action: () => this.shiftTile(-1, 0) },
            { label: '→', action: () => this.shiftTile(1, 0) },
        ];
        shifts.forEach(t => {
            const btn = document.createElement('button');
            btn.textContent = t.label;
            btn.title = 'Shift';
            btn.onclick = t.action;
            tools.appendChild(btn);
        });

        // Flip Tools
        const flips = [
            { label: 'H-Flip', action: () => this.flipTile('h') },
            { label: 'V-Flip', action: () => this.flipTile('v') },
        ];
        flips.forEach(t => {
            const btn = document.createElement('button');
            btn.textContent = t.label;
            btn.title = t.label;
            btn.onclick = t.action;
            tools.appendChild(btn);
        });

        // Fill Tool
        const btnFill = document.createElement('button');
        btnFill.textContent = 'Fill';
        btnFill.title = 'Flood Fill';
        btnFill.onclick = () => this.activateFillTool();

        this.toolMode = 'pencil'; // pencil, fill
        btnFill.onclick = () => {
            this.toolMode = this.toolMode === 'fill' ? 'pencil' : 'fill';
            btnFill.style.background = this.toolMode === 'fill' ? '#666' : '';
        };
        tools.appendChild(btnFill);

        this.container.appendChild(tools);


        // Canvas Wrapper
        const wrapper = document.createElement('div');
        wrapper.className = 'chr-canvas-wrapper';

        // Editor Canvas
        this.canvas = document.createElement('canvas');
        this.canvas.width = 8 * this.scale;
        this.canvas.height = 8 * this.scale;
        this.canvas.className = 'chr-canvas';
        this.ctx = this.canvas.getContext('2d');
        this.ctx.imageSmoothingEnabled = false;

        wrapper.appendChild(this.canvas);
        this.container.appendChild(wrapper);


        // Mouse Events
        let isDrawing = false;

        const handleMouse = (e, type) => {
            const rect = this.canvas.getBoundingClientRect();
            const x = Math.floor((e.clientX - rect.left) / this.scale);
            const y = Math.floor((e.clientY - rect.top) / this.scale);

            if (x >= 0 && x < 8 && y >= 0 && y < 8) {
                if (this.toolMode === 'fill' && type === 'down') {
                    this.floodFill(x, y, this.currentColorIndex);
                } else if (this.toolMode === 'pencil') {
                    if (type === 'down') {
                        isDrawing = true;
                        this.updatePixel(x, y, this.currentColorIndex);
                    } else if (type === 'move' && isDrawing) {
                        this.updatePixel(x, y, this.currentColorIndex);
                    }
                }
            }
        };

        this.canvas.addEventListener('mousedown', (e) => handleMouse(e, 'down'));
        this.canvas.addEventListener('mousemove', (e) => handleMouse(e, 'move'));
        window.addEventListener('mouseup', () => { isDrawing = false; });

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

        // Bank View Modal
        this.createBankModal();

        this.render();
    }

    createBankModal() {
        this.modal = document.createElement('div');
        this.modal.className = 'bank-view-modal';

        const content = document.createElement('div');
        content.className = 'bank-view-content';

        const header = document.createElement('div');
        header.className = 'bank-view-header';
        const title = document.createElement('h3');
        title.textContent = 'CHR Bank';

        // Import Button
        const btnImport = document.createElement('label');
        btnImport.className = 'import-btn';
        btnImport.textContent = 'Import PNG';
        btnImport.title = 'Import 128x128 PNG (nearest color)';
        btnImport.style.cursor = 'pointer';
        btnImport.style.marginLeft = '10px';
        btnImport.style.background = '#444';
        btnImport.style.padding = '2px 6px';
        btnImport.style.border = '1px solid #666';
        btnImport.style.fontSize = '12px';
        btnImport.style.color = '#fff';

        const fileInput = document.createElement('input');
        fileInput.type = 'file';
        fileInput.accept = '.png';
        fileInput.style.display = 'none';
        fileInput.onchange = (e) => {
            if (e.target.files && e.target.files[0]) {
                this.processImportedImage(e.target.files[0]);
            }
        };
        btnImport.appendChild(fileInput);


        const closeBtn = document.createElement('button');
        closeBtn.className = 'close-modal-btn';
        closeBtn.textContent = '×';
        closeBtn.onclick = () => this.toggleBankView(false);

        header.appendChild(title);
        header.appendChild(btnImport);
        header.appendChild(closeBtn);
        content.appendChild(header);

        // 128x128 canvas (16x16 tiles * 8 pixels)
        // Scale it up by 2x or 3x for visibility
        const bankScale = 3;
        this.bankCanvas = document.createElement('canvas');
        this.bankCanvas.width = 128 * bankScale;
        this.bankCanvas.height = 128 * bankScale;
        this.bankCanvas.className = 'bank-canvas';
        this.bankCtx = this.bankCanvas.getContext('2d');
        this.bankCtx.imageSmoothingEnabled = false;

        this.bankCanvas.addEventListener('click', (e) => {
            const rect = this.bankCanvas.getBoundingClientRect();
            const x = Math.floor((e.clientX - rect.left) / bankScale);
            const y = Math.floor((e.clientY - rect.top) / bankScale);

            const tileX = Math.floor(x / 8);
            const tileY = Math.floor(y / 8);
            const tileIdx = tileY * 16 + tileX;

            if (tileIdx >= 0 && tileIdx < 256) {
                this.setTile(tileIdx);
                this.toggleBankView(false);
            }
        });

        // Drag and Drop support
        this.bankCanvas.addEventListener('dragover', (e) => {
            e.preventDefault();
            this.bankCanvas.style.borderColor = '#fff';
        });
        this.bankCanvas.addEventListener('dragleave', (e) => {
             this.bankCanvas.style.borderColor = '#444';
        });
        this.bankCanvas.addEventListener('drop', (e) => {
            e.preventDefault();
            this.bankCanvas.style.borderColor = '#444';
            if (e.dataTransfer.files && e.dataTransfer.files[0]) {
                this.processImportedImage(e.dataTransfer.files[0]);
            }
        });

        content.appendChild(this.bankCanvas);
        this.modal.appendChild(content);
        document.body.appendChild(this.modal);
    }

    toggleBankView(show) {
        if (show === undefined) {
            show = !this.modal.classList.contains('active');
        }

        if (show) {
            this.modal.classList.add('active');
            this.renderBank();
        } else {
            this.modal.classList.remove('active');
        }
    }

    processImportedImage(file) {
        if (file.type !== 'image/png') {
            alert("Only PNG images are supported.");
            return;
        }

        const reader = new FileReader();
        reader.onload = (e) => {
            const img = new Image();
            img.onload = () => {
                const tempCanvas = document.createElement('canvas');
                tempCanvas.width = 128;
                tempCanvas.height = 128;
                const tempCtx = tempCanvas.getContext('2d');
                // Draw image. If larger, it will be cropped? No, drawImage scales if we provide w,h.
                // But we want 1:1 pixel mapping.
                // We'll just draw at 0,0 and whatever fits fits.
                tempCtx.drawImage(img, 0, 0);

                const imageData = tempCtx.getImageData(0, 0, 128, 128);
                const data = imageData.data; // RGBA array

                if (!this.assets || !this.assets.chr_bank) {
                    if (this.assets) this.assets.chr_bank = new Array(4096).fill(0);
                    else return;
                }

                for (let tileY = 0; tileY < 16; tileY++) {
                    for (let tileX = 0; tileX < 16; tileX++) {
                        const tileIdx = tileY * 16 + tileX;
                        const bankOffset = tileIdx * 16;

                        // Zero out this tile
                        for(let k=0; k<16; k++) this.assets.chr_bank[bankOffset+k] = 0;

                        for (let y = 0; y < 8; y++) {
                            for (let x = 0; x < 8; x++) {
                                // Pixel coord in image
                                const px = tileX * 8 + x;
                                const py = tileY * 8 + y;

                                const i = (py * 128 + px) * 4;
                                const r = data[i];
                                const g = data[i+1];
                                const b = data[i+2];
                                const a = data[i+3];

                                let colorIdx = 0;
                                if (a < 128) {
                                    colorIdx = 0; // Transparent -> Color 0
                                } else {
                                    colorIdx = this.findNearestColorIndex(r, g, b);
                                }

                                const bitMask = 1 << (7 - x);
                                if (colorIdx & 1) this.assets.chr_bank[bankOffset + y] |= bitMask;
                                if (colorIdx & 2) this.assets.chr_bank[bankOffset + y + 8] |= bitMask;
                            }
                        }
                    }
                }
                this.render();
                this.renderBank();
                window.dispatchEvent(new Event('chr-changed'));
                // alert("Imported CHR Bank.");
            };
            img.src = e.target.result;
        };
        reader.readAsDataURL(file);
    }

    hexToRgb(hex) {
        var shorthandRegex = /^#?([a-f\d])([a-f\d])([a-f\d])$/i;
        hex = hex.replace(shorthandRegex, function(m, r, g, b) {
            return r + r + g + g + b + b;
        });

        var result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        return result ? {
            r: parseInt(result[1], 16),
            g: parseInt(result[2], 16),
            b: parseInt(result[3], 16)
        } : null;
    }

    findNearestColorIndex(r, g, b) {
        let bestDist = Infinity;
        let bestIdx = 0;

        for (let i = 0; i < 4; i++) {
            const hex = this.renderPalette[i];
            const rgb = this.hexToRgb(hex);
            if (!rgb) continue;

            // Euclidean distance squared
            const dist = (r - rgb.r) ** 2 + (g - rgb.g) ** 2 + (b - rgb.b) ** 2;
            if (dist < bestDist) {
                bestDist = dist;
                bestIdx = i;
            }
        }
        return bestIdx;
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.chr_bank || this.assets.chr_bank.length === 0) {
            this.assets.chr_bank = new Array(4096).fill(0);
        }
        this.updateRenderPalette();
        this.currentTileIndex = 0;
        this.render();
    }

    updateRenderPalette() {
        if (this.assets && this.assets.palettes && this.assets.palettes.length > 0) {
            const pal = this.assets.palettes[0];
            if (pal && window.paletteEditor && window.paletteEditor.nesPalette) {
                this.renderPalette = pal.colors.map(c => '#' + window.paletteEditor.nesPalette[c & 0x3F]);
            }
        }
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

    updatePixel(x, y, color) {
        if (!this.assets || !this.assets.chr_bank) return;
        const tileOffset = this.currentTileIndex * 16;
        const bitMask = 1 << (7 - x);
        const lowByteIdx = tileOffset + y;
        const highByteIdx = tileOffset + y + 8;

        this.assets.chr_bank[lowByteIdx] &= ~bitMask;
        this.assets.chr_bank[highByteIdx] &= ~bitMask;

        if (color & 1) this.assets.chr_bank[lowByteIdx] |= bitMask;
        if (color & 2) this.assets.chr_bank[highByteIdx] |= bitMask;

        this.render();
        window.dispatchEvent(new Event('chr-changed'));
    }

    getPixel(x, y) {
        if (!this.assets || !this.assets.chr_bank) return 0;
        const tileOffset = this.currentTileIndex * 16;
        const bitMask = 1 << (7 - x);
        const lowByte = this.assets.chr_bank[tileOffset + y];
        const highByte = this.assets.chr_bank[tileOffset + y + 8];
        const bit0 = (lowByte & bitMask) ? 1 : 0;
        const bit1 = (highByte & bitMask) ? 1 : 0;
        return bit0 + (bit1 << 1);
    }

    shiftTile(dx, dy) {
        if (!this.assets || !this.assets.chr_bank) return;

        // We'll operate on a temporary 8x8 buffer to handle wraps/shifts cleanly
        let buffer = new Array(64);
        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                buffer[y*8+x] = this.getPixel(x, y);
            }
        }

        let newBuffer = new Array(64).fill(0);

        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                // Source coordinates
                let srcX = x - dx;
                let srcY = y - dy;

                // If out of bounds, it's 0 (no wrap for shifts)
                if (srcX >= 0 && srcX < 8 && srcY >= 0 && srcY < 8) {
                    newBuffer[y*8+x] = buffer[srcY*8+srcX];
                } else {
                     newBuffer[y*8+x] = 0; // or wrap? Prompt says "Shift" which usually implies 0 fill.
                }
            }
        }

        // Write back
        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                this.updatePixel(x, y, newBuffer[y*8+x]);
            }
        }
    }

    flipTile(axis) {
        if (!this.assets || !this.assets.chr_bank) return;
        let buffer = new Array(64);
        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                buffer[y*8+x] = this.getPixel(x, y);
            }
        }

        let newBuffer = new Array(64).fill(0);
        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                let srcX = x;
                let srcY = y;

                if (axis === 'h') srcX = 7 - x;
                if (axis === 'v') srcY = 7 - y;

                newBuffer[y*8+x] = buffer[srcY*8+srcX];
            }
        }

        // Write back
        const tileOffset = this.currentTileIndex * 16;
        // Zero out tile
        for(let i=0; i<16; i++) this.assets.chr_bank[tileOffset+i] = 0;

        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                const color = newBuffer[y*8+x];
                const bitMask = 1 << (7 - x);
                if (color & 1) this.assets.chr_bank[tileOffset + y] |= bitMask;
                if (color & 2) this.assets.chr_bank[tileOffset + y + 8] |= bitMask;
            }
        }
        this.render();
        window.dispatchEvent(new Event('chr-changed'));
    }

    floodFill(startX, startY, targetColor) {
        const sourceColor = this.getPixel(startX, startY);
        if (sourceColor === targetColor) return;

        const stack = [[startX, startY]];
        const seen = new Set();

        // Copy tile data to local buffer for fast access
        let buffer = new Array(64);
        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                buffer[y*8+x] = this.getPixel(x, y);
            }
        }

        while (stack.length > 0) {
            const [x, y] = stack.pop();
            const key = `${x},${y}`;
            if (seen.has(key)) continue;

            if (x < 0 || x >= 8 || y < 0 || y >= 8) continue;

            const idx = y * 8 + x;
            if (buffer[idx] === sourceColor) {
                buffer[idx] = targetColor;
                seen.add(key);

                stack.push([x + 1, y]);
                stack.push([x - 1, y]);
                stack.push([x, y + 1]);
                stack.push([x, y - 1]);
            }
        }

        // Write back buffer
        const tileOffset = this.currentTileIndex * 16;
        for(let i=0; i<16; i++) this.assets.chr_bank[tileOffset+i] = 0;

        for(let y=0; y<8; y++) {
            for(let x=0; x<8; x++) {
                const color = buffer[y*8+x];
                const bitMask = 1 << (7 - x);
                if (color & 1) this.assets.chr_bank[tileOffset + y] |= bitMask;
                if (color & 2) this.assets.chr_bank[tileOffset + y + 8] |= bitMask;
            }
        }
        this.render();
        window.dispatchEvent(new Event('chr-changed'));
    }

    render() {
        if (!this.ctx) return;
        this.ctx.fillStyle = '#000';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        if (!this.assets || !this.assets.chr_bank) return;

        const tileOffset = this.currentTileIndex * 16;

        for (let y = 0; y < 8; y++) {
            const lowByte = this.assets.chr_bank[tileOffset + y];
            const highByte = this.assets.chr_bank[tileOffset + y + 8];

            for (let x = 0; x < 8; x++) {
                const bitMask = 1 << (7 - x);
                const bit0 = (lowByte & bitMask) ? 1 : 0;
                const bit1 = (highByte & bitMask) ? 1 : 0;
                const colorVal = bit0 + (bit1 << 1);

                this.ctx.fillStyle = this.renderPalette[colorVal];
                this.ctx.fillRect(x * this.scale, y * this.scale, this.scale, this.scale);
            }
        }
    }

    renderBank() {
        if (!this.bankCtx || !this.assets || !this.assets.chr_bank) return;

        const scale = this.bankCanvas.width / 128; // Should match what we set (e.g. 3)

        this.bankCtx.fillStyle = '#000';
        this.bankCtx.fillRect(0, 0, this.bankCanvas.width, this.bankCanvas.height);

        for(let t = 0; t < 256; t++) {
            const tileX = t % 16;
            const tileY = Math.floor(t / 16);
            const tileOffset = t * 16;

            for(let y = 0; y < 8; y++) {
                const lowByte = this.assets.chr_bank[tileOffset + y];
                const highByte = this.assets.chr_bank[tileOffset + y + 8];

                for(let x = 0; x < 8; x++) {
                     const bitMask = 1 << (7 - x);
                     const bit0 = (lowByte & bitMask) ? 1 : 0;
                     const bit1 = (highByte & bitMask) ? 1 : 0;
                     const colorVal = bit0 + (bit1 << 1);

                     this.bankCtx.fillStyle = this.renderPalette[colorVal];
                     this.bankCtx.fillRect(
                         (tileX * 8 + x) * scale,
                         (tileY * 8 + y) * scale,
                         scale, scale
                     );
                }
            }
        }

        // Highlight current tile
        const cx = this.currentTileIndex % 16;
        const cy = Math.floor(this.currentTileIndex / 16);
        this.bankCtx.strokeStyle = '#fff';
        this.bankCtx.lineWidth = 2;
        this.bankCtx.strokeRect(
            cx * 8 * scale,
            cy * 8 * scale,
            8 * scale,
            8 * scale
        );
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('chr-editor-root')) {
        window.chrEditor = new CHREditor();
    }
});
