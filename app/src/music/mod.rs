use caw::prelude::*;
use currawong::{prelude::*, signal_player::SignalPlayer};
use std::{cell::RefCell, rc::Rc};

mod level1;
mod level2;
mod menu;
mod sound_effects;
mod sound_effects_;

#[derive(Clone, Copy, Debug)]
pub enum Track {
    Menu,
    Level1,
    Level2,
}

struct Control {
    volume: f64,
    signal: Sf64,
    sig_stereo: Stereo<SigBoxed<f32>, SigBoxed<f32>>,
}

impl Control {
    fn new() -> Self {
        Self {
            signal: const_(0.0),
            volume: 1.0,
            sig_stereo: Stereo::new(Sig(0.0).boxed(), Sig(0.0).boxed()),
        }
    }
}

pub struct MusicState {
    control: Rc<RefCell<Control>>,
    sfx: Sfx,
    sig_stereo: Stereo<SigBoxed<f32>, SigBoxed<f32>>,
    // This starts as `None` becuse when running in a browser, an audio context can only be created
    // in response to IO.
    player: Option<PlayerAsyncStereo>,
    sfx_signal: Sf64,
    sfx_signal_player: Option<SignalPlayer>,
    sfx_player: Option<PlayerAsyncStereo>,
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
        let sig_stereo = Stereo::new_fn_channel({
            let control = Rc::clone(&control);
            move |channel| {
                let control = Rc::clone(&control);
                Sig::from_buf_fn(move |ctx, buf| {
                    let mut control = control.borrow_mut();
                    let sig = control.sig_stereo.get_mut(channel);
                    sig.sample_into_buf(ctx, buf);
                    for x in buf.iter_mut() {
                        *x *= control.volume as f32;
                    }
                })
                .boxed()
            }
        });
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
            sig_stereo,
            sfx_signal,
            sfx_signal_player: None,
            player: None,
            sfx_player: None,
        }
    }

    pub fn set_track(&self, track: Option<Track>) {
        let mut control = self.control.borrow_mut();
        control.sig_stereo = match track {
            None => Stereo::new(Sig(0.0).boxed(), Sig(0.0).boxed()),
            Some(Track::Level1) => level1::sig_stereo(),
            Some(Track::Level2) => level2::sig_stereo(),
            Some(Track::Menu) => menu::sig_stereo(),
        };
    }

    pub fn set_volume(&self, volume: f64) {
        self.control.borrow_mut().volume = volume;
    }

    pub fn tick(&mut self) {
        if self.player.is_none() {
            self.player = Some(
                Player::new()
                    .unwrap()
                    .into_async_stereo(Default::default())
                    .unwrap(),
            );
        }
        if self.sfx_player.is_none() {
            self.sfx_player = Some(
                Player::new()
                    .unwrap()
                    .into_async_stereo(Default::default())
                    .unwrap(),
            );
        }
        if self.sfx_signal_player.is_none() {
            self.sfx_signal_player = Some(make_sfx_signal_player());
        }
        self.sfx_signal_player
            .as_mut()
            .unwrap()
            .send_signal(&mut self.sfx_signal);
        self.player
            .as_mut()
            .unwrap()
            .play_signal_stereo(&mut self.sig_stereo);
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
    pub fn sfx_melee(&self) {
        self.sfx.melee.fire()
    }
    pub fn sfx_death(&self) {
        self.sfx.death.fire()
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
    melee: SfxTrigger,
    death: SfxTrigger,
}

fn make_sfx() -> (Sfx, Sf64) {
    let sfx = Sfx {
        pistol: SfxTrigger::new(),
        shotgun: SfxTrigger::new(),
        rocket: SfxTrigger::new(),
        explosion: SfxTrigger::new(),
        melee: SfxTrigger::new(),
        death: SfxTrigger::new(),
    };
    let signal = sum([
        sound_effects::pistol(sfx.pistol.trigger()),
        sound_effects::shotgun(sfx.shotgun.trigger()),
        sound_effects::rocket(sfx.rocket.trigger()),
        sound_effects::explosion(sfx.explosion.trigger()),
        sound_effects::melee(sfx.melee.trigger()),
        sound_effects::death(sfx.death.trigger()),
    ])
    .mix(|dry| dry.filter(reverb().room_size(0.8).build()));
    (sfx, signal)
}
