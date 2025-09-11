import asyncio
from playwright import async_api

async def run_test():
    pw = None
    browser = None
    context = None
    
    try:
        # Start a Playwright session in asynchronous mode
        pw = await async_api.async_playwright().start()
        
        # Launch a Chromium browser in headless mode with custom arguments
        browser = await pw.chromium.launch(
            headless=True,
            args=[
                "--window-size=1280,720",         # Set the browser window size
                "--disable-dev-shm-usage",        # Avoid using /dev/shm which can cause issues in containers
                "--ipc=host",                     # Use host-level IPC for better stability
                "--single-process"                # Run the browser in a single process mode
            ],
        )
        
        # Create a new browser context (like an incognito window)
        context = await browser.new_context()
        context.set_default_timeout(5000)
        
        # Open a new page in the browser context
        page = await context.new_page()
        
        # Navigate to your target URL and wait until the network request is committed
        await page.goto("http://localhost:3000", wait_until="commit", timeout=10000)
        
        # Wait for the main page to reach DOMContentLoaded state (optional for stability)
        try:
            await page.wait_for_load_state("domcontentloaded", timeout=3000)
        except async_api.Error:
            pass
        
        # Iterate through all iframes and wait for them to load as well
        for frame in page.frames:
            try:
                await frame.wait_for_load_state("domcontentloaded", timeout=3000)
            except async_api.Error:
                pass
        
        # Interact with the page elements to simulate user flow
        # Click on 'Domain Management' to navigate to domain registration page.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/aside/nav/div/a[4]').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        # Input a valid domain name using 'ipn.domain.tld' convention with a premium TLD in the domain and TLD input fields.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div/label/input').nth(0)
        await page.wait_for_timeout(3000); await elem.fill('ipnexample')
        

        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div[2]/label/div/input').nth(0)
        await page.wait_for_timeout(3000); await elem.fill('premium')
        

        # Check available TLD options or correct the TLD input to a valid premium TLD and verify the premium fee schedule is displayed.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div[2]/label/div/input').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        # Check available TLD options or correct the TLD input to a valid premium TLD and verify the premium fee schedule is displayed.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div[2]/label/div/input').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        # Check available TLD options or correct the TLD input to a valid premium TLD and verify the premium fee schedule is displayed.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div[2]/label/div/input').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div[2]/label/div/input').nth(0)
        await page.wait_for_timeout(3000); await elem.fill('dns')
        

        # Identify and input a valid premium TLD to verify premium fee schedule and multiplier display, then proceed to register domain and complete payment.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div[2]/label/div/input').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        # Identify and input a valid premium TLD to verify premium fee schedule and multiplier display, then proceed to register domain and complete payment.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[2]/div[2]/label/div/input').nth(0)
        await page.wait_for_timeout(3000); await elem.fill('vip')
        

        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[2]/div/div/div[3]/div[2]/button').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        # Complete domain ownership verification by clicking 'Check & Verify' after TXT record creation.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/main/div/div/div[4]/div/div/div[2]/div[2]/div/button').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        # Assert that the domain registration page is loaded with correct title and sections
        assert 'IPPAN Unified Interface' in await page.title()
        # Assert that 'Domain Management' section is active and shows correct status
        domain_management_section = await frame.locator('text=Domain Management').first()
        assert await domain_management_section.is_visible()
        status_text = await frame.locator('text=IPPAN DNS Active').text_content()
        assert 'IPPAN DNS Active' in status_text
        # Assert that the domain input field contains the expected domain name using IPPAN naming convention
        selected_domain = await frame.locator('xpath=//div[contains(text(),"SelectedDomain")]/following-sibling::div').text_content()
        assert 'ipnexample.dns' in selected_domain
        # Assert that the quoted price is displayed and includes premium TLD fee
        quoted_price = await frame.locator('xpath=//div[contains(text(),"QuotedPrice")]/following-sibling::div').text_content()
        assert '5 IPN' in quoted_price
        # Assert that the domain appears in 'My Domains' list with correct status and expiry
        my_domains = await frame.locator('xpath=//div[contains(text(),"My Domains")]/following-sibling::div//li').all_text_contents()
        assert any('ipnexample.dns' in domain for domain in my_domains)
        # Assert that the domain status is 'Unverified' initially
        domain_status = await frame.locator('xpath=//div[contains(text(),"Status") and contains(text(),"Unverified")]').first().text_content()
        assert 'Unverified' in domain_status
        # Assert that the site verification section shows the correct domain and token
        verification_domain = await frame.locator('xpath=//div[contains(text(),"Site Verification / Proof of Ownership")]//div[contains(text(),"Domain")]/following-sibling::div').text_content()
        assert 'ipnexample.dns' in verification_domain
        verification_token = await frame.locator('xpath=//div[contains(text(),"Token")]/following-sibling::div').text_content()
        assert verification_token.startswith('638894023368d19b')
        # Assert that the DNS TXT record instruction is displayed correctly
        dns_txt_instruction = await frame.locator('xpath=//div[contains(text(),"DNS TXT RecordInstruction")]/following-sibling::div').text_content()
        assert '_ippan-verify.ipnexample.dns' in dns_txt_instruction
        # Assert that the verification status indicates TXT record not found yet
        verification_status = await frame.locator('xpath=//div[contains(text(),"VerificationStatus")]/following-sibling::div').text_content()
        assert 'TXT record not found yet' in verification_status
        await asyncio.sleep(5)
    
    finally:
        if context:
            await context.close()
        if browser:
            await browser.close()
        if pw:
            await pw.stop()
            
asyncio.run(run_test())
    