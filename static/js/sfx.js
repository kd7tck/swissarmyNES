class SequenceCanvas {
    constructor(container, data, min, max, color, label, onChange) {
        this.container = container;
        this.data = data; // Reference to the array
        this.min = min;
        this.max = max;
        this.range = max - min;
        this.color = color;
        this.label = label;
        this.onChange = onChange;
        this.isDrawing = false;
        this.onMouseUp = null;

        this.init();
    }

    init() {
        this.container.innerHTML = '';

        // Controls
        const controls = document.createElement('div');
        controls.className = 'seq-controls';

        const btnClear = document.createElement('button');
        btnClear.textContent = 'Clear';
        btnClear.onclick = () => {
            this.data.length = 0; // Clear array
            this.onChange();
            this.draw();
        };

        const btnAdd = document.createElement('button');
        btnAdd.textContent = '+ Step';
        btnAdd.onclick = () => {
            this.data.push(this.min); // Add zero/min value
            this.onChange();
            this.draw();
        };

        const btnTrim = document.createElement('button');
        btnTrim.textContent = '- Step';
        btnTrim.onclick = () => {
            this.data.pop();
            this.onChange();
            this.draw();
        };

        const lblInfo = document.createElement('span');
        lblInfo.textContent = `${this.label}: ${this.data.length} steps`;
        this.lblInfo = lblInfo;

        controls.appendChild(lblInfo);
        controls.appendChild(btnAdd);
        controls.appendChild(btnTrim);
        controls.appendChild(btnClear);
        this.container.appendChild(controls);

        // Canvas Wrapper
        const wrapper = document.createElement('div');
        wrapper.className = 'seq-canvas-wrapper';
        this.wrapper = wrapper;
        this.container.appendChild(wrapper);

        this.canvas = document.createElement('canvas');
        this.canvas.height = 150;
        this.ctx = this.canvas.getContext('2d');
        wrapper.appendChild(this.canvas);

        // Events
        this.canvas.addEventListener('mousedown', (e) => this.startDrawing(e));
        this.canvas.addEventListener('mousemove', (e) => this.drawMove(e));

        this.onMouseUp = () => this.stopDrawing();
        window.addEventListener('mouseup', this.onMouseUp);

        // Initial Draw
        this.draw();
    }

    destroy() {
        if (this.onMouseUp) {
            window.removeEventListener('mouseup', this.onMouseUp);
        }
        this.container.innerHTML = '';
    }

    getMousePos(e) {
        const rect = this.canvas.getBoundingClientRect();
        return {
            x: e.clientX - rect.left,
            y: e.clientY - rect.top
        };
    }

    startDrawing(e) {
        this.isDrawing = true;
        this.drawMove(e);
    }

    stopDrawing() {
        this.isDrawing = false;
    }

    drawMove(e) {
        if (!this.isDrawing) return;
        const pos = this.getMousePos(e);
        const stepWidth = 10;
        const index = Math.floor(pos.x / stepWidth);

        if (index >= 0 && index < this.data.length) {
            // Calculate value from Y
            // Y=0 is Max, Y=Height is Min
            const height = this.canvas.height;
            const normalized = 1 - (pos.y / height); // 0 to 1 (bottom to top)
            let val = Math.round(this.min + (normalized * this.range));

            // Clamp
            val = Math.max(this.min, Math.min(this.max, val));

            this.data[index] = val;
            this.onChange();
            this.draw();
        } else if (index >= this.data.length && index < 64) {
            // Optional: Auto-extend if drawing past end?
            // For now, strict explicit add
        }
    }

    draw() {
        const width = Math.max(300, this.data.length * 10);
        this.canvas.width = width;
        const h = this.canvas.height;
        const ctx = this.ctx;
        const stepWidth = 10;

        // Background
        ctx.fillStyle = '#222';
        ctx.fillRect(0, 0, width, h);

        // Zero line (if min < 0)
        if (this.min < 0 && this.max > 0) {
            const zeroY = h - ((0 - this.min) / this.range) * h;
            ctx.beginPath();
            ctx.moveTo(0, zeroY);
            ctx.lineTo(width, zeroY);
            ctx.strokeStyle = '#444';
            ctx.stroke();
        }

        // Bars
        ctx.fillStyle = this.color;
        this.data.forEach((val, i) => {
            const x = i * stepWidth;
            // Calculate height and y

            // Normalize value 0..1
            const n = (val - this.min) / this.range;

            const barHeight = Math.max(2, n * h);

            if (this.min < 0) {
                 // Zero center mode
                 const zeroY = h - ((0 - this.min) / this.range) * h;
                 const y = h - ((val - this.min) / this.range) * h;

                 // Draw line from zeroY to y
                 ctx.beginPath();
                 ctx.moveTo(x + stepWidth/2, zeroY);
                 ctx.lineTo(x + stepWidth/2, y);
                 ctx.strokeStyle = this.color;
                 ctx.lineWidth = stepWidth - 2;
                 ctx.stroke();

                 // Cap point
                 ctx.fillStyle = '#fff';
                 ctx.fillRect(x + 1, y - 1, stepWidth - 2, 3);
            } else {
                // Bottom up mode
                const barH = (n * h);
                ctx.fillRect(x + 1, h - barH, stepWidth - 2, barH);
            }
        });

        // Grid
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 1;
        for (let i = 0; i <= this.data.length; i++) {
            ctx.beginPath();
            ctx.moveTo(i * stepWidth, 0);
            ctx.lineTo(i * stepWidth, h);
            ctx.stroke();
        }

        this.lblInfo.textContent = `${this.label}: ${this.data.length} steps`;
    }
}

