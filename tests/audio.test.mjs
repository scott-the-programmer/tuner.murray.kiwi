import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const source = readFileSync(new URL('../src/audio.js', import.meta.url), 'utf8');
function browser({ sampleRate = 48000, secure = true, media } = {}) {
    const messages = [];
    let stopped = 0, closed = 0, ended, timer, connected = 0;
    const track = { stop: () => stopped++, addEventListener: (_, callback) => ended = callback };
    const stream = { getTracks: () => [track], getAudioTracks: () => [track] };
    class AudioContext {
        sampleRate = sampleRate;
        resume = async () => {};
        close = async () => { closed++; };
        createMediaStreamSource = () => ({ connect: () => connected++ });
        createAnalyser = () => ({
            fftSize: 0,
            getFloatTimeDomainData: buffer => {
                for (let i = 0; i < buffer.length; i++) buffer[i] = 0.2 * Math.sin(i / 20);
            }
        });
    }
    const window = { isSecureContext: secure, AudioContext };
    const context = {
        window,
        navigator: { mediaDevices: { getUserMedia: media || (async () => stream) } },
        dioxus: { send: message => messages.push(message) },
        setTimeout: callback => { timer = callback; return 1; },
        clearTimeout: () => { timer = undefined; }
    };
    return {
        window, messages, stream,
        run: () => runInNewContext(`(async () => { ${source} })()`, context),
        ended: () => ended(),
        resources: () => ({ stopped, closed, connected, timer })
    };
}

test('capture produces finite PCM at common and high sample rates; stop releases resources', async () => {
    for (const sampleRate of [44100, 48000, 96000, 192000]) {
        const b = browser({ sampleRate });
        await b.run();
        assert.equal(b.messages[0].kind, 'ready');
        const frame = b.messages[1];
        assert.equal(frame.samples.length, 2048);
        assert.ok(frame.samples.every(Number.isFinite));
        assert.ok(frame.sample_rate >= 12000 && frame.sample_rate < 15000);
        b.ended();
        b.window.tunerSession.stop();
        assert.equal(b.resources().stopped, 1);
        assert.equal(b.resources().closed, 1);
        assert.equal(b.resources().timer, undefined);
        assert.equal(b.messages.at(-1).kind, 'stopped');
    }
});

test('stop during permission prompt releases microphone when permission later completes', async () => {
    let grant;
    const b = browser({ media: () => new Promise(resolve => { grant = resolve; }) });
    const task = b.run();
    b.window.tunerSession.stop();
    grant(b.stream);
    await task;
    assert.equal(b.resources().stopped, 1);
    assert.equal(b.resources().connected, 0);
    assert.ok(!b.messages.some(message => message.kind === 'ready'));
});

test('permission errors and insecure origins show actionable messages', async () => {
    const denied = browser({ media: async () => { throw { name: 'NotAllowedError' }; } });
    await denied.run();
    assert.match(denied.messages[0].message, /permission was blocked/);
    assert.equal(denied.messages.at(-1).kind, 'stopped');
    const insecure = browser({ secure: false });
    await insecure.run();
    assert.match(insecure.messages[0].message, /HTTPS or localhost/);
});
