import { loadConfig } from './config.js';
import { ControlClient } from './control.js';
import { pollOnce } from './queue.js';

const config = loadConfig();
const client = new ControlClient(config);
let running = true;

for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.once(signal, () => { running = false; });
}

console.log(JSON.stringify({ level: 'info', msg: 'import worker started', worker: config.workerId }));

while (running) {
  try {
    const handled = await pollOnce(client, config);
    if (!handled) await new Promise((resolve) => setTimeout(resolve, config.pollIntervalMs));
  } catch (error) {
    console.error(JSON.stringify({ level: 'error', msg: 'poll iteration failed', err: String(error) }));
    await new Promise((resolve) => setTimeout(resolve, config.pollIntervalMs));
  }
}