class SFXEditor {
    constructor(rootId) {
        this.root = document.getElementById(rootId);
        this.sfxList = [];
        this.currentIndex = -1;
        this.currentTab = 'vol';
        this.seqEditor = null;
        this.initUI();
    }

    initUI() {
        // Build the split view
        this.root.innerHTML = `
            <div class="sfx-container">
                <div class="sfx-list-panel">
                    <h3>Sound Effects</h3>
                    <div class="sfx-buttons">
                         <button id="btn-add-sfx">New</button>
                         <button id="btn-delete-sfx" disabled>Delete</button>
                    </div>
                    <ul class="sfx-list" id="sfx-list"></ul>
                </div>
                <div class="sfx-editor-panel" id="sfx-properties" style="display:none;">
                    <h3>Properties</h3>
                    <div style="display:flex; flex-wrap:wrap; gap:10px;">
                        <div class="sfx-property-row">
                            <label>Name:</label>
                            <input type="text" id="sfx-name" />
                        </div>
                        <div class="sfx-property-row">
                            <label>Channel:</label>
                            <select id="sfx-channel">
                                <option value="0">Pulse 1</option>
                                <option value="1">Pulse 2</option>
                                <option value="2">Triangle</option>
                                <option value="3">Noise</option>
                            </select>
                        </div>
                        <div class="sfx-property-row">
                            <label>Priority:</label>
                            <input type="number" id="sfx-priority" min="0" max="255" title="Higher value interrupts lower" style="width:60px;" />
                        </div>
                        <div class="sfx-property-row">
                            <label>Speed:</label>
                            <input type="number" id="sfx-speed" min="1" max="255" value="1" title="Frames per step" style="width:60px;" />
                        </div>
                        <div class="sfx-property-row">
                            <label>Loop:</label>
                            <input type="checkbox" id="sfx-loop" title="Loop sequence continuously" />
                        </div>
                    </div>

                    <div class="sfx-seq-tabs">
                        <button id="tab-vol" class="active">Volume</button>
                        <button id="tab-pitch">Pitch</button>
                        <button id="tab-duty">Duty</button>
                    </div>

                    <div id="sfx-seq-editor-container" class="sfx-seq-editor">
                        <!-- Sequence Canvas Injected Here -->
                    </div>
                </div>
                <div id="sfx-empty-state" style="flex-grow: 1; display: flex; justify-content: center; align-items: center; color: #666; background: #2e2e2e;">
                    Select or create a Sound Effect
                </div>
            </div>
        `;

        this.bindEvents();
    }

    bindEvents() {
        document.getElementById('btn-add-sfx').onclick = () => this.addSFX();
        document.getElementById('btn-delete-sfx').onclick = () => this.deleteSFX();

        const updateCurrent = () => {
             if (this.currentIndex >= 0 && this.sfxList[this.currentIndex]) {
                 this.sfxList[this.currentIndex].name = document.getElementById('sfx-name').value;
                 this.sfxList[this.currentIndex].channel = parseInt(document.getElementById('sfx-channel').value);
                 this.sfxList[this.currentIndex].priority = parseInt(document.getElementById('sfx-priority').value);
                 this.sfxList[this.currentIndex].speed = parseInt(document.getElementById('sfx-speed').value);
                 this.sfxList[this.currentIndex].does_loop = document.getElementById('sfx-loop').checked;

                 this.renderList(); // To update name
             }
        };

        document.getElementById('sfx-name').onchange = updateCurrent;
        document.getElementById('sfx-channel').onchange = updateCurrent;
        document.getElementById('sfx-priority').onchange = updateCurrent;
        document.getElementById('sfx-speed').onchange = updateCurrent;
        document.getElementById('sfx-loop').onchange = updateCurrent;

        // Tabs
        ['vol', 'pitch', 'duty'].forEach(type => {
            document.getElementById(`tab-${type}`).onclick = () => {
                this.currentTab = type;
                this.updateTabs();
                this.renderSequenceEditor();
            };
        });
    }

