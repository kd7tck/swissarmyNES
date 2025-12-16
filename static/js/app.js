document.addEventListener('DOMContentLoaded', () => {
    const navItems = document.querySelectorAll('.nav-item');
    const views = document.querySelectorAll('.view');
    const btnCompile = document.getElementById('btn-compile');
    const btnRun = document.getElementById('btn-run');
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

    // Handle initial load based on hash
    // Initialize Audio Tracker
    if (window.AudioTracker) {
        window.audioTracker = new window.AudioTracker('audio-tracker-root');
    }

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

    // Handle Compile
    if (btnCompile && codeEditor) {
        btnCompile.addEventListener('click', async () => {
            const source = codeEditor.value;
            // Get assets from the ProjectManager
            const assets = window.projectManager ? window.projectManager.assets : null;

            const payload = {
                source: source,
                assets: assets
            };

            try {
                const response = await fetch('/api/compile', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });

                if (response.ok) {
                    // Success: Download the blob
                    const blob = await response.blob();
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
                    // Error: Show message
                    const text = await response.text();
                    alert('Compilation Failed:\n' + text);
                }
            } catch (err) {
                console.error(err);
                alert('Network Error: ' + err.message);
            }
        });
    }

    // Handle Run (Emulator)
    if (btnRun) {
        btnRun.addEventListener('click', () => {
            // In the future, this will launch the WASM emulator.
            // For now, we alert the user.
            alert('Emulator integration is coming in Phase 25. Please compile and run the downloaded ROM in your preferred NES emulator.');
        });
    }
});
