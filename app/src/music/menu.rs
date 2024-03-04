use currawong::prelude::*;

struct Effects {
    drum_volume: Sf64,
    drum_low_pass_filter: Sf64,
}

impl Effects {
    fn new() -> Self {
        Self {
            drum_volume: const_(0.4),
            drum_low_pass_filter: const_(0.3),
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

fn voice1(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
    effect_x: Sf64,
    effect_y: Sf64,
) -> Sf64 {
    let oscillator = oscillator_hz(Waveform::Saw, note.freq_hz()).build()
        + oscillator_hz(Waveform::Saw, note.freq_hz() / 2.0).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .decay_s(0.9)
        .sustain_01(0.1)
        .release_s(0.0)
        .build()
        .exp_01(1.0);
    oscillator.filter(
        low_pass_moog_ladder(env * (30 * note.freq_hz() * effect_x))
            .resonance(effect_y * 1.0)
            .build(),
    )
}

fn voice1_bass(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
) -> Sf64 {
    let oscillator = oscillator_hz(Waveform::Saw, note.freq_hz()).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.1)
        .sustain_01(1.0)
        .release_s(0.1)
        .build()
        .exp_01(1.0);
    oscillator.filter(low_pass_moog_ladder(env * 1000.0).build())
}

fn arp_shape(trigger: Trigger) -> Signal<ArpeggiatorShape> {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::{cell::RefCell, rc::Rc};
    struct State {
        rng: StdRng,
        indices: Vec<Option<usize>>,
    }
    let state = Rc::new(RefCell::new(State {
        rng: StdRng::from_entropy(),
        indices: vec![Some(0); 8],
    }));
    const MAX_INDEX: usize = 6;
    trigger
        .divide(16)
        .on_unit({
            let state = Rc::clone(&state);
            move || {
                let mut state = state.borrow_mut();
                let index_to_change = state.rng.gen::<usize>() % state.indices.len();
                let value = if state.indices.len() <= 1 || state.rng.gen::<f64>() < 0.9 {
                    Some(state.rng.gen::<usize>() % MAX_INDEX)
                } else {
                    None
                };
                state.indices[index_to_change] = value;
            }
        })
        .map({
            let state = Rc::clone(&state);
            move |()| {
                let state = state.borrow();
                ArpeggiatorShape::Indices(state.indices.clone())
            }
        })
}

fn virtual_key_events(trigger: Trigger) -> Signal<Vec<KeyEvent>> {
    use std::{cell::RefCell, rc::Rc};
    let chords = vec![
        chord(note_name::C, MINOR),
        chord(note_name::C, MINOR),
        chord(note_name::G, MINOR),
    ];
    struct State {
        index: usize,
    }
    let state = Rc::new(RefCell::new(State { index: 0 }));
    trigger
        .divide(32)
        .on({
            let state = Rc::clone(&state);
            move || {
                let octave_base = note::A2;
                let mut state = state.borrow_mut();
                let mut events = Vec::new();
                if state.index > 0 {
                    let prev_chord = chords[(state.index - 1) % chords.len()];
                    prev_chord.with_notes(Inversion::InOctave { octave_base }, |note| {
                        events.push(KeyEvent {
                            note,
                            pressed: false,
                            velocity_01: 1.0,
                        })
                    })
                }
                let current_chord = chords[state.index % chords.len()];
                current_chord.with_notes(Inversion::InOctave { octave_base }, |note| {
                    events.push(KeyEvent {
                        note,
                        pressed: true,
                        velocity_01: 1.0,
                    })
                });
                state.index += 1;
                events
            }
        })
        .map(|opt| if let Some(x) = opt { x } else { Vec::new() })
}

fn virtual_key_events_bass(trigger: Trigger) -> Signal<Vec<KeyEvent>> {
    use std::{cell::RefCell, rc::Rc};
    let notes = vec![note::C2, note::C2, note::G2];
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

pub fn signal() -> Sf64 {
    let effects = Effects::new();
    let _hat_closed = 1 << 0;
    let snare = 1 << 1;
    let kick = 1 << 2;
    let trigger = periodic_trigger_hz(3.0).build();
    let drums = drum_loop(trigger.divide(2), vec![kick, snare]);
    let arp_config = ArpeggiatorConfig::default()
        .shape(arp_shape(trigger.clone()))
        .extend_octaves_high(1);
    let resonance = oscillator_s(Waveform::Sine, 60.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01();
    let cutoff = 0.5
        * oscillator_s(Waveform::Sine, 45.0)
            .reset_offset_01(-0.25)
            .build()
            .signed_to_01()
        + 0.3;
    let room_size = 0.5
        + oscillator_s(Waveform::Sine, 100.0)
            .reset_offset_01(-0.25)
            .build()
            .signed_to_01()
            * 0.5;
    let keys = virtual_key_events(trigger.clone())
        .arpeggiate(trigger.clone(), arp_config)
        .voice_descs_polyphonic(2, 0)
        .into_iter()
        .map(move |voice_desc| voice1(voice_desc, cutoff.clone(), resonance.clone()))
        .sum::<Sf64>()
        .mix(|dry| dry.filter(reverb().room_size(room_size).damping(0.5).build()))
        .filter(high_pass_butterworth(1.0).build());
    let bass_desc = virtual_key_events_bass(trigger.clone()).voice_desc_monophonic();
    let bass = voice1_bass(bass_desc);
    (drums.filter(low_pass_moog_ladder(effects.drum_low_pass_filter * 20000).build())
        * effects.drum_volume)
        + keys * 0.2
        + bass * 0.2
}
