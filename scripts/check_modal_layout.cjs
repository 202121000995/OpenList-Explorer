const http = require('http');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const DIST = path.join(ROOT, 'dist');
const OUTPUT = path.join(ROOT, 'src-tauri', 'target', 'modal-check');
const WIDTH = 1280;
const HEIGHT = 800;

const MIME = {
  '.css': 'text/css',
  '.html': 'text/html',
  '.ico': 'image/x-icon',
  '.js': 'text/javascript',
  '.json': 'application/json',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2'
};

function findBrowser() {
  const candidates = [
    process.env.BROWSER_EXE,
    'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe'
  ];
  const found = candidates.find((candidate) => candidate && fs.existsSync(candidate));
  if (!found) throw new Error('Chrome/Edge was not found. Set BROWSER_EXE to enable modal screenshots.');
  return found;
}

function startServer() {
  const server = http.createServer((req, res) => {
    const pathname = decodeURIComponent(new URL(req.url, 'http://127.0.0.1').pathname);
    let file = path.join(DIST, pathname === '/' ? 'index.html' : pathname);
    if (!file.startsWith(DIST) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
      file = path.join(DIST, 'index.html');
    }
    res.setHeader('content-type', MIME[path.extname(file)] || 'application/octet-stream');
    fs.createReadStream(file).pipe(res);
  });

  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => resolve(server));
  });
}

function sampleTasks() {
  const now = Date.now();
  return [
    {
      id: 'u1',
      type: 'upload',
      source: 'local',
      name: 'very-long-upload-file-name-that-should-ellipsis-but-not-push-buttons-2026.iso',
      path: '/remote/uploads/very-long-upload-file-name-that-should-ellipsis-but-not-push-buttons-2026.iso',
      status: 'running',
      progress: 37,
      speed: 3145728,
      stage: 'uploading',
      message: '上传中',
      localPath: 'C:/Users/wangn/Videos/test.iso',
      createdAt: now,
      updatedAt: now
    },
    {
      id: 'u2',
      type: 'upload',
      source: 'local',
      name: 'photo_backup.zip',
      path: '/remote/uploads/photo_backup.zip',
      status: 'waiting',
      progress: 4,
      speed: 0,
      stage: 'queued',
      message: '等待中',
      localPath: 'C:/Users/wangn/Desktop/photo_backup.zip',
      createdAt: now,
      updatedAt: now
    },
    {
      id: 'd1',
      type: 'download',
      source: 'local',
      name: 'download-with-super-long-name-and-symbols-openlist-ui-layout-check.tar.gz',
      path: '/downloads/download-with-super-long-name-and-symbols-openlist-ui-layout-check.tar.gz',
      status: 'running',
      progress: 68,
      speed: 9437184,
      stage: 'downloading',
      message: '下载中',
      localPath: 'C:/Users/wangn/Downloads/layout-check.tar.gz',
      createdAt: now,
      updatedAt: now
    },
    {
      id: 'd2',
      type: 'download',
      source: 'openlist-offline',
      name: 'cloud-offline-task-long-url-example.mkv',
      path: '/remote/cloud-offline-task-long-url-example.mkv',
      status: 'failed',
      progress: 13,
      speed: 0,
      stage: 'failed',
      rawStatus: 'failed',
      message: '网络错误',
      failureReason: '网络错误',
      remoteId: 'remote-1',
      createdAt: now,
      updatedAt: now
    }
  ];
}

async function setupPage(browser, baseUrl) {
  const page = await browser.newPage({ viewport: { width: WIDTH, height: HEIGHT }, deviceScaleFactor: 1 });
  await page.addInitScript((tasks) => {
    localStorage.setItem('openlist.onboardingDone', '1');
    localStorage.setItem('openlist.theme', 'dark');
    localStorage.setItem('openlist.tasks', JSON.stringify(tasks));
  }, sampleTasks());
  await page.goto(`${baseUrl}/files`, { waitUntil: 'networkidle' });
  return page;
}

async function screenshotSettings(browser, baseUrl) {
  const page = await setupPage(browser, baseUrl);
  await page.locator('.window-actions .chrome-button').first().click();
  const modal = page.locator('.settings-shell-modal').last();
  await modal.waitFor({ state: 'visible', timeout: 5000 });
  await page.waitForTimeout(250);
  const target = path.join(OUTPUT, 'settings-clicked.png');
  await page.screenshot({ path: target, fullPage: true });
  await page.close();
  return target;
}

async function screenshotTaskModal(browser, baseUrl, navIndex, filename) {
  const page = await setupPage(browser, baseUrl);
  await page.locator('.titlebar-nav-item').nth(navIndex).click();
  const modal = page.locator('.task-shell-modal').filter({ has: page.locator('.task-card') }).last();
  await modal.waitFor({ state: 'visible', timeout: 5000 });
  await modal.locator('.task-card').first().waitFor({ state: 'visible', timeout: 5000 });
  await page.waitForTimeout(250);
  const target = path.join(OUTPUT, filename);
  await page.screenshot({ path: target, fullPage: true });

  const metrics = await modal.evaluate((root) => Array.from(root.querySelectorAll('.task-card')).map((card) => {
    const rect = (selector) => {
      const element = selector ? card.querySelector(selector) : card;
      const box = element.getBoundingClientRect();
      return {
        x: Math.round(box.x),
        width: Math.round(box.width),
        right: Math.round(box.right)
      };
    };
    const cardBox = rect();
    const progressBox = rect('.task-progress');
    const actionsBox = rect('.task-card-actions');
    const stateBox = rect('.task-card-state');
    return {
      card: cardBox,
      progress: progressBox,
      state: stateBox,
      actions: actionsBox,
      overflow: Math.round(actionsBox.right - cardBox.right),
      rightGap: Math.round(cardBox.right - actionsBox.right),
      progressToStateGap: Math.round(stateBox.x - progressBox.right)
    };
  }));

  const bad = metrics.find((item) => item.overflow > 0 || item.rightGap > 24 || item.progress.width < 240);
  if (bad) {
    throw new Error(`${filename} layout check failed: ${JSON.stringify(bad)}`);
  }

  await page.close();
  return { target, metrics };
}

async function main() {
  if (!fs.existsSync(DIST)) {
    throw new Error('dist does not exist. Run npm.cmd run build first.');
  }
  fs.mkdirSync(OUTPUT, { recursive: true });

  const { chromium } = require('playwright');
  const server = await startServer();
  const baseUrl = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch({ headless: true, executablePath: findBrowser() });

  try {
    const settings = await screenshotSettings(browser, baseUrl);
    const uploads = await screenshotTaskModal(browser, baseUrl, 1, 'uploads-clicked.png');
    const downloads = await screenshotTaskModal(browser, baseUrl, 2, 'downloads-clicked.png');
    console.log(`[PASS] settings -> ${settings}`);
    console.log(`[PASS] uploads -> ${uploads.target}`);
    console.log(`[PASS] downloads -> ${downloads.target}`);
  } finally {
    await browser.close();
    server.close();
  }
}

main().catch((error) => {
  console.error(`[FAIL] ${error.message}`);
  process.exit(1);
});
