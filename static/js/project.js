// Project Management Logic

class ProjectManager {
    constructor() {
        this.projectList = document.getElementById('project-list');
        this.currentProject = null;

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

            // Update Editor
            const editor = document.getElementById('code-editor');
            if (editor) {
                editor.value = project.source;
                // Trigger update event to refresh syntax highlighting
                editor.dispatchEvent(new Event('input'));
            }

            // Update UI title or status
            document.getElementById('current-project-name').textContent = name;

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

        try {
            const response = await fetch(`/api/projects/${this.currentProject}`, {
                method: 'POST',
                body: source
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
