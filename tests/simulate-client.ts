import WebSocket from 'ws';
import { readFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const samplePath = path.join(__dirname, '..', 'fixtures', 'lecture-sample.json');
const sample = JSON.parse(readFileSync(samplePath, 'utf8'));

const ws = new WebSocket('ws://127.0.0.1:3847');

ws.on('open', () => {
    console.log('[SimClient] Connected to Rust WebSocket server!');
    const startTime = Date.now();

    for (const entry of sample) {
        setTimeout(() => {
            console.log(`[SimClient] Sending entry at time ${entry.time}ms: "${entry.text}"`);
            ws.send(JSON.stringify({
                type: 'subtitle',
                id: `seg-${entry.time}`,
                text: entry.text,
                timestampMs: Date.now()
            }));
        }, entry.time);
    }

    const lastTime = sample[sample.length - 1].time;
    setTimeout(() => {
        console.log('[SimClient] All sample entries sent. Closing client.');
        ws.close();
        process.exit(0);
    }, lastTime + 2000);
});

ws.on('error', (err) => {
    console.error('[SimClient] WebSocket error:', err);
});
