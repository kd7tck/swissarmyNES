document.addEventListener('DOMContentLoaded', () => {
    const navItems = document.querySelectorAll('.nav-item');
    const views = document.querySelectorAll('.view');

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
});
