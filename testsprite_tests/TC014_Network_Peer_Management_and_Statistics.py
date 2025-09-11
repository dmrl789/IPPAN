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
        # Navigate back to the IPPAN unified-ui app at http://localhost:3000 to retry loading the app UI.
        await page.goto('http://localhost:3000', timeout=10000)
        

        # Directly query the network peers API endpoint to verify it returns connected peers with peer_id, address, and status.
        await page.goto('http://localhost:3000/api/network/peers', timeout=10000)
        

        # Use browser developer tools or a script to fetch and inspect the raw JSON response from the /api/network/peers endpoint to verify it lists connected peers with peer_id, address, and status.
        await page.goto('http://localhost:3000/api/network/peers', timeout=10000)
        

        # Query the network statistics API endpoint to check if it returns total_peers, active_connections, bytes_sent, and bytes_received with valid numeric values.
        await page.goto('http://localhost:3000/api/network/stats', timeout=10000)
        

        # Click on 'Network Map' navigation link to check if it shows connected peers or network statistics in the UI.
        frame = context.pages[-1]
        elem = frame.locator('xpath=html/body/div/div/div/aside/nav/div[2]/a[7]').nth(0)
        await page.wait_for_timeout(3000); await elem.click(timeout=5000)
        

        # Verify if bytes_sent and bytes_received data are available elsewhere or if they are missing from the UI and API. Then complete the task.
        await page.mouse.wheel(0, window.innerHeight)
        

        # Assert the network peers API response contains peer_id, address, and status for each peer
        peers_response = await page.evaluate('''() => fetch('/api/network/peers').then(res => res.json())''')
        assert isinstance(peers_response, list), 'Peers response should be a list'
        for peer in peers_response:
            assert 'peer_id' in peer, 'Each peer should have a peer_id'
            assert 'address' in peer, 'Each peer should have an address'
            assert 'status' in peer, 'Each peer should have a status'
            assert isinstance(peer['peer_id'], str) and peer['peer_id'], 'peer_id should be a non-empty string'
            assert isinstance(peer['address'], str) and peer['address'], 'address should be a non-empty string'
            assert peer['status'] in ['online', 'offline', 'syncing'], 'status should be one of online, offline, syncing'
          
        # Assert the network statistics API response contains total_peers, active_connections, bytes_sent, and bytes_received with valid numeric values
        stats_response = await page.evaluate('''() => fetch('/api/network/stats').then(res => res.json())''')
        assert 'total_peers' in stats_response and isinstance(stats_response['total_peers'], int) and stats_response['total_peers'] >= 0, 'total_peers should be a non-negative integer'
        assert 'active_connections' in stats_response and isinstance(stats_response['active_connections'], int) and stats_response['active_connections'] >= 0, 'active_connections should be a non-negative integer'
        assert 'bytes_sent' in stats_response and isinstance(stats_response['bytes_sent'], int) and stats_response['bytes_sent'] >= 0, 'bytes_sent should be a non-negative integer'
        assert 'bytes_received' in stats_response and isinstance(stats_response['bytes_received'], int) and stats_response['bytes_received'] >= 0, 'bytes_received should be a non-negative integer'
        await asyncio.sleep(5)
    
    finally:
        if context:
            await context.close()
        if browser:
            await browser.close()
        if pw:
            await pw.stop()
            
asyncio.run(run_test())
    