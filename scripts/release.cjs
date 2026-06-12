const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..');
const PACKAGE_JSON = path.join(ROOT, 'package.json');
const PACKAGE_LOCK = path.join(ROOT, 'package-lock.json');
const TAURI_CONF = path.join(ROOT, 'src-tauri', 'tauri.conf.json');
const CARGO_TOML = path.join(ROOT, 'src-tauri', 'Cargo.toml');
const CARGO_LOCK = path.join(ROOT, 'src-tauri', 'Cargo.lock');

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\r\n`, 'utf8');
}

function parseVersion(version) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match) throw new Error(`Unsupported semver version: ${version}`);
  return match.slice(1).map(Number);
}

function nextVersion(current, bump) {
  if (/^\d+\.\d+\.\d+$/.test(bump)) return bump;
  const [major, minor, patch] = parseVersion(current);
  if (bump === 'major') return `${major + 1}.0.0`;
  if (bump === 'minor') return `${major}.${minor + 1}.0`;
  if (bump === 'patch') return `${major}.${minor}.${patch + 1}`;
  throw new Error('Usage: npm.cmd run release -- <patch|minor|major|x.y.z> [--no-publish]');
}

function run(command, args) {
  const line = [command, ...args].join(' ');
  console.log(`\n> ${line}`);
  const result = spawnSync(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', line], {
    cwd: ROOT,
    env: process.env,
    stdio: 'inherit'
  });
  if (result.status !== 0) {
    throw new Error(`${line} failed with exit code ${result.status}`);
  }
}

function updateVersions(version) {
  const pkg = readJson(PACKAGE_JSON);
  const lock = readJson(PACKAGE_LOCK);
  const tauri = readJson(TAURI_CONF);
  const cargoToml = fs.readFileSync(CARGO_TOML, 'utf8');
  const cargoLock = fs.readFileSync(CARGO_LOCK, 'utf8');

  pkg.version = version;
  tauri.version = version;
  lock.version = version;
  if (lock.packages && lock.packages['']) {
    lock.packages[''].version = version;
  }

  writeJson(PACKAGE_JSON, pkg);
  writeJson(PACKAGE_LOCK, lock);
  writeJson(TAURI_CONF, tauri);
  fs.writeFileSync(CARGO_TOML, cargoToml.replace(/^version = "\d+\.\d+\.\d+"/m, `version = "${version}"`), 'utf8');
  fs.writeFileSync(
    CARGO_LOCK,
    cargoLock.replace(
      /(name = "openlist-explorer"\r?\nversion = ")\d+\.\d+\.\d+(")/,
      `$1${version}$2`
    ),
    'utf8'
  );
}

function main() {
  const args = process.argv.slice(2);
  const publish = !args.includes('--no-publish');
  const bump = args.find((arg) => !arg.startsWith('--'));
  if (!bump) {
    throw new Error('Usage: npm.cmd run release -- <patch|minor|major|x.y.z> [--no-publish]');
  }

  const current = readJson(PACKAGE_JSON).version;
  const version = nextVersion(current, bump);
  console.log(`Release version: ${current} -> ${version}`);
  updateVersions(version);

  run('npm.cmd', ['run', 'check:encoding']);
  run('npm.cmd', ['run', 'build']);
  run('npm.cmd', ['run', 'check:modals']);
  run('npm.cmd', ['run', 'tauri:build']);
  run('scripts\\p0_release_check.cmd', []);

  if (publish) {
    run('scripts\\publish_github.cmd', []);
  } else {
    console.log('\n[OK] Release build complete. Publishing skipped by --no-publish.');
  }
}

try {
  main();
} catch (error) {
  console.error(`[FAIL] ${error.message}`);
  process.exit(1);
}
