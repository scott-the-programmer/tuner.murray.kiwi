mod pitch;

use dioxus::prelude::*;
use serde::Deserialize;

const CSS: &str = include_str!("../assets/style.css");
const AUDIO: &str = include_str!("audio.js");
const NAMES: [&str; 12] = [
    "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
];

#[derive(Clone, Copy, PartialEq)]
struct Tuning {
    name: &'static str,
    strings: &'static [u8],
}
const GUITAR: [Tuning; 7] = [
    Tuning {
        name: "Standard",
        strings: &[40, 45, 50, 55, 59, 64],
    },
    Tuning {
        name: "Drop D",
        strings: &[38, 45, 50, 55, 59, 64],
    },
    Tuning {
        name: "Half step down",
        strings: &[39, 44, 49, 54, 58, 63],
    },
    Tuning {
        name: "Full step down",
        strings: &[38, 43, 48, 53, 57, 62],
    },
    Tuning {
        name: "Open C",
        strings: &[36, 43, 48, 55, 60, 64],
    },
    Tuning {
        name: "Open D",
        strings: &[38, 45, 50, 54, 57, 62],
    },
    Tuning {
        name: "Open G",
        strings: &[38, 43, 50, 55, 59, 62],
    },
];
const UKULELE: [Tuning; 3] = [
    Tuning {
        name: "Standard · high G",
        strings: &[67, 60, 64, 69],
    },
    Tuning {
        name: "Low G",
        strings: &[55, 60, 64, 69],
    },
    Tuning {
        name: "Baritone",
        strings: &[50, 55, 59, 64],
    },
];

#[derive(Deserialize)]
struct AudioMessage {
    kind: String,
    #[serde(default)]
    samples: Vec<f32>,
    #[serde(default)]
    sample_rate: f64,
    #[serde(default)]
    message: String,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut ukulele = use_signal(|| false);
    let mut tuning_index = use_signal(|| 0usize);
    let mut selected = use_signal(|| None::<usize>);
    let mut automatic_index = use_signal(|| 0usize);
    let mut reference = use_signal(|| 440.0);
    let mut listening = use_signal(|| false);
    let mut ready = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut detected = use_signal(|| None::<pitch::Pitch>);
    let mut generation = use_signal(|| 0u32);
    let mut tone_playing = use_signal(|| false);
    let tunings: &[Tuning] = if ukulele() { &UKULELE } else { &GUITAR };
    let tuning = tunings[tuning_index()];
    let index = selected()
        .unwrap_or(automatic_index())
        .min(tuning.strings.len() - 1);
    let midi = tuning.strings[index];
    let note = NAMES[midi as usize % 12];
    let octave = midi / 12 - 1;
    let target = pitch::frequency(midi, reference());
    let current = detected();
    let deviation = current.map(|p| pitch::cents(p.frequency, target));
    let in_tune = deviation.is_some_and(|c| c.abs() <= 5.0);
    let meter_position = deviation.unwrap_or(0.0).clamp(-50.0, 50.0) + 50.0;
    let status = if tone_playing() {
        "Reference tone playing"
    } else if !listening() {
        "Microphone off"
    } else if !ready() {
        "Waiting for microphone…"
    } else if current.is_none() {
        "Pluck one string"
    } else if in_tune {
        "In tune"
    } else if deviation.unwrap() < 0.0 {
        "Too low · tighten string"
    } else {
        "Too high · loosen string"
    };
    let readout = deviation.map(|c| format!("{c:+.0}")).unwrap_or("—".into());
    let actual_hz = current
        .map(|p| format!("{:.1} Hz", p.frequency))
        .unwrap_or("No pitch yet".into());

    use_effect(move || {
        let tuning = if ukulele() {
            UKULELE[tuning_index()]
        } else {
            GUITAR[tuning_index()]
        };
        if let Some(p) = detected() {
            automatic_index.set(pitch::nearest_string(
                p.frequency,
                tuning.strings,
                reference(),
            ));
        }
    });
    use_drop(move || {
        document::eval("window.tunerSession?.stop(); window.tunerTone?.stop();");
    });

