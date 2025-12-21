from playwright.sync_api import sync_playwright, expect

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context()
        page = context.new_page()

        try:
            # Navigate to app
            page.goto("http://localhost:3000")

            # Create a new project to ensure clean state
            # Wait for project explorer
            expect(page.get_by_text("Projects")).to_be_visible()

            # Click New Project
            # The prompt dialog is tricky in headless.
            # We can handle the dialog event.
            page.on("dialog", lambda dialog: dialog.accept("verify_screens"))

            page.click("#btn-new-project")

            # Wait for project to load (project name appears)
            expect(page.locator("#current-project-name")).to_have_text("verify_screens")

            # Navigate to Screens tab
            page.click("a[href='#screens']")

            # Check if Screen Editor is visible
            expect(page.locator("#screens")).to_be_visible()
            expect(page.locator("h1", has_text="Screen Editor")).to_be_visible()

            # Check for empty list
            expect(page.locator(".screen-list")).to_be_empty()

            # Add a screen
            # Button is in the header of the list panel
            # It has text '+'
            page.click(".screen-list-header button[title='Add Screen']")

            # Expect a new item in the list
            expect(page.locator(".screen-list-item")).to_have_count(1)
            expect(page.locator(".screen-list-item")).to_contain_text("Screen 0")

            # Select it (it should be selected automatically, but let's click to be sure)
            page.click(".screen-list-item")

            # Check if canvas exists
            expect(page.locator(".screen-canvas")).to_be_visible()

            # Draw something?
            # We can try clicking on the canvas.
            # Metatile 0 is selected by default. Let's try to select Metatile 1 if available?
            # Newly created project has empty metatiles list?
            # The default assets in `create_project` has `metatiles: vec![]`.
            # So the palette will be empty.

            # We need to add a metatile first to verify drawing.
            # Navigate to Map tab (Metatile Editor is there)
            page.click("a[href='#map']")
            expect(page.locator("#metatile-editor-root")).to_be_visible()

            # Add a metatile
            # The Metatile Editor has a '+' button.
            # We need to find it. It's in the left column.
            # Assuming MetatileEditor structure...
            # The script might be fragile here if I don't use specific selectors.
            # But let's just take a screenshot of the Screen Editor with one screen.

            page.click("a[href='#screens']")

            # Screenshot
            page.screenshot(path="verification/screen_editor.png")
            print("Screenshot saved to verification/screen_editor.png")

        except Exception as e:
            print(f"Error: {e}")
            page.screenshot(path="verification/error.png")
        finally:
            browser.close()

if __name__ == "__main__":
    run()
