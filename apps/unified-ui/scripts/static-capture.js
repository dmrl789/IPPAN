/* eslint-disable */
const http = require('http');
const fs = require('fs');
const path = require('path');
const puppeteer = require('puppeteer');

const appDir = path.resolve(__dirname, '..');
const buildDir = path.join(appDir, '.next');
const serverAppDir = path.join(buildDir, 'server', 'app');

function getContentType(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  switch (ext) {
    case '.html': return 'text/html; charset=utf-8';
    case '.js': return 'application/javascript; charset=utf-8';
    case '.css': return 'text/css; charset=utf-8';
    case '.png': return 'image/png';
    case '.jpg':
    case '.jpeg': return 'image/jpeg';
    case '.svg': return 'image/svg+xml';
    case '.json': return 'application/json; charset=utf-8';
    case '.woff2': return 'font/woff2';
    default: return 'application/octet-stream';
  }
}

function createStaticServer(port) {
  const server = http.createServer((req, res) => {
    try {
      if (req.url === '/' || req.url === '/index.html') {
        const indexPath = path.join(serverAppDir, 'index.html');
        const html = fs.readFileSync(indexPath);
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        res.end(html);
        return;
      }
      if (req.url && req.url.startsWith('/_next/')) {
        // Map '/_next/*' to '.next/*'
        const rel = req.url.replace('/_next/', '');
        const filePath = path.join(buildDir, rel);
        if (fs.existsSync(filePath) && fs.statSync(filePath).isFile()) {
          res.writeHead(200, { 'Content-Type': getContentType(filePath) });
          fs.createReadStream(filePath).pipe(res);
          return;
        }
      }
      res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end('Not Found');
    } catch (err) {
      res.writeHead(500, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end('Internal Server Error');
    }
  });

  return new Promise((resolve) => {
    server.listen(port, () => resolve(server));
  });
}

async function main() {
  const port = 39999;
  const server = await createStaticServer(port);
  let browser;
  try {
    browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox', '--disable-setuid-sandbox'] });
    const page = await browser.newPage();
    await page.setViewport({ width: 1440, height: 900, deviceScaleFactor: 1 });
    await page.goto(`http://localhost:${port}/`, { waitUntil: 'load', timeout: 60000 });
    const outPath = path.resolve('/workspace', 'unified-ui-screenshot.png');
    await page.screenshot({ path: outPath, fullPage: false });
    console.log(`Saved screenshot to: ${outPath}`);
  } finally {
    if (browser) await browser.close();
    server.close();
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
