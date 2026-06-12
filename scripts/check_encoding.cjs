const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const INCLUDE_EXTENSIONS = new Set([
  '.cmd',
  '.cjs',
  '.css',
  '.html',
  '.js',
  '.json',
  '.md',
  '.ps1',
  '.py',
  '.rs',
  '.toml',
  '.ts',
  '.tsx',
  '.vue'
]);
const EXCLUDED_DIRS = new Set([
  '.git',
  '.agents',
  '.codex',
  'dist',
  'node_modules',
  'target',
  '__pycache__'
]);
const MOJIBAKE_PATTERNS = [
  '鑴氭湰',
  '鍙戝竷',
  '涓婁紶',
  '涓嬭浇',
  '鐣岄潰',
  '璁剧疆',
  '鏆傛棤',
  '鎴愬姛',
  '澶辫触',
  '绛夊緟',
  '杩涜',
  '鍒犻櫎',
  '銆',
  '鈥'
];

function walk(dir, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (EXCLUDED_DIRS.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(full, files);
    } else if (INCLUDE_EXTENSIONS.has(path.extname(entry.name).toLowerCase())) {
      files.push(full);
    }
  }
  return files;
}

function hasUtf8Bom(buffer) {
  return buffer.length >= 3 && buffer[0] === 0xef && buffer[1] === 0xbb && buffer[2] === 0xbf;
}

let failures = 0;
for (const file of walk(ROOT)) {
  if (path.basename(file) === 'check_encoding.cjs') continue;

  const buffer = fs.readFileSync(file);
  const text = buffer.toString('utf8');
  const rel = path.relative(ROOT, file);

  if (text.includes('\uFFFD')) {
    console.error(`[FAIL] ${rel}: invalid UTF-8 replacement character found`);
    failures += 1;
  }
  if (hasUtf8Bom(buffer)) {
    console.error(`[FAIL] ${rel}: UTF-8 BOM found`);
    failures += 1;
  }

  const pattern = MOJIBAKE_PATTERNS.find((item) => text.includes(item));
  if (pattern) {
    console.error(`[FAIL] ${rel}: possible mojibake text found: ${pattern}`);
    failures += 1;
  }
}

if (failures) {
  console.error(`[FAIL] Encoding check failed with ${failures} issue(s).`);
  process.exit(1);
}

console.log('[PASS] UTF-8 encoding and mojibake checks passed.');
