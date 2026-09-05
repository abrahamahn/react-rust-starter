import { spawn } from 'node:child_process';
import { root, loadEnvironment } from './env.mjs';
loadEnvironment();
const command = process.argv[2];
if (!['serve','migrate','check','prune'].includes(command)) throw new Error('Use serve, migrate, check or prune');
const child = spawn('cargo', ['run','--locked','-p','starter-server','--',command], { cwd: root, stdio: 'inherit', env: process.env });
child.on('error', () => { console.error('Could not start Cargo. Install the pinned Rust toolchain.'); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
for (const signal of ['SIGINT','SIGTERM']) process.on(signal, () => child.kill(signal));
