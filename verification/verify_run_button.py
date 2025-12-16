from playwright.sync_api import sync_playwright

def verify_run_alert():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        page.goto("http://localhost:3000")

        # Click the "Run (Emulator)" button
        # Since the button triggers an alert, we need to handle the dialog

        def handle_dialog(dialog):
            print(f"Dialog message: {dialog.message}")
            if "Emulator integration is coming in Phase 25" in dialog.message:
                print("Verification Successful: Alert message matches expected.")
            else:
                print("Verification Failed: Unexpected alert message.")
            dialog.accept()

        page.on("dialog", handle_dialog)

        page.click("#btn-run")

        # Take a screenshot for good measure (though alert won't show in screenshot usually)
        page.screenshot(path="verification/run_button_check.png")

        browser.close()

if __name__ == "__main__":
    verify_run_alert()
