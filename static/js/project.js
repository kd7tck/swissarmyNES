// Project Management Logic

class ProjectManager {
    constructor() {
        this.projectList = document.getElementById('project-list');
        this.fileList = document.getElementById('file-list');
        this.currentProject = null;
        this.currentFile = null;
        this.assets = null; // Store project assets here

        this.init();
    }

    init() {
        this.refreshProjects();

        // Bind buttons
        document.getElementById('btn-new-project').addEventListener('click', () => this.createNewProject());
        document.getElementById('btn-save-project').addEventListener('click', () => this.saveCurrentProject());
        document.getElementById('btn-new-file').addEventListener('click', () => this.createNewFile());
    }

    async refreshProjects() {
        try {
            const response = await fetch('/api/projects');
            if (!response.ok) throw new Error('Failed to list projects');

            const projects = await response.json();
            this.renderProjectList(projects);
        } catch (err) {
            console.error(err);
            alert('Error fetching projects: ' + err.message);
        }
    }

    renderProjectList(projects) {
        this.projectList.innerHTML = '';
        projects.forEach(name => {
            const li = document.createElement('li');
            li.textContent = name;
            li.onclick = () => this.loadProject(name);
            this.projectList.appendChild(li);
        });
    }

    async createNewProject() {
        const name = prompt("Enter project name (alphanumeric, _, -):");
        if (!name) return;

        try {
            const response = await fetch('/api/projects', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name })
            });

            if (!response.ok) {
                const txt = await response.text();
                throw new Error(txt);
            }

            // Refresh and load
            await this.refreshProjects();
            await this.loadProject(name);

        } catch (err) {
            alert('Error creating project: ' + err.message);
        }
    }

    async loadProject(name) {
        try {
            const response = await fetch(`/api/projects/${name}`);
            if (!response.ok) throw new Error('Failed to load project');

            const project = await response.json();

            this.currentProject = name;
            this.assets = project.assets || { chr_bank: [], palettes: [], nametables: [] };

            // Show file explorer
            const explorer = document.getElementById('file-explorer');
            if (explorer) explorer.style.display = 'block';

            await this.refreshFiles();

            // Load main.swiss by default
            await this.loadFile('main.swiss');

            // Update UI title or status
            document.getElementById('current-project-name').textContent = name;

            // Load Audio Tracks
            if (window.audioTracker) {
                window.audioTracker.loadData(this.assets || {});
            }

            // Load SFX
            if (window.sfxEditor) {
                window.sfxEditor.loadData(this.assets.sound_effects || []);
            }

            // Dispatch an event to let other components know the project loaded
            window.dispatchEvent(new CustomEvent('project-loaded', { detail: { assets: this.assets } }));

        } catch (err) {
            alert('Error loading project: ' + err.message);
        }
    }

    async refreshFiles() {
        if (!this.currentProject) return;
        try {
            const response = await fetch(`/api/projects/${this.currentProject}/files`);
            if (!response.ok) throw new Error('Failed to list files');
            const files = await response.json();
            this.renderFileList(files);
        } catch (err) {
            console.error(err);
        }
    }

    renderFileList(files) {
        this.fileList.innerHTML = '';
        files.forEach(filename => {
            const li = document.createElement('li');
            li.textContent = filename;
            if (filename === this.currentFile) {
                li.classList.add('active');
            }
            li.onclick = () => {
                this.loadFile(filename);
            };
            this.fileList.appendChild(li);
        });
    }

    async loadFile(filename) {
        if (!this.currentProject) return;
        try {
            const response = await fetch(`/api/projects/${this.currentProject}/files/${filename}`);
            if (!response.ok) throw new Error('Failed to load file');
            const content = await response.text();

            const editor = document.getElementById('code-editor');
            if (editor) {
                editor.value = content;
                editor.dispatchEvent(new Event('input'));
            }

            this.currentFile = filename;

            // Re-render list to update active class
            Array.from(this.fileList.children).forEach(li => {
                if (li.textContent === filename) li.classList.add('active');
                else li.classList.remove('active');
            });

        } catch (err) {
            alert('Error loading file: ' + err.message);
        }
    }

    async createNewFile() {
        if (!this.currentProject) return;
        const filename = prompt("Enter file name (e.g., lib.swiss):");
        if (!filename) return;

        try {
            const response = await fetch(`/api/projects/${this.currentProject}/files`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ filename: filename, content: "" })
            });

            if (!response.ok) {
                 const txt = await response.text();
                 throw new Error(txt);
            }

            await this.refreshFiles();
            await this.loadFile(filename);
        } catch (err) {
            alert('Error creating file: ' + err.message);
        }
    }

    async saveCurrentProject(silent = false) {
        if (!this.currentProject) {
            if (!silent) alert("No project loaded!");
            return;
        }

        const editor = document.getElementById('code-editor');
        const source = editor.value;

        // 1. Save Current File
        if (this.currentFile) {
            try {
                const response = await fetch(`/api/projects/${this.currentProject}/files/${this.currentFile}`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ content: source })
                });
                if (!response.ok) throw new Error('Failed to save file');
            } catch (err) {
                alert('Error saving file: ' + err.message);
                return;
            }
        }

        // 2. Save Assets
        // Collect Audio Data
        if (window.audioTracker) {
             const audioData = window.audioTracker.getData();
             this.assets.audio_tracks = audioData.audio_tracks;
             this.assets.samples = audioData.samples;
             this.assets.envelopes = audioData.envelopes;
        }

        // Collect SFX Data
        if (window.sfxEditor) {
            this.assets.sound_effects = window.sfxEditor.getData();
        }

        // Collect World Data
        if (window.worldEditor) {
            this.assets.world = window.worldEditor.getData();
        }

        const payload = {
            assets: this.assets
        };

        try {
            // We use save_project just for assets now
            const response = await fetch(`/api/projects/${this.currentProject}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (!response.ok) throw new Error('Failed to save assets');

            if (!silent) alert('Project saved!');
        } catch (err) {
            alert('Error saving project assets: ' + err.message);
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    window.projectManager = new ProjectManager();
});
