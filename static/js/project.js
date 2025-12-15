// Project Management Logic

class ProjectManager {
    constructor() {
        this.projectList = document.getElementById('project-list');
        this.currentProject = null;
        this.assets = null; // Store project assets here

        this.init();
    }

    init() {
        this.refreshProjects();

        // Bind buttons
        document.getElementById('btn-new-project').addEventListener('click', () => this.createNewProject());
        document.getElementById('btn-save-project').addEventListener('click', () => this.saveCurrentProject());
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

            // Update Editor
            const editor = document.getElementById('code-editor');
            if (editor) {
                editor.value = project.source;
                // Trigger update event to refresh syntax highlighting
                editor.dispatchEvent(new Event('input'));
            }

            // Update UI title or status
            document.getElementById('current-project-name').textContent = name;

            // Load Audio Tracks
            if (window.audioTracker && this.assets.audio_tracks) {
                window.audioTracker.loadData(this.assets);
            } else if (window.audioTracker) {
                // Initialize empty if no tracks
                window.audioTracker.loadData({ audio_tracks: [] });
            }

            // Dispatch an event to let other components know the project loaded
            window.dispatchEvent(new CustomEvent('project-loaded', { detail: { assets: this.assets } }));

        } catch (err) {
            alert('Error loading project: ' + err.message);
        }
    }

    async saveCurrentProject() {
        if (!this.currentProject) {
            alert("No project loaded!");
            return;
        }

        const editor = document.getElementById('code-editor');
        const source = editor.value;

        // Collect Audio Data
        if (window.audioTracker) {
             const audioData = window.audioTracker.getData();
             this.assets.audio_tracks = audioData.tracks;
        }

        // Dispatch event to gather asset data from editors if needed
        // But we rely on the shared `this.assets` object being mutually updated.
        // Other editors should update window.projectManager.assets directly or listen for changes.

        const payload = {
            source: source,
            assets: this.assets
        };

        try {
            const response = await fetch(`/api/projects/${this.currentProject}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (!response.ok) throw new Error('Failed to save project');

            alert('Project saved!');
        } catch (err) {
            alert('Error saving project: ' + err.message);
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    window.projectManager = new ProjectManager();
});
