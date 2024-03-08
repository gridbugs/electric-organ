use currawong::prelude::*;

pub fn melee(trigger: Trigger) -> Sf64 {
    kick(trigger)
        .build()
        .filter(low_pass_moog_ladder(4000.0).build())
        * 2.0
}

pub fn death(trigger: Trigger) -> Sf64 {
    let make_noise = || noise().filter(sample_and_hold(trigger.clone()).build());
    let duration = 1.0;
    let env = adsr_linear_01(trigger.to_gate())
        .key_press(&trigger)
        .release_s(duration)
        .build()
        .exp_01(1.0);
    let osc = oscillator_hz(
        Waveform::Pulse,
        (&env * (200.0 + make_noise() * 100)) + 50.0,
    )
    .pulse_width_01(0.5)
    .build();
    let filtered_osc = osc
        .filter(down_sample(((1.0 - &env) * 100.0) + 1.0).build())
        .filter(quantize(10.0 * &env).build());
    filtered_osc.lazy_zero(&trigger.to_gate_with_duration_s(duration).to_01()) * 0.4
}

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
        osc.filter(low_pass_moog_ladder(&env * (4000.0 + make_noise() * 2000)).build());
    (filtered_osc * &env).lazy_zero(&env)
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
    filtered_osc.lazy_zero(&env)
}

pub fn rocket(trigger: Trigger) -> Sf64 {
    let make_noise = || noise().filter(sample_and_hold(trigger.clone()).build());
    let duration = 0.4;
    let env = adsr_linear_01(trigger.to_gate_with_duration_s(duration))
        .key_press(&trigger)
        .decay_s(duration)
        .sustain_01(0.0)
        .build()
        .exp_01(1.0);
    let noise = noise().filter(low_pass_moog_ladder(8000.0).build()) * 10.0;
    let filtered_osc =
        noise.filter(low_pass_moog_ladder(&env * (5000.0 + make_noise() * 1000)).build());
    filtered_osc.lazy_zero(&env)
}

pub fn explosion(trigger: Trigger) -> Sf64 {
    let make_noise = || noise().filter(sample_and_hold(trigger.clone()).build());
    let duration = 0.8;
    let env = (adsr_linear_01(trigger.to_gate_with_duration_s(duration))
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
    let osc = oscillator_hz(Waveform::Sine, (&env * (100.0 + make_noise() * 100)) + 20.0)
        .pulse_width_01(0.1)
        .build();
    let noise = noise().filter(low_pass_moog_ladder(2000.0).build()) * 10.0;
    let filtered_osc =
        (osc + noise).filter(low_pass_moog_ladder(&env * (10000.0 + make_noise() * 5000)).build());
    filtered_osc.lazy_zero(&env)
}
