from playwright.sync_api import sync_playwright

def verify_project_manager():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Navigate to the app
        page.goto("http://localhost:3000")

        # Wait for the project explorer to appear
        page.wait_for_selector(".project-explorer")

        # Click "New Project" button
        # Handling prompt: we need to handle the dialog before it appears
        page.on("dialog", lambda dialog: dialog.accept("my_test_project"))

        page.click("#btn-new-project")

        # Wait for the project to appear in the list
        page.wait_for_selector("li >> text=my_test_project")

        # Click the project to load it
        page.click("li >> text=my_test_project")

        # Verify current project name is updated
        project_name = page.inner_text("#current-project-name")
        assert "my_test_project" in project_name

        # Take screenshot
        page.screenshot(path="verification/project_manager.png")

        browser.close()

if __name__ == "__main__":
    verify_project_manager()
