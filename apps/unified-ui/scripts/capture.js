/* eslint-disable */
const { spawn } = require('child_process');
const puppeteer = require('puppeteer');
const path = require('path');
const fs = require('fs');
const axios = require('axios');

async function main() {
  // Ensure env vars to avoid external calls during render
  process.env.NEXT_PUBLIC_GATEWAY_URL = process.env.NEXT_PUBLIC_GATEWAY_URL || 'http://localhost:8081/api';
  process.env.NEXT_PUBLIC_API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL || 'http://localhost:7080';
  process.env.NEXT_PUBLIC_WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:0/ws';
  process.env.NEXT_PUBLIC_AI_ENABLED = process.env.NEXT_PUBLIC_AI_ENABLED || '1';
  process.env.NEXT_PUBLIC_ENABLE_FULL_UI = process.env.NEXT_PUBLIC_ENABLE_FULL_UI || '1';

  const appDir = path.resolve(__dirname, '..');
  const port = Number(process.env.PORT) || 3055;

  fs.writeFileSync('/workspace/capture-log.txt', 'Starting next start...\n');
  const child = spawn(
    process.execPath,
    [require.resolve('next/dist/bin/next'), 'start', '-p', String(port)],
    {
      cwd: appDir,
      env: { ...process.env, PORT: String(port) },
      stdio: ['ignore', 'pipe', 'pipe']
    }
  );

  child.stdout.on('data', (d) => {
    fs.appendFileSync('/workspace/capture-log.txt', `next: ${d.toString()}`);
  });
  child.stderr.on('data', (d) => {
    fs.appendFileSync('/workspace/capture-log.txt', `next-err: ${d.toString()}`);
  });

  // Wait for server to be ready
  const baseUrl = `http://localhost:${port}`;
  const start = Date.now();
  let ready = false;
  while (!ready && Date.now() - start < 120000) {
    try {
      await axios.get(baseUrl, { timeout: 2000 });
      ready = true;
      break;
    } catch (_) {
      await new Promise((r) => setTimeout(r, 500));
    }
  }
  if (!ready) {
    child.kill('SIGTERM');
    throw new Error('Next server did not become ready in time');
  }
  fs.appendFileSync('/workspace/capture-log.txt', `Server ready at ${baseUrl}\n`);

  let browser;
  try {
    browser = await puppeteer.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-setuid-sandbox']
    });
    const page = await browser.newPage();
    await page.setViewport({ width: 1440, height: 900, deviceScaleFactor: 1 });
    await page.goto(baseUrl, { waitUntil: 'load', timeout: 60000 });
    fs.appendFileSync('/workspace/capture-log.txt', `Navigated to ${baseUrl}\n`);

    const outPath = path.resolve('/workspace', 'unified-ui-screenshot.png');
    await page.screenshot({ path: outPath, fullPage: false });
    fs.appendFileSync('/workspace/capture-log.txt', `Screenshot saved to ${outPath}\n`);
  } finally {
    if (browser) await browser.close();
    child.kill('SIGTERM');
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
