from playwright.sync_api import sync_playwright

def verify_audio_tracker():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Navigate to the app
        page.goto("http://localhost:3000")

        # Click on the Audio tab
        page.click("a[href='#audio']")

        # Wait for the tracker to be visible
        page.wait_for_selector("#audio-tracker-root")

        # Toggle some notes
        # Row 10, Col 0
        cell1 = page.locator(".tracker-cell[data-row='10'][data-col='0']")
        cell1.click()

        # Row 10, Col 4
        cell2 = page.locator(".tracker-cell[data-row='10'][data-col='4']")
        cell2.click()

        # Take a screenshot
        page.screenshot(path="verification/audio_tracker.png")

        browser.close()

if __name__ == "__main__":
    verify_audio_tracker()
