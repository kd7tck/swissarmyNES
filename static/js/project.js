// Project Management Logic

class ProjectManager {
    constructor() {
        this.projectList = document.getElementById('project-list');
        this.fileList = document.getElementById('file-list');
        this.currentProject = null;
        this.currentFile = null;
        this.assets = null; // Store project assets here
        this.loadRevision = 0;
        this.fileLoadRevision = 0;

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
        const revision = ++this.loadRevision;
        try {
            const response = await fetch(`/api/projects/${name}`);
            if (!response.ok) throw new Error('Failed to load project');

            const project = await response.json();

            // Gather the explorer and source before publishing the project. A delayed
            // response must never pair the previous editor text with a new project's assets.
            const [filesResponse, mainResponse] = await Promise.all([
                fetch(`/api/projects/${name}/files`),
                fetch(`/api/projects/${name}/files/main.swiss`)
            ]);
            if (!filesResponse.ok || !mainResponse.ok) throw new Error('Failed to load project files');
            const [files, source] = await Promise.all([filesResponse.json(), mainResponse.text()]);
            if (revision !== this.loadRevision) return false;

            this.currentProject = name;
            this.currentFile = 'main.swiss';
            this.fileLoadRevision++;
            this.assets = project.assets || { chr_bank: [], palettes: [], nametables: [] };

            // Show file explorer
            const explorer = document.getElementById('file-explorer');
            if (explorer) explorer.style.display = 'block';

            this.renderFileList(files);
            const editor = document.getElementById('code-editor');
            if (editor) {
                editor.value = source;
                editor.dispatchEvent(new Event('input'));
            }

            // Update UI title or status
            document.getElementById('current-project-name').textContent = name;

            // Graphics listeners publish the new palette/CHR before sprite previews
            // ask those editors for their rendered bank.
            window.dispatchEvent(new CustomEvent('project-loaded', { detail: { assets: this.assets } }));

            // Load Audio Tracks
            if (window.audioTracker) {
                window.audioTracker.loadData(this.assets || {});
            }

            // Load SFX
            if (window.sfxEditor) {
                window.sfxEditor.loadData(this.assets.sound_effects || []);
            }

            // Load Sprites
            if (window.spriteEditor) {
                window.spriteEditor.loadData(this.assets || {});
            }

            return true;

        } catch (err) {
            if (revision === this.loadRevision) alert('Error loading project: ' + err.message);
            return false;
        }
    }

    async refreshFiles() {
        if (!this.currentProject) return;
        const projectName = this.currentProject;
        try {
            const response = await fetch(`/api/projects/${projectName}/files`);
            if (!response.ok) throw new Error('Failed to list files');
            const files = await response.json();
            if (projectName === this.currentProject) this.renderFileList(files);
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
        const projectName = this.currentProject;
        const revision = ++this.fileLoadRevision;
        try {
            // Switching files preserves the edited source, including included modules.
            if (this.currentFile && this.currentFile !== filename && !await this.saveCurrentProject(true)) return false;
            if (projectName !== this.currentProject || revision !== this.fileLoadRevision) return false;
            const response = await fetch(`/api/projects/${projectName}/files/${filename}`);
            if (!response.ok) throw new Error('Failed to load file');
            const content = await response.text();
            if (projectName !== this.currentProject || revision !== this.fileLoadRevision) return false;
            this.currentFile = filename;

            const editor = document.getElementById('code-editor');
            if (editor) {
                editor.value = content;
                editor.dispatchEvent(new Event('input'));
            }

            // Re-render list to update active class
            Array.from(this.fileList.children).forEach(li => {
                if (li.textContent === filename) li.classList.add('active');
                else li.classList.remove('active');
            });
            return true;

        } catch (err) {
            if (projectName === this.currentProject && revision === this.fileLoadRevision) alert('Error loading file: ' + err.message);
            return false;
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
            return true; // The standalone editor compiles its source directly.
        }

        const editor = document.getElementById('code-editor');
        const source = editor.value;
        const projectName = this.currentProject;
        const filename = this.currentFile;
        // Snapshot all editor data before the first await; a project switch during a
        // save must not write the newly loaded assets into the previous project.
        const assets = JSON.parse(JSON.stringify(this.assets || {}));
        if (window.audioTracker) Object.assign(assets, window.audioTracker.getData());
        if (window.sfxEditor) assets.sound_effects = window.sfxEditor.getData();
        if (window.worldEditor) assets.world = window.worldEditor.getData();
        if (window.spriteEditor) Object.assign(assets, window.spriteEditor.getData());
        const payload = JSON.stringify({ assets });

        // Serialize writes so an older, slower autosave cannot overwrite a newer
        // compile request's source or asset snapshot on disk.
        const previous = this.saveQueue || Promise.resolve();
        this.saveQueue = previous.then(() => this.saveSnapshot(projectName, filename, source, payload, silent));
        return this.saveQueue;
    }

    async saveSnapshot(projectName, filename, source, payload, silent) {

        // 1. Save Current File
        if (filename) {
            try {
                const response = await fetch(`/api/projects/${projectName}/files/${filename}`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ content: source })
                });
                if (!response.ok) throw new Error('Failed to save file');
            } catch (err) {
                alert('Error saving file: ' + err.message);
                return false;
            }
        }

        // 2. Save the asset snapshot to the same project as the source.
        try {
            // We use save_project just for assets now
            const response = await fetch(`/api/projects/${projectName}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: payload
            });

            if (!response.ok) throw new Error('Failed to save assets');

            if (!silent) alert('Project saved!');
            return true;
        } catch (err) {
            alert('Error saving project assets: ' + err.message);
            return false;
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    window.projectManager = new ProjectManager();
});