    rsx! {
        style { "{CSS}" }
        document::Title { "tuner.murray.kiwi" }
        main { class: "shell",
            header { class: "header",
                a { class: "brand", href: "/", aria_label: "tuner.murray.kiwi home",
                    "tuner.murray.kiwi"
                }
            }
            section { class: "tuner", aria_label: "Instrument tuner",
                div { class: "controls",
                    div { class: "instrument-switch", aria_label: "Instrument",
                        button { class: if !ukulele() { "chosen" } else { "" }, aria_pressed: (!ukulele()).to_string(),
                            onclick: move |_| { ukulele.set(false); tuning_index.set(0); selected.set(None); automatic_index.set(0); detected.set(None); }, "Guitar" }
                        button { class: if ukulele() { "chosen" } else { "" }, aria_pressed: ukulele().to_string(),
                            onclick: move |_| { ukulele.set(true); tuning_index.set(0); selected.set(None); automatic_index.set(0); detected.set(None); }, "Ukulele" }
                    }
                    label { class: "tuning-select",
                        span { class: "sr-only", "Tuning" }
                        select { value: "{tuning_index}", onchange: move |event| {
                            if let Ok(value) = event.value().parse::<usize>() { tuning_index.set(value); selected.set(None); automatic_index.set(0); detected.set(None); }
                        },
                            for (i, option) in tunings.iter().enumerate() { option { value: "{i}", selected: i == tuning_index(), "{option.name}" } }
                        }
                    }
                }
                div { class: if in_tune { "display in-tune" } else { "display" },
                    div { class: "status", role: "status", span { class: if listening() { "status-light active" } else { "status-light" } } "{status}" }
                    div { class: "pitch-note", "{note}" sub { "{octave}" } }
                    div { class: "pitch-details", span { "{actual_hz}" } span { class: "detail-separator", "/" } span { "Target {target:.1} Hz" } }
                    div { class: "meter", aria_label: "Pitch deviation", role: "img",
                        div { class: "tune-zone" }
                        div { class: "meter-ticks",
                            for tick in 0..=20 { span { class: if tick == 10 { "tick center" } else if tick % 5 == 0 { "tick major" } else { "tick" } } }
                        }
                        div { class: if current.is_some() { "needle visible" } else { "needle" }, style: "left: {meter_position}%;" }
                    }
                    div { class: "meter-labels", span { "♭" } span { "−50" } span { "0" } span { "+50" } span { "♯" } }
                    div { class: "cents", "{readout}" span { " cents" } }
                }
                div { class: "strings-section",
                    div { class: "strings-heading", span { "Your strings" }
                        button { class: if selected().is_none() { "auto active" } else { "auto" }, aria_pressed: selected().is_none().to_string(), onclick: move |_| selected.set(None), span { class: "auto-dot" } "Auto detect" }
                    }
                    div { class: "strings", style: "--strings: {tuning.strings.len()};",
                        for (i, string) in tuning.strings.iter().enumerate() {
                            button { class: if index == i { "string selected" } else { "string" }, aria_pressed: (selected() == Some(i)).to_string(), aria_label: format!("String {}, {}{}, lock target", tuning.strings.len()-i, NAMES[*string as usize%12], string/12-1),
                                onclick: move |_| selected.set(Some(i)),
                                span { class: "string-number", "{tuning.strings.len()-i}" }
                                span { class: "string-note", "{NAMES[*string as usize%12]}" sub { "{string/12-1}" } }
                                span { class: "string-line" }
                            }
                        }
                    }
                    p { class: "string-help", if selected().is_none() { "Tap a string to lock the target." } else { "String locked. Tap Auto detect to unlock." } }
                }
                div { class: "listen-area",
                    button { class: if listening() { "listen-button listening" } else { "listen-button" },
                        onclick: move |_| {
                            if listening() {
                                generation += 1; listening.set(false); ready.set(false); detected.set(None);
                                document::eval("window.tunerSession?.stop();");
                            } else {
                                generation += 1;
                                let run = generation();
                                listening.set(true); ready.set(false); error.set(String::new());
                                spawn(async move {
                                    let mut audio = document::eval(AUDIO);
                                    while let Ok(message) = audio.recv::<AudioMessage>().await {
                                        if generation() != run { break; }
                                        match message.kind.as_str() {
                                            "ready" => ready.set(true),
                                            "samples" => {
                                                if !tone_playing() {
                                                    let next = pitch::detect(&message.samples, message.sample_rate);
                                                    // Smooth only nearby pitches so string changes remain immediate.
                                                    detected.set(next.map(|mut p| {
                                                        if let Some(old) = detected.peek().as_ref()
                                                            && pitch::cents(p.frequency, old.frequency).abs() < 40.0 { p.frequency = old.frequency * 0.55 + p.frequency * 0.45; }
                                                        p
                                                    }));
                                                }
                                            }
                                            "error" => { error.set(message.message); listening.set(false); ready.set(false); detected.set(None); break; }
                                            "stopped" => { listening.set(false); ready.set(false); detected.set(None); break; }
                                            _ => {}
                                        }
                                    }
                                    if generation() == run && listening() { listening.set(false); ready.set(false); detected.set(None); }
                                });
                            }
                        },
                        span { aria_hidden: "true", if listening() { "■" } else { "◉" } }
                        if listening() { "Stop listening" } else { "Start tuning" }
                    }
                    p { if listening() { "Listening through your microphone" } else { "Allow your microphone to get started" } }
                    if !error().is_empty() { p { class: "error", role: "alert", "{error}" } }
                }
                div { class: "bottom-controls",
                    label { class: "calibration", "A₄" select { aria_label: "Reference pitch", value: "{reference}", onchange: move |event| { if let Ok(value) = event.value().parse::<f64>() { reference.set(value); } },
                        for hz in 432..=446 { option { value: "{hz}", selected: hz as f64 == reference(), "{hz} Hz" } }
                    } }
                    button { class: "tone-button", disabled: tone_playing(), onclick: move |_| {
                        tone_playing.set(true); detected.set(None);
                        spawn(async move {
                            let script = format!(r#"
                                window.tunerTone?.stop();
                                const ctx = new (window.AudioContext || window.webkitAudioContext)();
                                window.tunerTone = {{ stop: () => ctx.close().catch(() => {{}}) }};
                                try {{
                                    await ctx.resume();
                                    const osc = ctx.createOscillator(); const gain = ctx.createGain();
                                    osc.frequency.value = {target}; osc.type = 'sine';
                                    gain.gain.setValueAtTime(0, ctx.currentTime);
                                    gain.gain.linearRampToValueAtTime(0.15, ctx.currentTime + 0.03);
                                    gain.gain.setValueAtTime(0.15, ctx.currentTime + 1.4);
                                    gain.gain.linearRampToValueAtTime(0, ctx.currentTime + 1.6);
                                    osc.connect(gain); gain.connect(ctx.destination); osc.start(); osc.stop(ctx.currentTime + 1.65);
                                    await new Promise(resolve => osc.onended = resolve);
                                }} finally {{ await ctx.close(); }}
                                return true;
                            "#);
                            let result = document::eval(&script).await;
                            tone_playing.set(false);
                            if result.is_err() { error.set("Could not play the reference tone. Check your audio output and try again.".into()); }
                        });
                    }, span { aria_hidden: "true", "♫" } if tone_playing() { "Playing…" } else { "Play {note}{octave}" } }
                }
            }
            footer { "Your audio stays on your device." }
        }
    }
}
