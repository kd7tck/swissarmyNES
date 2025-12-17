from playwright.sync_api import sync_playwright

def verify_audio_ui():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Navigate to the app (assuming port 3000)
        page.goto("http://localhost:3000")

        # Click on Audio Tab
        page.click("a[href='#audio']")

        # Verify Dropdowns
        # Track Selector
        track_select = page.locator("#audio-track-select")
        # Check text of options
        print("Track Options:", track_select.inner_text())

        # Instrument Selector
        inst_select = page.locator("#audio-instrument-select")
        print("Instrument Options:", inst_select.inner_text())

        # Select Track 2
        track_select.select_option("1")

        # Select an instrument
        inst_select.select_option("207") # 75% Decay

        # Take screenshot
        page.screenshot(path="verification/audio_ui.png")

        browser.close()

if __name__ == "__main__":
    verify_audio_ui()
