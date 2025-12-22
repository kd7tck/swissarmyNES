from playwright.sync_api import sync_playwright
import time

def run(playwright):
    browser = playwright.chromium.launch(headless=True)
    context = browser.new_context()
    page = context.new_page()

    # Capture console logs
    logs = []
    page.on("console", lambda msg: logs.append(msg.text))

    print("Navigating to home...")
    try:
        page.goto("http://localhost:3000", timeout=10000)
    except Exception as e:
        print(f"Failed to load page: {e}")
        browser.close()
        return

    print("Waiting for editor...")
    try:
        page.wait_for_selector("#code-editor", timeout=10000)
    except:
        print("Editor didn't load.")
        browser.close()
        return

    print("Clicking Run...")
    # Ensure button exists
    if page.is_visible("#btn-run"):
        page.click("#btn-run")
    else:
        print("Run button not visible")
        browser.close()
        return

    print("Waiting for emulator overlay...")
    try:
        page.wait_for_selector("#emulator-overlay", timeout=10000)
    except:
        print("Overlay didn't appear.")
        browser.close()
        return

    print("Waiting for emulation loop...")
    time.sleep(5) # Wait for 5 seconds

    print("Checking logs...")
    debug_logs = [l for l in logs if "[DEBUG]" in l]
    if debug_logs:
        print("Found debug logs:")
        for l in debug_logs:
            print(l)
    else:
        print("No debug logs found!")
        print("All logs:", logs)

    page.screenshot(path="verification/debug_protocol.png")
    browser.close()

with sync_playwright() as playwright:
    run(playwright)
