from playwright.sync_api import sync_playwright, expect
import time

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={'width': 1280, 'height': 800})
        page = context.new_page()

        try:
            print("Navigating to app...")
            page.goto("http://localhost:3000")

            # Wait for app to load
            expect(page.locator("#app")).to_be_visible(timeout=30000)

            # Create a project to enable full functionality
            print("Creating project...")
            project_name = f"TestProject_{int(time.time())}"
            page.on("dialog", lambda dialog: dialog.accept(project_name))
            page.get_by_title("New Project").click()

            # Wait for project to load (file explorer appears)
            expect(page.locator("#file-explorer")).to_be_visible(timeout=10000)

            print("Navigating to SFX tab...")
            # Click SFX Tab
            page.get_by_role("link", name="SFX").click()

            # Verify SFX section is active
            expect(page.locator("#sfx")).to_have_class("view active")

            print("Creating new SFX...")
            # Click New SFX button
            page.click("#btn-add-sfx")

            # Verify list item created
            expect(page.locator("#sfx-list li")).to_have_count(1)

            print("Editing Properties...")
            # Edit Properties
            page.fill("#sfx-name", "Jump Sound")
            page.select_option("#sfx-channel", "3") # Noise
            page.fill("#sfx-priority", "50")
            page.fill("#sfx-speed", "2")
            page.check("#sfx-loop")

            # Verify List item updated (it updates on change)
            expect(page.locator("#sfx-list li").first).to_have_text("Jump Sound")

            # Take Screenshot
            print("Taking screenshot...")
            time.sleep(1)
            page.screenshot(path="verification/sfx_editor.png")
            print("Screenshot saved to verification/sfx_editor.png")

        except Exception as e:
            print(f"Error: {e}")
            page.screenshot(path="verification/error.png")
        finally:
            browser.close()

if __name__ == "__main__":
    run()
