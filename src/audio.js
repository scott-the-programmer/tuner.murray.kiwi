// Web Audio handles capture only. Rust processes each PCM frame.
window.tunerSession?.stop();
const session = { stopped: false, stream: null, context: null, timer: null };
session.stop = () => {
    if (session.stopped) return;
    session.stopped = true;
    clearTimeout(session.timer);
    session.stream?.getTracks().forEach(track => track.stop());
    session.context?.close().catch(() => {});
    dioxus.send({ kind: "stopped" });
};
window.tunerSession = session;
try {
    if (!window.isSecureContext || !navigator.mediaDevices?.getUserMedia) {
        throw new Error("Microphone access needs HTTPS or localhost. Open this tuner in a supported browser.");
    }
    session.stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false }, video: false
    });
    if (session.stopped) {
        session.stream.getTracks().forEach(track => track.stop());
        return;
    }
    const AudioContext = window.AudioContext || window.webkitAudioContext;
    session.context = new AudioContext();
    await session.context.resume();
    const source = session.context.createMediaStreamSource(session.stream);
    const analyser = session.context.createAnalyser();
    const stride = Math.max(1, Math.floor(session.context.sampleRate / 12000));
    const size = 2048;
    analyser.fftSize = Math.min(32768, 2 ** Math.ceil(Math.log2(size * stride)));
    source.connect(analyser);
    const buffer = new Float32Array(analyser.fftSize);
    dioxus.send({ kind: "ready" });
    const capture = () => {
        if (session.stopped) return;
        analyser.getFloatTimeDomainData(buffer);
        const samples = new Array(size);
        const start = buffer.length - size * stride;
        for (let i = 0; i < size; i++) {
            let total = 0;
            for (let j = 0; j < stride; j++) total += buffer[start + i * stride + j];
            samples[i] = total / stride;
        }
        dioxus.send({ kind: "samples", samples, sample_rate: session.context.sampleRate / stride });
        session.timer = setTimeout(capture, 90);
    };
    session.stream.getAudioTracks()[0].addEventListener("ended", session.stop);
    capture();
} catch (error) {
    if (!session.stopped) {
        const messages = {
            NotAllowedError: "Microphone permission was blocked. Allow microphone access in your browser, then try again.",
            NotFoundError: "No microphone found. Connect a microphone, then try again.",
            NotReadableError: "Microphone is busy. Close other apps using it, then try again."
        };
        dioxus.send({ kind: "error", message: messages[error.name] || error.message || "Could not start microphone. Try again." });
        session.stop();
    }
}
