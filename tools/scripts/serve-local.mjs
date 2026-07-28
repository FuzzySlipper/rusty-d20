import { createServer } from 'node:net';
import { spawn } from 'node:child_process';

const host = '0.0.0.0';
const probeHost = '127.0.0.1';
const port = await freePort();
const publicUrl = `http://${probeHost}:${port}`;
await run('pnpm', ['run', 'build']);
const child = spawn('cargo', ['run', '-p', 'rusty-d20', '--bin', 'rusty-d20-host', '--', '--address', `${host}:${port}`], {
  stdio: 'inherit',
  shell: false,
});

console.log(`BASE_URL=${publicUrl}`);

process.on('SIGINT', () => {
  child.kill('SIGINT');
});

process.on('SIGTERM', () => {
  child.kill('SIGTERM');
});

child.on('exit', (code) => {
  process.exit(code ?? 0);
});

function freePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, probeHost, () => {
      const address = server.address();
      server.close(() => {
        if (address !== null && typeof address === 'object') {
          resolve(address.port);
        } else {
          reject(new Error('Could not allocate a local port'));
        }
      });
    });
    server.on('error', reject);
  });
}

function run(command, arguments_) {
  return new Promise((resolve, reject) => {
    const process = spawn(command, arguments_, { stdio: 'inherit', shell: false });
    process.on('error', reject);
    process.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} exited with code ${code ?? 'unknown'}`));
      }
    });
  });
}
