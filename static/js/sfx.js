class SFXEditor {
    constructor(rootId) {
        this.root = document.getElementById(rootId);
        this.sfxList = [];
        this.currentIndex = -1;
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
                        <input type="number" id="sfx-priority" min="0" max="255" title="Higher value interrupts lower" />
                    </div>
                     <div class="sfx-property-row">
                        <label>Speed:</label>
                        <input type="number" id="sfx-speed" min="1" max="255" value="1" title="Frames per step" />
                    </div>
                    <div class="sfx-property-row">
                        <label>Loop:</label>
                        <input type="checkbox" id="sfx-loop" title="Loop sequence continuously" />
                    </div>

                    <div style="margin-top: auto; padding: 10px; background: #222; border: 1px dashed #555; color: #888; text-align: center;">
                        Sequence Editors coming in Phase 25c.
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
        } else {
            panel.style.display = 'none';
            emptyState.style.display = 'flex';
        }
    }

    addSFX() {
        const newSFX = {
            name: `SFX ${this.sfxList.length + 1}`,
            channel: 0,
            priority: 10,
            speed: 1,
            does_loop: false,
            vol_sequence: [],
            pitch_sequence: [],
            duty_sequence: []
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
