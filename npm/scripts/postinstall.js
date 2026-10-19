#!/usr/bin/env node
'use strict';

const { FALLBACK_HINT, download } = require('../lib/asset');

download().catch((error) => {
  process.stderr.write(`living-docs: could not install the binary.\n${error.message}\n${FALLBACK_HINT}\n`);
  process.exit(1);
});
