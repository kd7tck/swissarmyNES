from playwright.sync_api import Page, expect, sync_playwright
import time

def verify_compile_flow(page: Page):
    # Enable console logging
    page.on("console", lambda msg: print(f"BROWSER CONSOLE: {msg.text}"))
    page.on("pageerror", lambda err: print(f"BROWSER ERROR: {err}"))

    # 1. Arrange: Go to the app homepage.
    page.goto("http://localhost:3000")

    # Wait for app to be ready
    page.wait_for_selector("#btn-new-project")

    # 2. Interact: Create New Project
    # We need to ensure window.prompt returns "TestProject"
    # We use add_init_script for future navigations, but here we are already loaded.
    # So we just assign it on the current page.
    page.evaluate("window.prompt = () => 'TestProject';")

    print("Clicking New Project...")
    page.locator("#btn-new-project").click()

    # Wait for the project to be created and loaded.
    # The app should:
    # 1. POST /api/projects with name
    # 2. Refresh list
    # 3. Load project
    # 4. Update #current-project-name

    print("Waiting for TestProject to load...")
    expect(page.locator("#current-project-name")).to_have_text("TestProject", timeout=5000)
    print("Project loaded successfully.")

    # 3. Act: Click Compile
    # Handle alert for successful compilation

    def handle_dialog(dialog):
        print(f"Dialog message: {dialog.message}")
        dialog.accept()

    page.on("dialog", handle_dialog)

    print("Clicking Compile...")
    with page.expect_download() as download_info:
        page.locator("#btn-compile").click()

    download = download_info.value
    print(f"Download started: {download.suggested_filename}")

    # 4. Screenshot
    time.sleep(1) # Wait for alert/ui update
    page.screenshot(path="/home/jules/verification/compile_verification.png")
    print("Screenshot saved.")

if __name__ == "__main__":
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        try:
            verify_compile_flow(page)
        finally:
            browser.close()
