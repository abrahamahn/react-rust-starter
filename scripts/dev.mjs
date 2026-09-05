import { spawn } from 'node:child_process';
import { root, loadEnvironment } from './env.mjs';
loadEnvironment();
const children = [];
let stopping = false;
function stop(code = 0) {
  if (stopping) return;
  stopping = true; process.exitCode = code;
  for (const child of children) {
    if (child.exitCode !== null || !child.pid) continue;
    try { if (process.platform === 'win32') child.kill('SIGTERM'); else process.kill(-child.pid, 'SIGTERM'); } catch { /* Already exited. */ }
  }
}
function start(command, args) {
  const child = spawn(command, args, { cwd: root, env: process.env, stdio: 'inherit', detached: process.platform !== 'win32' });
  children.push(child);
  child.on('error', () => { console.error(`Unable to start ${command}. Check the prerequisites.`); stop(1); });
  child.on('exit', code => { if (!stopping) stop(code ?? 1); });
}
start('cargo', ['run','--locked','-p','starter-server','--','serve']);
start(process.execPath, [process.env.npm_execpath, '--filter','@starter/pwa','dev']);
process.on('SIGINT', () => stop()); process.on('SIGTERM', () => stop());
