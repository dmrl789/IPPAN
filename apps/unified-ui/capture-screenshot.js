const puppeteer = require('puppeteer');
const path = require('path');

(async () => {
  const port = process.env.PORT || 3010;
  const url = `http://localhost:${port}`;
  const outputPath = path.join(__dirname, 'unified-ui-screenshot.png');

  console.log(`Waiting for server at ${url}...`);
  
  // Wait for server to be ready
  const http = require('http');
  let serverReady = false;
  for (let i = 0; i < 30; i++) {
    try {
      await new Promise((resolve, reject) => {
        const req = http.get(url, (res) => {
          if (res.statusCode === 200) {
            serverReady = true;
            console.log('Server is ready!');
          }
          resolve();
        });
        req.on('error', () => resolve());
        req.setTimeout(1000, () => {
          req.destroy();
          resolve();
        });
      });
      if (serverReady) break;
    } catch (e) {
      // Server not ready yet
    }
    await new Promise(resolve => setTimeout(resolve, 1000));
  }

  if (!serverReady) {
    console.error('Server did not become ready in time');
    process.exit(1);
  }

  console.log('Launching browser...');
  const browser = await puppeteer.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });

  try {
    const page = await browser.newPage();
    
    // Set viewport size
    await page.setViewport({
      width: 1920,
      height: 1080,
      deviceScaleFactor: 2
    });

    console.log(`Navigating to ${url}...`);
    await page.goto(url, {
      waitUntil: 'networkidle2',
      timeout: 30000
    });

    // Wait a bit for any animations to complete
    await new Promise(resolve => setTimeout(resolve, 2000));

    console.log(`Taking screenshot...`);
    await page.screenshot({
      path: outputPath,
      fullPage: true,
      type: 'png'
    });

    console.log(`Screenshot saved to: ${outputPath}`);
  } finally {
    await browser.close();
  }
})();
