class EnvelopeEditor {
    constructor(rootId) {
        this.root = document.getElementById(rootId);
        this.envelopes = [];
        this.currentEnvelopeIndex = -1;
        this.initUI();
    }

    initUI() {
        this.root.innerHTML = `
            <div style="display: flex; gap: 10px; height: 100%;">
                <div style="width: 200px; border-right: 1px solid #555;">
                    <div style="margin-bottom: 10px;">
                        <button id="btn-new-envelope">New</button>
                        <button id="btn-del-envelope">Del</button>
                    </div>
                    <ul id="envelope-list" style="list-style: none; padding: 0;"></ul>
                </div>
                <div style="flex: 1; display: flex; flex-direction: column;">
                    <div style="margin-bottom: 10px;">
                        <label>Name: <input type="text" id="env-name" style="background:#333; color:white; border:1px solid #555;"></label>
                        <label>Loop Index: <input type="number" id="env-loop" min="-1" max="63" value="-1" style="width:50px; background:#333; color:white; border:1px solid #555;"></label>
                        <span style="font-size: 0.8em; color: #aaa;">(-1 = No Loop)</span>
                    </div>
                    <div id="env-graph" style="flex: 1; background: #222; position: relative; border: 1px solid #444; overflow-x: auto;">
                        <!-- Graph -->
                    </div>
                    <div style="margin-top: 10px; display: flex; gap: 10px; align-items: center;">
                        <button id="btn-add-step">Add Step</button>
                        <button id="btn-del-step">Del Step</button>
                        <label>Value: <input type="number" id="step-value" style="width:50px;"></label>
                        <label>Duration: <input type="number" id="step-dur" min="1" value="1" style="width:50px;"></label>
                        <span id="step-info" style="color: #aaa; font-size: 0.8em;">Select a step to edit</span>
                    </div>
                </div>
            </div>
        `;

        document.getElementById('btn-new-envelope').onclick = () => this.createEnvelope();
        document.getElementById('btn-del-envelope').onclick = () => this.deleteEnvelope();
        document.getElementById('btn-add-step').onclick = () => this.addStep();
        document.getElementById('btn-del-step').onclick = () => this.deleteStep();

        document.getElementById('env-name').onchange = (e) => {
            if(this.currentEnvelopeIndex >= 0) {
                this.envelopes[this.currentEnvelopeIndex].name = e.target.value;
                this.renderList();
            }
        };
        document.getElementById('env-loop').onchange = (e) => {
            if(this.currentEnvelopeIndex >= 0) {
                let val = parseInt(e.target.value);
                this.envelopes[this.currentEnvelopeIndex].loop_index = (val < 0) ? null : val;
                this.renderGraph(); // Re-render to show loop marker
            }
        };
    }

    createEnvelope() {
        this.envelopes.push({
            name: `Envelope ${this.envelopes.length + 1}`,
            steps: [[15, 8]],
            loop_index: null
        });
        this.currentEnvelopeIndex = this.envelopes.length - 1;
        this.renderList();
        this.renderGraph();
        // Notify AudioTracker to update dropdowns?
        // We can dispatch an event.
        window.dispatchEvent(new Event('envelopes-updated'));
    }

    deleteEnvelope() {
        if(this.currentEnvelopeIndex >= 0) {
            this.envelopes.splice(this.currentEnvelopeIndex, 1);
            this.currentEnvelopeIndex = -1;
            this.renderList();
            this.renderGraph();
            window.dispatchEvent(new Event('envelopes-updated'));
        }
    }

    renderList() {
        const list = document.getElementById('envelope-list');
        list.innerHTML = '';
        this.envelopes.forEach((env, i) => {
            const li = document.createElement('li');
            li.textContent = env.name;
            li.style.padding = '5px';
            li.style.cursor = 'pointer';
            li.style.borderBottom = '1px solid #444';
            li.style.backgroundColor = (i === this.currentEnvelopeIndex) ? '#444' : 'transparent';
            li.onclick = () => {
                this.currentEnvelopeIndex = i;
                this.renderList();
                this.renderGraph();
            };
            list.appendChild(li);
        });
    }

    renderGraph() {
        const graph = document.getElementById('env-graph');
        graph.innerHTML = '';
        document.getElementById('step-info').textContent = 'Select a step to edit';

        if (this.currentEnvelopeIndex < 0) return;

        const env = this.envelopes[this.currentEnvelopeIndex];
        document.getElementById('env-name').value = env.name;
        document.getElementById('env-loop').value = (env.loop_index === null || env.loop_index === undefined) ? -1 : env.loop_index;

        let x = 10;
        const scale = 5;

        // Axis Line
        const axis = document.createElement('div');
        axis.style.position = 'absolute';
        axis.style.left = '0';
        axis.style.right = '0';
        axis.style.bottom = '100px'; // Zero line
        axis.style.height = '1px';
        axis.style.background = '#555';
        graph.appendChild(axis);

        env.steps.forEach((step, i) => {
            const val = step[0];
            const dur = step[1];

            const bar = document.createElement('div');
            bar.style.position = 'absolute';
            bar.style.left = `${x}px`;

            const height = Math.abs(val) * scale;
            bar.style.height = `${height}px`;
            bar.style.width = `${Math.max(2, dur * 5 - 1)}px`; // Scale duration width

            if (val >= 0) {
                bar.style.bottom = '100px';
                bar.style.background = '#4CAF50';
            } else {
                bar.style.top = `${graph.offsetHeight - 100}px`; // From zero line downwards
                bar.style.background = '#F44336';
            }

            bar.title = `Step ${i}: Val ${val}, Dur ${dur}`;
            bar.style.cursor = 'pointer';

            bar.onclick = (e) => {
                e.stopPropagation(); // Prevent deselect?
                this.selectStep(i);
            };

            if (i === env.loop_index) {
                bar.style.borderTop = '2px solid yellow';
            }

            graph.appendChild(bar);
            x += Math.max(2, dur * 5);
        });
    }

    selectStep(index) {
        const env = this.envelopes[this.currentEnvelopeIndex];
        const step = env.steps[index];

        document.getElementById('step-info').textContent = `Editing Step ${index}`;
        document.getElementById('step-value').value = step[0];
        document.getElementById('step-dur').value = step[1];

        document.getElementById('step-value').onchange = (e) => {
            step[0] = parseInt(e.target.value);
            this.renderGraph();
        };
        document.getElementById('step-dur').onchange = (e) => {
            step[1] = Math.max(1, parseInt(e.target.value));
            this.renderGraph();
        };

        this.selectedStepIndex = index;
    }

    addStep() {
        if (this.currentEnvelopeIndex < 0) return;
        this.envelopes[this.currentEnvelopeIndex].steps.push([10, 8]);
        this.renderGraph();
    }

    deleteStep() {
        if (this.currentEnvelopeIndex < 0 || this.selectedStepIndex === undefined) return;
        this.envelopes[this.currentEnvelopeIndex].steps.splice(this.selectedStepIndex, 1);
        this.selectedStepIndex = undefined;
        this.renderGraph();
    }

    getEnvelopes() {
        return this.envelopes;
    }

    loadEnvelopes(data) {
        this.envelopes = data || [];
        // Ensure steps are arrays (tuples)
        this.envelopes.forEach(e => {
            if (!e.steps) e.steps = [];
        });
        this.renderList();
    }
}

window.EnvelopeEditor = EnvelopeEditor;
