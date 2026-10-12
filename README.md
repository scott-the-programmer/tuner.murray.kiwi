# tuner.murray.kiwi

An ad-free guitar and ukulele tuner built with Rust and Dioxus. Microphone audio stays in the browser. No server, accounts, analytics, or external assets.

## Run

Install Rust with the WebAssembly target and Trunk:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve
```

Open http://localhost:8088, click **Start tuning**, and allow microphone access. Pluck one string at a time. The meter turns green within ±5 cents. Tighten a flat string and loosen a sharp string.

Auto detect chooses the nearest string in your tuning. Click a string to lock the target, useful when a string is far out of tune. Click **Auto detect** to unlock it. **Play** produces a short reference tone and pauses pitch analysis during playback.

Guitar presets: standard, Drop D, half step down, full step down, Open C, Open D, Open G. Ukulele presets: standard high G, low G, baritone. A4 reference is adjustable from 432 to 446 Hz.

New guitar tunings, from string 6 to string 1:

| Tuning | Notes |
| --- | --- |
| Open C | C2 G2 C3 G3 C4 E4 |
| Full step down | D2 G2 C3 F3 A3 D4 |
| Open D | D2 A2 D3 F♯3 A3 D4 |

## Build and check

```sh
cargo test
cargo clippy --all-targets -- -D warnings
node --test tests/audio.test.mjs
trunk build --release
```

Deploy `dist/` to any static host using HTTPS. Microphone access requires HTTPS or localhost. The deployed app has no backend.

## Raspberry Pi deployment

Pushes to `main` run checks, build the release WebAssembly bundle, and publish the nginx image to GHCR for AMD64, ARM64, and ARMv7. The GHCR package must be public so the Pi and Watchtower can pull it without registry credentials.

On the Pi, `/home/scott/tuner/docker-compose.yml` runs the tuner on the existing `smkiwi_default` network. The smkiwi Cloudflare Tunnel routes `tuner.murray.kiwi` to `http://tuner:80`. The existing Watchtower checks for image updates every five minutes. Container restart policy is `unless-stopped`.

```sh
cd /home/scott/tuner
docker compose pull
docker compose up -d
curl --fail http://127.0.0.1:8081/healthz
```

To roll back, replace the Compose image tag with `sha-<previous commit>` and run `docker compose up -d`. Disable the Watchtower label while keeping a rollback version pinned.

## Audio

Web Audio captures PCM with echo cancellation, noise suppression, and automatic gain control disabled. A small JavaScript bridge passes downsampled frames to Rust. Rust uses YIN pitch detection, rejects silence/noise, interpolates the period, and smooths nearby readings. Detection covers 55–1100 Hz.

Reference tones use a browser oscillator. No audio is recorded, stored, or transmitted. Browser microphone permission is requested only after pressing Start tuning. Stop listening releases the microphone, including when permission completes after stopping.
