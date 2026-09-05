import test from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('..', import.meta.url));
const files = ['scripts/env.mjs', 'scripts/rename.mjs', 'apps/pwa/public/manifest.webmanifest', 'apps/pwa/src/app/config.ts', 'apps/pwa/index.html'];

test('a separate copy can preview and apply branding without touching the source', () => {
  const copy = mkdtempSync(join(tmpdir(), 'starter-rename-'));
  const original = files.map(path => readFileSync(join(root, path), 'utf8'));
  try {
    for (const path of files) {
      mkdirSync(dirname(join(copy, path)), { recursive: true });
      cpSync(join(root, path), join(copy, path));
    }
    const run = (...args) => spawnSync(process.execPath, [join(copy, 'scripts/rename.mjs'), ...args], { cwd: copy, encoding: 'utf8', timeout: 10000 });
    assert.equal(run('Another App', 'another-app').status, 0);
    assert.deepEqual(files.map(path => readFileSync(join(copy, path), 'utf8')), original);
    assert.equal(run('Another & App', 'another-app', '--apply').status, 0);
    const manifest = JSON.parse(readFileSync(join(copy, 'apps/pwa/public/manifest.webmanifest'), 'utf8'));
    assert.equal(manifest.name, 'Another & App');
    assert.equal(manifest.id, '/another-app');
    assert.match(readFileSync(join(copy, 'apps/pwa/index.html'), 'utf8'), /<title>Another &amp; App<\/title>/);
    assert.match(readFileSync(join(copy, 'apps/pwa/src/app/config.ts'), 'utf8'), /another-app/);
    assert.notEqual(run('<script>', 'another-app', '--apply').status, 0);
    assert.notEqual(run('Another App', '../outside', '--apply').status, 0);
    assert.deepEqual(files.map(path => readFileSync(join(root, path), 'utf8')), original);
  } finally {
    rmSync(copy, { recursive: true, force: true });
  }
});

test('install icons have their declared dimensions and packages do not require BSLT', () => {
  for (const size of [192, 512]) {
    const image = readFileSync(join(root, `apps/pwa/public/icon-${size}.png`));
    assert.equal(image.subarray(1, 4).toString(), 'PNG');
    assert.equal(image.readUInt32BE(16), size);
    assert.equal(image.readUInt32BE(20), size);
  }
  for (const path of ['package.json', 'apps/pwa/package.json']) {
    const manifest = JSON.parse(readFileSync(join(root, path), 'utf8'));
    const dependencies = Object.keys({ ...manifest.dependencies, ...manifest.devDependencies });
    assert.ok(dependencies.every(name => !name.startsWith('@bslt/')));
    assert.ok(!dependencies.includes('fastify'));
  }
});
