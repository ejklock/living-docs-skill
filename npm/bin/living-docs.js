#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const { FALLBACK_HINT, ensureBinary } = require('../lib/asset');

async function main() {
  const binary = await ensureBinary();
  const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.signal) process.kill(process.pid, result.signal);
  process.exit(result.status === null ? 1 : result.status);
}

main().catch((error) => {
  process.stderr.write(`living-docs: ${error.message}\n${FALLBACK_HINT}\n`);
  process.exit(1);
});