    updateTabs() {
        ['vol', 'pitch', 'duty'].forEach(type => {
            const btn = document.getElementById(`tab-${type}`);
            if (type === this.currentTab) btn.classList.add('active');
            else btn.classList.remove('active');
        });
    }

    loadData(data) {
        // Ensure data is an array
        this.sfxList = Array.isArray(data) ? data : [];

        // Validate/Fix data if fields missing
        this.sfxList.forEach(sfx => {
            if (sfx.does_loop === undefined) sfx.does_loop = false;
            if (sfx.priority === undefined) sfx.priority = 10;
            if (sfx.speed === undefined) sfx.speed = 1;
            if (sfx.channel === undefined) sfx.channel = 0;
            if (!sfx.vol_sequence) sfx.vol_sequence = [];
            if (!sfx.pitch_sequence) sfx.pitch_sequence = [];
            if (!sfx.duty_sequence) sfx.duty_sequence = [];
        });

        this.currentIndex = -1;
        this.renderList();
        this.updateEditor();
    }

    getData() {
        return this.sfxList;
    }

    renderList() {
        const list = document.getElementById('sfx-list');
        list.innerHTML = '';
        this.sfxList.forEach((sfx, i) => {
            const li = document.createElement('li');
            li.className = 'sfx-list-item';
            if (i === this.currentIndex) li.classList.add('active');
            li.textContent = sfx.name;
            li.onclick = () => {
                this.currentIndex = i;
                this.renderList();
                this.updateEditor();
            };
            list.appendChild(li);
        });

        document.getElementById('btn-delete-sfx').disabled = (this.currentIndex < 0);
    }

    updateEditor() {
        const panel = document.getElementById('sfx-properties');
        const emptyState = document.getElementById('sfx-empty-state');

        if (this.currentIndex >= 0 && this.sfxList[this.currentIndex]) {
            panel.style.display = 'flex';
            emptyState.style.display = 'none';

            const sfx = this.sfxList[this.currentIndex];
            document.getElementById('sfx-name').value = sfx.name;
            document.getElementById('sfx-channel').value = sfx.channel;
            document.getElementById('sfx-priority').value = sfx.priority;
            document.getElementById('sfx-speed').value = sfx.speed;
            document.getElementById('sfx-loop').checked = sfx.does_loop;

            this.updateTabs();
            this.renderSequenceEditor();
        } else {
            panel.style.display = 'none';
            emptyState.style.display = 'flex';

            // Ensure editor is cleaned up
            if (this.seqEditor) {
                this.seqEditor.destroy();
                this.seqEditor = null;
            }
        }
    }

    renderSequenceEditor() {
        // Cleanup old
        if (this.seqEditor) {
            this.seqEditor.destroy();
            this.seqEditor = null;
        }

        if (this.currentIndex < 0) return;

        const container = document.getElementById('sfx-seq-editor-container');
        const sfx = this.sfxList[this.currentIndex];

        if (this.currentTab === 'vol') {
            this.seqEditor = new SequenceCanvas(container, sfx.vol_sequence, 0, 15, '#4CAF50', 'Volume', () => {});
        } else if (this.currentTab === 'pitch') {
            this.seqEditor = new SequenceCanvas(container, sfx.pitch_sequence, -32, 32, '#E91E63', 'Pitch Offset', () => {});
        } else if (this.currentTab === 'duty') {
            this.seqEditor = new SequenceCanvas(container, sfx.duty_sequence, 0, 3, '#2196F3', 'Duty Cycle', () => {});
        }
    }

    addSFX() {
        const newSFX = {
            name: `SFX ${this.sfxList.length + 1}`,
            channel: 0,
            priority: 10,
            speed: 1,
            does_loop: false,
            vol_sequence: [15, 12, 8, 4, 0],
            pitch_sequence: [0, 0, 0, 0, 0],
            duty_sequence: [0, 0, 0, 0, 0]
        };
        this.sfxList.push(newSFX);
        this.currentIndex = this.sfxList.length - 1;
        this.renderList();
        this.updateEditor();
    }

    deleteSFX() {
        if (this.currentIndex >= 0) {
            if (confirm(`Delete ${this.sfxList[this.currentIndex].name}?`)) {
                this.sfxList.splice(this.currentIndex, 1);
                this.currentIndex = -1;
                this.renderList();
                this.updateEditor();
            }
        }
    }
}
window.SFXEditor = SFXEditor;
