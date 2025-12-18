from playwright.sync_api import sync_playwright, expect
import re
import time

def run(playwright):
    browser = playwright.chromium.launch(headless=True)
    context = browser.new_context()
    page = context.new_page()

    # 1. Load the app
    try:
        page.goto("http://localhost:3000")
    except Exception as e:
        print(f"Failed to load page: {e}")
        return

    # 2. Create a new project
    # Trigger prompt handling
    def handle_dialog(dialog):
        print(f"Dialog message: {dialog.message}")
        if "project name" in dialog.message:
            dialog.accept("MyMultiFileProject")
        elif "file name" in dialog.message:
            dialog.accept("lib.swiss")
        else:
            dialog.accept()

    page.on("dialog", handle_dialog)

    # Click New Project
    page.click("#btn-new-project")

    # Wait for project to load and Files section to appear
    expect(page.locator("#current-project-name")).to_have_text("MyMultiFileProject")
    expect(page.locator("#file-explorer")).to_be_visible()

    # Verify main.swiss is in list
    expect(page.locator("#file-list")).to_contain_text("main.swiss")

    # 3. Create a new file
    page.click("#btn-new-file")

    # Wait for lib.swiss to appear
    expect(page.locator("#file-list")).to_contain_text("lib.swiss")

    # Verify lib.swiss is active (using simple class check)
    # Note: element handle logic
    lib_file = page.locator("#file-list li", has_text="lib.swiss")
    expect(lib_file).to_have_class(re.compile(r"active"))

    # 4. Take screenshot
    page.screenshot(path="/home/jules/verification/multifile.png")
    print("Screenshot taken at /home/jules/verification/multifile.png")

    browser.close()

if __name__ == "__main__":
    with sync_playwright() as playwright:
        run(playwright)
