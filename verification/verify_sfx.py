from playwright.sync_api import sync_playwright

def run(playwright):
    browser = playwright.chromium.launch(headless=True)
    page = browser.new_page()
    page.goto("http://localhost:3000")

    # Handle the "Enter project name" prompt
    def handle_dialog(dialog):
        dialog.accept("TestProject")

    page.on("dialog", handle_dialog)

    # Click "New Project" (+ button)
    page.click("#btn-new-project")

    # Wait for project to load
    page.wait_for_selector("#current-project-name")

    # Click SFX tab
    page.click("li[data-target='sfx']")

    # Wait for SFX Editor to initialize
    page.wait_for_selector("#sfx-editor-root")

    # Check for Import button
    import_btn = page.query_selector("#btn-import-sfx")
    if not import_btn:
        print("Import button not found!")
        browser.close()
        exit(1)

    # Create a new SFX to see Export button
    page.click("#btn-add-sfx")

    # Check for Export button
    export_btn = page.query_selector("#btn-export-sfx")
    if not export_btn:
        print("Export button not found!")
        browser.close()
        exit(1)

    # Take screenshot
    page.screenshot(path="verification/verification.png")

    print("Verification successful!")
    browser.close()

with sync_playwright() as playwright:
    run(playwright)
