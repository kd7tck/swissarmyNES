// Palette Editor Component

class PaletteEditor {
    constructor() {
        this.nesPalette = [
            '545454', '001e74', '081090', '300088', '440064', '5c0030', '540400', '3c1800', '202a00', '083a00', '004000', '003c00', '00323c', '000000', '000000', '000000',
            '989698', '084cc4', '3032ec', '5c1ee4', '8814b0', 'a01464', '982220', '783c00', '545a00', '287200', '087c00', '007628', '006678', '000000', '000000', '000000',
            'eceeec', '4c9aec', '787cec', 'b062ec', 'e454ec', 'ec58b4', 'ec6a64', 'd48820', 'a0aa00', '74c400', '4cd020', '38cc6c', '38b4cc', '3c3c3c', '000000', '000000',
            'ececec', 'a8ccec', 'bcbcec', 'd4b2ec', 'ecaeec', 'ecaed4', 'ecb4b0', 'e4c490', 'ccd278', 'b4de78', 'a8e290', '98e2b4', 'a0d6e4', 'a0a2a0', '000000', '000000'
        ];

        // 4 Background Palettes (4 colors each) + 4 Sprite Palettes (4 colors each)
        this.palettes = [];
        this.selectedColorIndex = 0x0D; // Default black

        this.container = document.getElementById('palette-editor-root');

        // Listen for project load
        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));

        // Initial render (likely empty sub-palettes until project loads)
        this.render();
    }

    onProjectLoaded(assets) {
        // Initialize palettes from assets or defaults
        this.palettes = [];
        const names = ["BG0", "BG1", "BG2", "BG3", "SP0", "SP1", "SP2", "SP3"];

        // Ensure assets.palettes exists
        if (!assets.palettes) {
            assets.palettes = [];
        }

        names.forEach(name => {
            let found = assets.palettes.find(p => p.name === name);
            if (!found) {
                found = { name: name, colors: [0x0F, 0x00, 0x10, 0x20] }; // Default grays
                assets.palettes.push(found);
            }
            this.palettes.push(found);
        });

        this.render();
    }

    render() {
        if (!this.container) return;
        this.container.innerHTML = '';

        const wrapper = document.createElement('div');
        wrapper.className = 'palette-editor';

        // 1. System Palette Picker
        const systemPaletteDiv = document.createElement('div');
        systemPaletteDiv.className = 'system-palette';
        this.nesPalette.forEach((color, idx) => {
            const swatch = document.createElement('div');
            swatch.className = 'swatch';
            swatch.style.backgroundColor = '#' + color;
            swatch.title = '$' + idx.toString(16).toUpperCase().padStart(2, '0');
            if (this.selectedColorIndex === idx) {
                swatch.classList.add('selected');
            }
            swatch.onclick = () => {
                this.selectedColorIndex = idx;
                this.render(); // Re-render to update selection UI
            };
            systemPaletteDiv.appendChild(swatch);
        });

        const sysHeader = document.createElement('h3');
        sysHeader.textContent = "System Colors";
        wrapper.appendChild(sysHeader);
        wrapper.appendChild(systemPaletteDiv);

        // 2. Sub-Palettes
        const subPalettesDiv = document.createElement('div');
        subPalettesDiv.className = 'sub-palettes';

        this.palettes.forEach((pal, palIdx) => {
            const row = document.createElement('div');
            row.className = 'palette-row';

            const label = document.createElement('span');
            label.className = 'palette-label';
            label.textContent = pal.name;
            row.appendChild(label);

            pal.colors.forEach((colIdx, slotIdx) => {
                const swatch = document.createElement('div');
                swatch.className = 'swatch big';
                const hex = this.nesPalette[colIdx & 0x3F];
                swatch.style.backgroundColor = '#' + hex;
                swatch.innerText = '$' + colIdx.toString(16).toUpperCase().padStart(2, '0');
                swatch.style.color = (parseInt(hex, 16) > 0x808080) ? '#000' : '#fff'; // Contrast text

                swatch.onclick = () => {
                    // Update the color
                    pal.colors[slotIdx] = this.selectedColorIndex;

                    // If slot 0, update all slot 0s (Shared background color rule)
                    if (slotIdx === 0) {
                        this.palettes.forEach(p => p.colors[0] = this.selectedColorIndex);
                    }

                    this.render();

                    // Trigger sync to project assets?
                    // We are mutating the objects inside assets.palettes directly.
                    // But we might want to notify that data changed?
                    // For now, saving will just read the current state.

                    // Dispatch event for other editors (like CHR editor)
                    window.dispatchEvent(new CustomEvent('palette-changed'));
                };

                row.appendChild(swatch);
            });

            subPalettesDiv.appendChild(row);
        });

        const subHeader = document.createElement('h3');
        subHeader.textContent = "Project Palettes";
        wrapper.appendChild(subHeader);
        wrapper.appendChild(subPalettesDiv);

        this.container.appendChild(wrapper);
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    // Only init if the element exists (it will be added to index.html)
    if (document.getElementById('palette-editor-root')) {
        window.paletteEditor = new PaletteEditor();
    }
});
