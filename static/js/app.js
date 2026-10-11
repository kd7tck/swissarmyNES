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
        if (targetId === 'studio') window.dispatchEvent(new CustomEvent('studio-opened'));
    }
    window.addEventListener('navigate-to', event => navigateTo(event.detail));

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
    if (window.GameStudio) window.gameStudio = new window.GameStudio();

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
    let compileRevision = 0;
    codeEditor?.addEventListener('input', () => { compileRevision++; });
    window.addEventListener('project-loaded', () => { compileRevision++; });
    window.addEventListener('emulator-run-invalidated', () => { compileRevision++; });
    window.addEventListener('game-design-changed', () => { compileRevision++; });
    async function performCompile(download = true, options = {}) {
        if (!codeEditor) return;
        // Reserve the result before autosaving so edits during the save invalidate it.
        const revision = ++compileRevision;
        const projectName = window.projectManager?.currentProject || null;

        // Save project first to ensure files on disk are up to date
        if (window.projectManager) {
            try {
                if (await window.projectManager.saveCurrentProject(true) === false) return;
            } catch (e) {
                console.error("Auto-save failed:", e);
                alert("Auto-save failed: " + e.message);
                return; // Stop compilation if save fails
            }
        }
        if (revision !== compileRevision || projectName !== (window.projectManager?.currentProject || null)) return;

        const source = codeEditor.value;
        // A project compile reads the asset snapshot just persisted by autosave.
        // Live component arrays may have been replaced since the project loaded.
        let assets = projectName ? null : (window.projectManager?.assets || null);
        const preview = options.previewTrackId !== undefined;
        if (preview) {
            const trackId = options.previewTrackId;
            if (!Number.isInteger(trackId) || trackId < 0 || trackId >= (window.audioTracker?.tracks.length || 0)) {
                alert('Choose an existing music track.');
                return;
            }
            assets = {chr_bank: [], palettes: [], nametables: [], ...window.audioTracker.getData()};
        }

        const payload = {
            source: preview ? `SUB Main()\nSound.Play(${options.previewTrackId})\nPPU.Ctrl($80)\nDO\nWAIT_VBLANK\nLOOP WHILE 1\nEND SUB` : options.sourceOverride ?? (projectName ? null : source),
            project_name: preview ? null : projectName,
            assets: assets
        };

        try {
            const response = await fetch('/api/compile', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (response.ok) {
                const json = await response.json();
                if (revision !== compileRevision || projectName !== (window.projectManager?.currentProject || null)) {
                    return; // A newer edit, compilation, or project superseded this result.
                }

                // Decode Base64 ROM
                const binaryString = atob(json.rom);
                const bytes = new Uint8Array(binaryString.length);
                for (let i = 0; i < binaryString.length; i++) {
                    bytes[i] = binaryString.charCodeAt(i);
                }

                if (download) {
                    // Download the blob
                    const blob = new Blob([bytes], { type: 'application/octet-stream' });
                    const url = window.URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.style.display = 'none';
                    a.href = url;
                    a.download = 'game.nes';
                    document.body.appendChild(a);
                    a.click();
                    a.remove();
                    window.URL.revokeObjectURL(url);
                    const mapUrl = URL.createObjectURL(new Blob([JSON.stringify(json.map, null, 2)], {type: 'application/json'}));
                    const mapLink = document.createElement('a');
                    mapLink.href = mapUrl;
                    mapLink.download = 'sourcemap.json';
                    document.body.appendChild(mapLink);
                    mapLink.click();
                    mapLink.remove();
                    URL.revokeObjectURL(mapUrl);
                    alert('Compilation successful. ROM and source map downloads requested.');
                } else {
                    // Send to Emulator
                    const event = new CustomEvent('emulator-load-rom', {
                        detail: {
                            romData: bytes,
                            sourceMap: json.map
                        }
                    });
                    window.dispatchEvent(event);
                }
            } else {
                // Error: Show message
                const text = await response.text();
                if (revision !== compileRevision || projectName !== (window.projectManager?.currentProject || null)) return;
                alert('Compilation Failed:\n' + text);
            }
        } catch (err) {
            if (revision !== compileRevision || projectName !== (window.projectManager?.currentProject || null)) return;
            console.error(err);
            alert('Network Error: ' + err.message);
        }
    }

    // Bind Compile Button
    if (btnCompile) {
        btnCompile.addEventListener('click', () => performCompile(true));
    }

    // Bind Emulator Request
    window.addEventListener('request-compile-and-run', event => performCompile(false, event.detail || {}));
});
