use currawong::prelude::*;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

struct Effects {
    tempo: Sf64,
    drum_volume: Sf64,
    drum_low_pass_filter: Sf64,
}

impl Effects {
    fn new() -> Self {
        Self {
            tempo: const_(0.6),
            drum_volume: const_(0.5),
            drum_low_pass_filter: const_(0.5),
        }
    }
}

fn drum_loop(trigger: Trigger, pattern: Vec<u8>) -> Sf64 {
    drum_loop_8(
        trigger.divide(1),
        pattern,
        vec![
            triggerable::hat_closed().build(),
            triggerable::snare().build(),
            triggerable::kick().build(),
        ],
    )
}

fn voice4(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
    effect_x: Sf64,
    effect_y: Sf64,
    effect_z: Sf64,
) -> Sf64 {
    let lfo = oscillator_hz(Waveform::Sine, 8.0).build().signed_to_01();
    let oscillator = oscillator_hz(Waveform::Pulse, note.freq_hz())
        .pulse_width_01(effect_z)
        .build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .decay_s(1.0)
        .sustain_01(0.1)
        .release_s(0.0)
        .build()
        .exp_01(1.0);
    oscillator
        .filter(low_pass_moog_ladder(env * effect_x * 10000.0 + lfo * 2000.0 * effect_y).build())
}

fn bass_voice(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
) -> Sf64 {
    let freq = note.freq_hz() / 2;
    let osc = oscillator_hz(Waveform::Triangle, &freq).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .release_s(0.1)
        .build()
        .exp_01(1.0);
    osc.filter(low_pass_moog_ladder(5000.0).build()) * env
}

fn virtual_key_events_bass(trigger: Trigger) -> Signal<Vec<KeyEvent>> {
    use std::{cell::RefCell, rc::Rc};
    let notes = vec![note::C2, note::E2, note::F2, note::F2];
    struct State {
        index: usize,
    }
    let state = Rc::new(RefCell::new(State { index: 0 }));
    trigger
        .divide(32)
        .on({
            let state = Rc::clone(&state);
            move || {
                let mut state = state.borrow_mut();
                let mut events = Vec::new();
                if state.index > 0 {
                    let prev_note = notes[(state.index - 1) % notes.len()];
                    events.push(KeyEvent {
                        note: prev_note,
                        pressed: false,
                        velocity_01: 1.0,
                    })
                }
                let current_note = notes[state.index % notes.len()];
                events.push(KeyEvent {
                    note: current_note,
                    pressed: true,
                    velocity_01: 1.0,
                });
                state.index += 1;
                events
            }
        })
        .map(|opt| if let Some(x) = opt { x } else { Vec::new() })
}

fn virtual_key_events(trigger: Trigger) -> Signal<Vec<KeyEvent>> {
    use std::{cell::RefCell, rc::Rc};
    let chords = vec![
        chord(note_name::C, MINOR),
        chord(note_name::E, MAJOR),
        chord(note_name::F, MAJOR),
        chord(note_name::F, MINOR),
    ];
    struct State {
        index: usize,
    }
    let state = Rc::new(RefCell::new(State { index: 0 }));
    trigger
        .divide(32)
        .on({
            let state = Rc::clone(&state);
            let inversion = Inversion::InOctave {
                octave_base: note::A2,
            };
            move || {
                let mut state = state.borrow_mut();
                let mut events = Vec::new();
                if state.index > 0 {
                    let prev_chord = chords[(state.index - 1) % chords.len()];
                    let mut count = 0;
                    prev_chord.with_notes(inversion, |note| {
                        events.push(KeyEvent {
                            note,
                            pressed: false,
                            velocity_01: 1.0,
                        });
                        if count < 2 {
                            events.push(KeyEvent {
                                note: note.add_octaves(1),
                                pressed: false,
                                velocity_01: 1.0,
                            });
                        }
                        count += 1;
                    })
                }
                let mut count = 0;
                let current_chord = chords[state.index % chords.len()];
                current_chord.with_notes(inversion, |note| {
                    events.push(KeyEvent {
                        note,
                        pressed: true,
                        velocity_01: 1.0,
                    });
                    if count < 2 {
                        events.push(KeyEvent {
                            note: note.add_octaves(1),
                            pressed: true,
                            velocity_01: 1.0,
                        });
                    }
                    count += 1;
                });
                state.index += 1;
                events
            }
        })
        .map(|opt| if let Some(x) = opt { x } else { Vec::new() })
}

pub fn signal() -> Sf64 {
    let effects = Effects::new();
    let _hat_closed = 1 << 0;
    let snare = 1 << 1;
    let kick = 1 << 2;

    let trigger = periodic_trigger_hz(effects.tempo * 8).build();
    let mut rng = StdRng::from_entropy();
    let mut snares = (0..11).map(|_| 0).collect::<Vec<_>>();
    for _ in 0..4 {
        snares.push(snare);
    }
    snares.shuffle(&mut rng);
    let snares = drum_loop(trigger.clone(), snares);

    #[rustfmt::skip]
    let drums2 = drum_loop(
        trigger.clone(),
        vec![
            kick,
            0,
            0,
            0,
            kick,
            0,
            0,
            0,
        ],
    ) + snares.clone();

    let drums = trigger.divide(128).to_signal().map_ctx({
        use std::cell::Cell;
        let count = Cell::new(-1);
        move |value, ctx| {
            if value {
                count.set(count.get() + 1);
            }
            drums2.sample(ctx)
        }
    });
    let arp_config = ArpeggiatorConfig::default()
        .shape(ArpeggiatorShape::Down)
        .extend_octaves_high(0);
    let distortion = oscillator_s(Waveform::Sine, 89.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01();
    let resonance = oscillator_s(Waveform::Sine, 127.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        * 0.3;
    let cutoff = oscillator_s(Waveform::Sine, 53.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        * 0.3
        + 0.1;
    let effect_z = oscillator_s(Waveform::Sine, 97.0).build().signed_to_01() * 0.3 + 0.2;
    let keys = virtual_key_events(trigger.clone())
        .arpeggiate(trigger.clone(), arp_config)
        .voice_descs_polyphonic(1, 0)
        .into_iter()
        .map(|voice_desc| {
            voice4(
                voice_desc,
                cutoff.clone(),
                resonance.clone(),
                effect_z.clone(),
            )
        })
        .sum::<Sf64>()
        .filter(
            compress()
                .scale(1.0 + &distortion * 4.0)
                .threshold(1.0 - &distortion * 0.5)
                .build(),
        )
        .mix(|dry| dry.filter(reverb().room_size(0.9).damping(0.5).build()))
        .filter(high_pass_butterworth(1.0).build());
    let bass = bass_voice(virtual_key_events_bass(trigger.clone()).voice_desc_monophonic());
    (drums.filter(low_pass_moog_ladder(effects.drum_low_pass_filter * 20000).build())
        * effects.drum_volume)
        + keys * (0.6 - cutoff)
        + bass * 0.2
}
