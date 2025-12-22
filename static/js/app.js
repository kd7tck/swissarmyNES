document.addEventListener('DOMContentLoaded', () => {
    const navItems = document.querySelectorAll('.nav-item');
    const views = document.querySelectorAll('.view');
    const btnCompile = document.getElementById('btn-compile');
    const codeEditor = document.getElementById('code-editor');

    function navigateTo(targetId) {
        // Update Nav
        navItems.forEach(item => {
            if (item.dataset.target === targetId) {
                item.classList.add('active');
            } else {
                item.classList.remove('active');
            }
        });

        // Update View
        views.forEach(view => {
            if (view.id === targetId) {
                view.classList.add('active');
            } else {
                view.classList.remove('active');
            }
        });
    }

    // Handle clicks
    navItems.forEach(item => {
        item.addEventListener('click', (e) => {
            e.preventDefault();
            const target = item.dataset.target;
            navigateTo(target);
            // Update URL hash without scrolling
            history.pushState(null, null, `#${target}`);
        });
    });

    // Initialize Audio Tracker
    if (window.AudioTracker) {
        window.audioTracker = new window.AudioTracker('audio-tracker-root');
    }

    // Initialize SFX Editor
    if (window.SFXEditor) {
        window.sfxEditor = new window.SFXEditor('sfx-editor-root');
    }

    // Initialize Sprite Editor
    if (window.SpriteEditor) {
        window.spriteEditor = new window.SpriteEditor();
    }

    // Handle initial load based on hash
    const initialHash = window.location.hash.replace('#', '');
    if (initialHash && document.getElementById(initialHash)) {
        navigateTo(initialHash);
    } else {
        // Default to code
        navigateTo('code');
    }

    // Handle back/forward buttons
    window.addEventListener('popstate', () => {
         const hash = window.location.hash.replace('#', '');
         if (hash && document.getElementById(hash)) {
             navigateTo(hash);
         } else {
             navigateTo('code');
         }
    });

    // Compilation Logic
    async function performCompile(download = true) {
        if (!codeEditor) return;

        // Save project first to ensure files on disk are up to date
        if (window.projectManager) {
            try {
                await window.projectManager.saveCurrentProject(true);
            } catch (e) {
                console.error("Auto-save failed:", e);
            }
        }

        const source = codeEditor.value;
        const assets = window.projectManager ? window.projectManager.assets : null;
        const projectName = window.projectManager ? window.projectManager.currentProject : null;

        const payload = {
            source: projectName ? null : source, // Use editor content only if no project
            project_name: projectName,
            assets: assets
        };

        try {
            const response = await fetch('/api/compile', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (response.ok) {
                const blob = await response.blob();
                if (download) {
                    // Download the blob
                    const url = window.URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.style.display = 'none';
                    a.href = url;
                    a.download = 'game.nes';
                    document.body.appendChild(a);
                    a.click();
                    window.URL.revokeObjectURL(url);
                    alert('Compilation Successful! ROM downloaded.');
                } else {
                    // Send to Emulator
                    const arrayBuffer = await blob.arrayBuffer();
                    const romData = new Uint8Array(arrayBuffer);
                    const event = new CustomEvent('emulator-load-rom', { detail: romData });
                    window.dispatchEvent(event);
                }
            } else {
                // Error: Show message
                const text = await response.text();
                alert('Compilation Failed:\n' + text);
            }
        } catch (err) {
            console.error(err);
            alert('Network Error: ' + err.message);
        }
    }

    // Bind Compile Button
    if (btnCompile) {
        btnCompile.addEventListener('click', () => performCompile(true));
    }

    // Bind Emulator Request
    window.addEventListener('request-compile-and-run', () => performCompile(false));
});
