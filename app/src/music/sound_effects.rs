use currawong::prelude::*;

pub fn pistol(trigger: Trigger) -> Sf64 {
    let make_noise = || noise().filter(sample_and_hold(trigger.clone()).build());
    let duration = 0.2;
    let env = adsr_linear_01(trigger.to_gate())
        .key_press(&trigger)
        .release_s(duration)
        .build()
        .exp_01(1.0);
    let osc = oscillator_hz(
        Waveform::Pulse,
        (&env * (300.0 + make_noise() * 100)) + 20.0,
    )
    .pulse_width_01(0.1)
    .build();
    let filtered_osc =
        osc.filter(low_pass_moog_ladder(&env * (10000.0 + make_noise() * 5000)).build());
    (filtered_osc * &env)
        .mix(|dry| dry.filter(reverb().room_size(0.8).build()))
        .lazy_zero(&env)
}

pub fn shotgun(trigger: Trigger) -> Sf64 {
    let make_noise = || noise().filter(sample_and_hold(trigger.clone()).build());
    let duration = 0.2;
    let env = adsr_linear_01(trigger.to_gate())
        .key_press(&trigger)
        .release_s(duration)
        .build()
        .exp_01(1.0);
    let osc = oscillator_hz(Waveform::Sine, (&env * (100.0 + make_noise() * 100)) + 20.0)
        .pulse_width_01(0.1)
        .build();
    let noise = noise().filter(low_pass_moog_ladder(2000.0).build()) * 10.0;
    let filtered_osc = (osc + noise).filter(
        low_pass_moog_ladder(&env * (10000.0 + make_noise() * 5000))
            .resonance(2.0)
            .build(),
    );
    filtered_osc
        .mix(|dry| dry.filter(reverb().room_size(0.8).build()))
        .lazy_zero(&env)
}

pub fn rocket(trigger: Trigger) -> Sf64 {
    let make_noise = || noise().filter(sample_and_hold(trigger.clone()).build());
    let duration = 0.8;
    let env = adsr_linear_01(trigger.to_gate_with_duration_s(duration))
        .key_press(&trigger)
        .attack_s(duration)
        .build()
        .exp_01(1.0);
    let noise = noise().filter(low_pass_moog_ladder(8000.0).build()) * 10.0;
    let filtered_osc =
        noise.filter(low_pass_moog_ladder(&env * (5000.0 + make_noise() * 1000)).build());
    filtered_osc
        .mix(|dry| dry.filter(reverb().room_size(0.8).build()))
        .lazy_zero(&env)
}

pub fn explosion(trigger: Trigger) -> Sf64 {
    let make_noise = || noise().filter(sample_and_hold(trigger.clone()).build());
    let duration = 0.8;
    let sweep = (adsr_linear_01(trigger.to_gate_with_duration_s(duration))
        .key_press(&trigger)
        .attack_s(duration / 4.0)
        .decay_s(3.0 * (duration / 4.0))
        .sustain_01(0.0)
        .build()
        .exp_01(1.0)
        + adsr_linear_01(trigger.to_gate())
            .key_press(&trigger)
            .release_s(duration)
            .build()
            .exp_01(1.0))
        / 2.0;
    let osc = oscillator_hz(
        Waveform::Sine,
        (&sweep * (100.0 + make_noise() * 100)) + 20.0,
    )
    .pulse_width_01(0.1)
    .build();
    let env = &sweep;
    let noise = noise().filter(low_pass_moog_ladder(2000.0).build()) * 10.0;
    let filtered_osc = (osc + noise).filter(
        low_pass_moog_ladder(env * (10000.0 + make_noise() * 5000))
            .resonance(2.0)
            .build(),
    );
    filtered_osc.mix(|dry| dry.filter(reverb().room_size(0.8).build()))
}
