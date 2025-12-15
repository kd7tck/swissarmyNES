from playwright.sync_api import sync_playwright

def verify_chr_editor():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Navigate to the app
        page.goto("http://localhost:3000")

        # Click on "Graphics" tab link
        page.get_by_role("link", name="Graphics").click()

        # Check if Palette Editor is visible (System Colors)
        page.wait_for_selector(".system-palette")

        # Check if CHR Editor is visible (Controls)
        page.wait_for_selector(".chr-controls")

        # Wait a bit for project load event
        page.wait_for_timeout(1000)

        # Take screenshot of the Graphics section
        page.screenshot(path="verification/chr_editor.png")

        browser.close()

if __name__ == "__main__":
    verify_chr_editor()
