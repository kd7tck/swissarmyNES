
from playwright.sync_api import sync_playwright

def verify_map_editor():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        try:
            # Go to app
            page.goto("http://localhost:3000")

            # Wait for project list and click a project or create one
            # The app likely shows projects or new project button
            # Let's create a new project called 'maptest'
            page.click('#btn-new-project')

            # Handle prompt (playwright handles dialogs?)
            # The app uses `prompt()` for new project name in project.js probably
            # We need to handle the dialog
            def handle_dialog(dialog):
                dialog.accept("maptest")
            page.on("dialog", handle_dialog)

            # Actually the `btn-new-project` click might trigger the prompt immediately
            # Let's try again with event listener set up before click

            # Wait for project to load (nav links become active?)
            # Or just wait a bit.
            page.wait_for_timeout(1000)

            # Navigate to Map tab
            # The link has href="#map"
            page.click('a[href="#map"]')

            # Wait for map editor to be visible
            page.wait_for_selector('#map-editor-root')

            # Check if canvas exists
            page.wait_for_selector('.map-canvas')

            # Take screenshot
            page.screenshot(path="verification/map_editor.png")
            print("Screenshot taken")

        except Exception as e:
            print(f"Error: {e}")
        finally:
            browser.close()

if __name__ == "__main__":
    verify_map_editor()
