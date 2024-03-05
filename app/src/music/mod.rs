use currawong::{prelude::*, signal_player::SignalPlayer};
use std::{cell::RefCell, rc::Rc};

mod level;
mod menu;
mod sound_effects;

#[derive(Clone, Copy, Debug)]
pub enum Track {
    Menu,
    Level,
}

struct Control {
    volume: f64,
    signal: Sf64,
}

impl Control {
    fn new() -> Self {
        Self {
            signal: const_(0.0),
            volume: 1.0,
        }
    }
}

pub struct MusicState {
    control: Rc<RefCell<Control>>,
    sfx: Sfx,
    signal: Sf64,
    // This starts as `None` becuse when running in a browser, an audio context can only be created
    // in response to IO.
    signal_player: Option<SignalPlayer>,
    sfx_signal: Sf64,
    sfx_signal_player: Option<SignalPlayer>,
}

fn make_signal_player() -> SignalPlayer {
    let mut signal_player = SignalPlayer::new().unwrap();
    signal_player.set_buffer_padding_sample_rate_ratio(0.25);
    signal_player
}

fn make_sfx_signal_player() -> SignalPlayer {
    let mut signal_player = SignalPlayer::new().unwrap();
    signal_player.set_buffer_padding_sample_rate_ratio(0.05);
    signal_player
}

impl MusicState {
    pub fn new() -> Self {
        let (sfx, sfx_signal) = make_sfx();
        let control = Rc::new(RefCell::new(Control::new()));
        let signal = Signal::from_fn({
            let control = Rc::clone(&control);
            move |ctx| {
                let control = control.borrow();
                let sample = control.signal.sample(ctx);
                sample * control.volume
            }
        });
        let sfx_signal = Signal::from_fn({
            let control = Rc::clone(&control);
            move |ctx| sfx_signal.sample(ctx) * control.borrow().volume
        });
        Self {
            sfx,
            control,
            signal,
            signal_player: None,
            sfx_signal,
            sfx_signal_player: None,
        }
    }

    pub fn set_track(&self, track: Option<Track>) {
        let mut control = self.control.borrow_mut();
        control.signal = match track {
            None => const_(0.0),
            Some(Track::Level) => level::signal(),
            Some(Track::Menu) => menu::signal(),
        }
    }

    pub fn set_volume(&self, volume: f64) {
        self.control.borrow_mut().volume = volume;
    }

    pub fn tick(&mut self) {
        if self.signal_player.is_none() {
            self.signal_player = Some(make_signal_player());
        }
        self.signal_player
            .as_mut()
            .unwrap()
            .send_signal(&mut self.signal);
        if self.sfx_signal_player.is_none() {
            self.sfx_signal_player = Some(make_sfx_signal_player());
        }
        self.sfx_signal_player
            .as_mut()
            .unwrap()
            .send_signal(&mut self.sfx_signal);
    }

    pub fn sfx_pistol(&self) {
        self.sfx.pistol.fire()
    }
    pub fn sfx_shotgun(&self) {
        self.sfx.shotgun.fire()
    }
    pub fn sfx_rocket(&self) {
        self.sfx.rocket.fire()
    }
    pub fn sfx_explosion(&self) {
        self.sfx.explosion.fire()
    }
}

struct SfxTrigger {
    state: Rc<RefCell<bool>>,
}

impl SfxTrigger {
    fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(false)),
        }
    }
    fn fire(&self) {
        *self.state.borrow_mut() = true;
    }
    fn trigger(&self) -> Trigger {
        let state = Rc::clone(&self.state);
        Gate::from_fn(move |_| {
            let mut state = state.borrow_mut();
            let prev_state = *state;
            *state = false;
            prev_state
        })
        .to_trigger_rising_edge()
    }
}

struct Sfx {
    pistol: SfxTrigger,
    shotgun: SfxTrigger,
    rocket: SfxTrigger,
    explosion: SfxTrigger,
}

fn make_sfx() -> (Sfx, Sf64) {
    let sfx = Sfx {
        pistol: SfxTrigger::new(),
        shotgun: SfxTrigger::new(),
        rocket: SfxTrigger::new(),
        explosion: SfxTrigger::new(),
    };
    let signal = sum([
        sound_effects::pistol(sfx.pistol.trigger()),
        sound_effects::shotgun(sfx.shotgun.trigger()),
        sound_effects::rocket(sfx.rocket.trigger()),
        sound_effects::explosion(sfx.explosion.trigger()),
    ]);
    (sfx, signal)
}
