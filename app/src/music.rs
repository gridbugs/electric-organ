use currawong::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

const C_MAJOR_SCALE: &[NoteName] = &[
    NoteName::A,
    NoteName::B,
    NoteName::C,
    NoteName::D,
    NoteName::E,
    NoteName::F,
    NoteName::G,
];

fn make_scale_base_freqs(note_names: &[NoteName]) -> Vec<Sfreq> {
    note_names
        .into_iter()
        .map(|&name| const_(Note::new(name, OCTAVE_0).freq()))
        .collect()
}

fn random_note_c_major(base_hz: Sf64, range_hz: Sf64) -> Sfreq {
    sfreq_hz(base_hz + (noise_01() * range_hz))
        .filter(quantize_to_scale(make_scale_base_freqs(C_MAJOR_SCALE)).build())
}

fn voice(freq: Sfreq, gate: Gate, effect1: Sf64, effect2: Sf64) -> Sf64 {
    let freq_hz = freq.hz();
    let osc = oscillator_hz(Waveform::Saw, freq_hz.clone()).build();
    let env_amp = adsr_linear_01(&gate).attack_s(0.01).release_s(0.5).build();
    let env_lpf = adsr_linear_01(&gate)
        .attack_s(0.01)
        .release_s(0.5)
        .build()
        .exp_01(1.0);
    osc.filter(
        low_pass_moog_ladder(1000.0 + 2000.0 * env_lpf * effect1)
            .resonance(1.0 * &effect2)
            .build(),
    )
    .filter(compress().scale(effect2 * 4.0).build())
    .mul_lazy(&env_amp)
}

fn random_replace_loop(
    trigger: Trigger,
    anchor: Sfreq,
    palette: Sfreq,
    length: usize,
    replace_probability_01: Sf64,
    anchor_probability_01: Sf64,
) -> Sfreq {
    let mut rng = StdRng::from_entropy();
    let mut sequence: Vec<Option<Freq>> = vec![None; length];
    let mut index = 0;
    let mut anchor_on_0 = false;
    let mut first_note = true;
    Signal::from_fn_mut(move |ctx| {
        let trigger = trigger.sample(ctx);
        if trigger {
            if rng.gen::<f64>() < replace_probability_01.sample(ctx) {
                sequence[index] = Some(palette.sample(ctx));
            }
            if index == 0 {
                anchor_on_0 = rng.gen::<f64>() < anchor_probability_01.sample(ctx);
            } else {
                first_note = false;
            }
        }
        let freq = if first_note {
            anchor.sample(ctx)
        } else if anchor_on_0 && index == 0 {
            anchor.sample(ctx)
        } else if let Some(freq) = sequence[index] {
            freq
        } else {
            let freq = palette.sample(ctx);
            sequence[index] = Some(freq);
            freq
        };
        if trigger {
            index = (index + 1) % sequence.len();
        }
        freq
    })
}

fn synth_signal(trigger: Trigger) -> Sf64 {
    let modulate = 1.0
        - oscillator_s(Waveform::Triangle, 60.0)
            .build()
            .signed_to_01();
    let effect1 = (1.0 - oscillator_s(Waveform::Sine, 47.0).build()).signed_to_01();
    let effect2 = oscillator_s(Waveform::Sine, 67.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01();
    let effect3 = oscillator_s(Waveform::Sine, 51.0).build().signed_to_01();
    let mk_voice = {
        |freq, trigger: Trigger| {
            let trigger = trigger.clone();
            let effect1 = effect1.clone();
            let effect2 = effect2.clone();
            let gate = trigger.to_gate_with_duration_s(0.02);
            voice(freq, gate, effect1.clone(), effect2.clone()).filter(
                compress()
                    .threshold(2.0)
                    .scale(1.0 + &modulate * 8.0)
                    .ratio(0.1)
                    .build(),
            )
        }
    };
    let poly_triggers = trigger_split_cycle(trigger, 2);
    let dry: Sf64 = poly_triggers
        .into_iter()
        .map(move |trigger| {
            let freq = random_replace_loop(
                trigger.clone(),
                const_(Note::new(NoteName::C, OCTAVE_1).freq()),
                random_note_c_major(const_(100.0), const_(300.0)),
                32,
                const_(0.1),
                const_(0.5),
            );
            mk_voice(freq, trigger.random_skip(0.5))
        })
        .sum();
    (dry.filter(reverb().room_size(0.9).build()) * 2.0 + (3.0 * effect3)) + dry
}

fn drum_signal(trigger: Trigger) -> Sf64 {
    const HAT_CLOSED: usize = 0;
    const SNARE: usize = 1;
    const KICK: usize = 2;
    let drum_pattern = {
        let hat_closed = 1 << HAT_CLOSED;
        let snare = 1 << SNARE;
        let kick = 1 << KICK;
        vec![
            hat_closed | kick,
            hat_closed,
            hat_closed | snare,
            hat_closed,
            hat_closed | kick,
            hat_closed,
            hat_closed | snare,
            hat_closed,
            hat_closed | kick,
            hat_closed,
            hat_closed | snare,
            hat_closed,
            hat_closed | kick,
            hat_closed | kick,
            hat_closed | snare,
            hat_closed,
        ]
    };
    let drum_sequence = bitwise_pattern_triggers_8(trigger, drum_pattern).triggers;
    match &drum_sequence.as_slice() {
        &[hat_closed_trigger, snare_trigger, kick_trigger, ..] => {
            hat_closed(hat_closed_trigger.clone()).build()
                + snare(snare_trigger.clone()).build()
                + kick(kick_trigger.clone()).build()
        }
        _ => panic!(),
    }
}

pub fn signal() -> Sf64 {
    let trigger = periodic_trigger_hz(4.0).build();
    synth_signal(trigger.divide(4)) * 0.15 + drum_signal(trigger.divide(4)) * 0.075
}
