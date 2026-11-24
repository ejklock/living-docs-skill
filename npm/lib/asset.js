'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const PACKAGE_ROOT = path.resolve(__dirname, '..');
const VENDOR_DIR = path.join(PACKAGE_ROOT, 'vendor');
const BINARY_PATH = path.join(VENDOR_DIR, 'living-docs');
const DEFAULT_DOWNLOAD_BASE = 'https://github.com/ejklock/living-docs-skill/releases/download/';

const TARGETS = {
  'darwin-arm64': 'aarch64-apple-darwin',
  'darwin-x64': 'x86_64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
};

const FALLBACK_HINT =
  'Install the binary another way instead: curl -fsSL https://raw.githubusercontent.com/ejklock/living-docs-skill/main/install.sh | bash (or run ./install.sh from a checkout).';

/**
 * The "<platform>-<arch>" key of the running host; LIVING_DOCS_PLATFORM overrides it.
 * @returns {string}
 */
function platformKey() {
  return process.env.LIVING_DOCS_PLATFORM || `${process.platform}-${process.arch}`;
}

/**
 * The release target triple for the running host.
 * @returns {string} the target triple
 * @throws {Error} naming the four supported targets when the host is not one of them
 */
function resolveTarget() {
  const key = platformKey();
  const target = TARGETS[key];
  if (target) return target;
  throw new Error(
    `Unsupported platform ${key}. Supported targets: ${Object.values(TARGETS).join(', ')}. ${FALLBACK_HINT}`,
  );
}

/** @returns {string} the package version, which is the release tag without its "v" */
function packageVersion() {
  return JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT, 'package.json'), 'utf8')).version;
}

/**
 * The URL directory holding this version's release assets.
 * LIVING_DOCS_DOWNLOAD_BASE replaces the GitHub releases location.
 * @returns {string} ends with a slash
 */
function releaseUrl() {
  const base = process.env.LIVING_DOCS_DOWNLOAD_BASE || DEFAULT_DOWNLOAD_BASE;
  return `${base.replace(/\/*$/, '/')}v${packageVersion()}/`;
}

async function fetchBytes(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`GET ${url} failed with HTTP ${response.status}`);
  return Buffer.from(await response.arrayBuffer());
}

/**
 * Throws unless the bytes hash to the digest in a `sha256sum`-style line ("<hex>  <name>").
 * @param {Buffer} bytes
 * @param {string} checksumText
 * @param {string} assetName
 */
function verifySha256(bytes, checksumText, assetName) {
  const expected = checksumText.trim().split(/\s+/)[0].toLowerCase();
  const actual = crypto.createHash('sha256').update(bytes).digest('hex');
  if (expected !== actual) {
    throw new Error(`Checksum mismatch for ${assetName}: expected ${expected}, got ${actual}.`);
  }
}

/**
 * The per-version binary path under the user's cache directory.
 * XDG_CACHE_HOME is used only when absolute, as the XDG Base Directory spec requires.
 * @returns {string}
 * @throws {Error} when there is no absolute XDG_CACHE_HOME and no HOME
 */
function cacheBinaryPath() {
  const home = process.env.HOME;
  const xdg = process.env.XDG_CACHE_HOME;
  const root = xdg && path.isAbsolute(xdg) ? xdg : home ? path.join(home, '.cache') : '';
  if (!root) throw new Error('Cannot locate a cache directory: no absolute XDG_CACHE_HOME and no HOME.');
  return path.join(root, 'living-docs', packageVersion(), 'living-docs');
}

/** @returns {boolean} true when the package's vendor directory exists or can be created, and is writable */
function vendorIsWritable() {
  try {
    fs.mkdirSync(VENDOR_DIR, { recursive: true });
    fs.accessSync(VENDOR_DIR, fs.constants.W_OK);
    return true;
  } catch {
    return false;
  }
}

/** Where a fresh download goes: the package's vendor directory when writable, else the user cache. */
function installPath() {
  return vendorIsWritable() ? BINARY_PATH : cacheBinaryPath();
}

function writeExecutableAtomically(bytes, target) {
  const dir = path.dirname(target);
  fs.mkdirSync(dir, { recursive: true });
  const partial = path.join(dir, `.living-docs.${process.pid}.partial`);
  try {
    fs.writeFileSync(partial, bytes, { mode: 0o755 });
    fs.chmodSync(partial, 0o755);
    fs.renameSync(partial, target);
  } finally {
    fs.rmSync(partial, { force: true });
  }
}

/**
 * Downloads the host's release asset, verifies its sha256 and moves it into place.
 * Nothing is left on disk when any step fails.
 * @returns {Promise<string>} the binary path
 */
async function download() {
  const assetName = `living-docs-${resolveTarget()}`;
  const url = `${releaseUrl()}${assetName}`;
  const target = installPath();
  const [bytes, checksum] = await Promise.all([
    fetchBytes(url),
    fetchBytes(`${url}.sha256`).then((buffer) => buffer.toString('utf8')),
  ]);
  verifySha256(bytes, checksum, assetName);
  writeExecutableAtomically(bytes, target);
  return target;
}

/**
 * The binary path: the package's vendor binary, else this version's cached binary, else a fresh download.
 * @returns {Promise<string>}
 */
async function ensureBinary() {
  if (fs.existsSync(BINARY_PATH)) return BINARY_PATH;
  const cached = cacheBinaryPathOrNone();
  if (cached && fs.existsSync(cached)) return cached;
  return download();
}

function cacheBinaryPathOrNone() {
  try {
    return cacheBinaryPath();
  } catch {
    return undefined;
  }
}

module.exports = { FALLBACK_HINT, TARGETS, BINARY_PATH, resolveTarget, releaseUrl, download, ensureBinary };
