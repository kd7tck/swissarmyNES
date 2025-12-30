from playwright.sync_api import sync_playwright

def verify_memory_viewer():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Navigate to the app
        page.goto("http://localhost:3000")

        # Wait for editor to load
        page.wait_for_selector("#code-editor")

        # Mock compilation and run since we don't need real backend compilation for UI verification
        # But we need to open the overlay. The Run button triggers compilation.
        # Alternatively, we can inject JS to simulate the emulator overlay opening.

        # Click the "Run" button. This usually sends a request.
        # If the backend is not fully responding with a valid ROM, the overlay might not open.
        # However, editor.js 'request-compile-and-run' event is listened to by app.js.
        # If compilation fails, we might see an error.

        # Let's try to manually invoke startEmulatorWithRom via console if needed,
        # but let's try the natural flow first.
        # The default editor content is valid code.

        page.click("#btn-run")

        # Wait for overlay
        try:
            page.wait_for_selector("#emulator-overlay", state="visible", timeout=10000)
        except:
            # If standard run fails (maybe due to compilation backend latency or issues in this env),
            # force open the overlay via JS for UI verification.
            page.evaluate("""
                window.editor.createEmulatorOverlay();
                window.editor.toggleMemoryViewer();
            """)

        # Check if Memory button exists
        memory_btn = page.get_by_role("button", name="Memory")
        if memory_btn.is_visible():
            memory_btn.click()

        # Wait for Memory Viewer to appear
        page.wait_for_selector("#memory-viewer", state="visible")

        # Take screenshot
        page.screenshot(path="verification/memory_viewer.png")

        browser.close()

if __name__ == "__main__":
    verify_memory_viewer()
