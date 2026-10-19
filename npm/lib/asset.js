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

function writeExecutableAtomically(bytes) {
  fs.mkdirSync(VENDOR_DIR, { recursive: true });
  const partial = path.join(VENDOR_DIR, `.living-docs.${process.pid}.partial`);
  try {
    fs.writeFileSync(partial, bytes, { mode: 0o755 });
    fs.chmodSync(partial, 0o755);
    fs.renameSync(partial, BINARY_PATH);
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
  const [bytes, checksum] = await Promise.all([
    fetchBytes(url),
    fetchBytes(`${url}.sha256`).then((buffer) => buffer.toString('utf8')),
  ]);
  verifySha256(bytes, checksum, assetName);
  writeExecutableAtomically(bytes);
  return BINARY_PATH;
}

/**
 * The binary path, downloading it first when it is missing.
 * @returns {Promise<string>}
 */
async function ensureBinary() {
  if (fs.existsSync(BINARY_PATH)) return BINARY_PATH;
  return download();
}

module.exports = { FALLBACK_HINT, TARGETS, BINARY_PATH, resolveTarget, releaseUrl, download, ensureBinary };
