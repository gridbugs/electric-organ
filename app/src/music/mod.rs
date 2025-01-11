use caw::prelude::*;
use std::{cell::RefCell, rc::Rc};

mod level1;
mod level2;
mod menu;
mod sound_effects;

#[derive(Clone, Copy, Debug)]
pub enum Track {
    Menu,
    Level1,
    Level2,
}

struct Control {
    volume: f64,
    sig_stereo: Stereo<SigBoxed<f32>, SigBoxed<f32>>,
}

impl Control {
    fn new() -> Self {
        Self {
            volume: 1.0,
            sig_stereo: Stereo::new(Sig(0.0).boxed(), Sig(0.0).boxed()),
        }
    }
}

pub struct MusicState {
    control: Rc<RefCell<Control>>,
    sfx: Sfx,
    sig_stereo: Stereo<SigBoxed<f32>, SigBoxed<f32>>,
    sfx_sig: SigBoxed<f32>,
    // This starts as `None` becuse when running in a browser, an audio context can only be created
    // in response to IO.
    player: Option<PlayerAsyncStereo>,
    sfx_player: Option<PlayerAsyncMono>,
}

impl MusicState {
    pub fn new() -> Self {
        let (sfx, sfx_sig) = make_sfx();
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
        Self {
            sfx,
            control,
            sig_stereo,
            sfx_sig,
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
                    .into_async_mono(Default::default())
                    .unwrap(),
            );
        }
        self.player
            .as_mut()
            .unwrap()
            .play_signal_stereo(&mut self.sig_stereo);
        self.sfx_player
            .as_mut()
            .unwrap()
            .play_signal_mono(&mut self.sfx_sig);
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
    fn trig(&self) -> FrameSig<impl FrameSigT<Item = bool>> {
        let state = Rc::clone(&self.state);
        FrameSig::from_fn(move |_ctx| {
            let mut state = state.borrow_mut();
            let prev_state = *state;
            *state = false;
            prev_state
        })
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

fn make_sfx() -> (Sfx, SigBoxed<f32>) {
    let sfx = Sfx {
        pistol: SfxTrigger::new(),
        shotgun: SfxTrigger::new(),
        rocket: SfxTrigger::new(),
        explosion: SfxTrigger::new(),
        melee: SfxTrigger::new(),
        death: SfxTrigger::new(),
    };
    let sig = (sound_effects::pistol(sfx.pistol.trig())
        + sound_effects::shotgun(sfx.shotgun.trig())
        + sound_effects::rocket(sfx.rocket.trig())
        + sound_effects::explosion(sfx.explosion.trig())
        + sound_effects::melee(sfx.melee.trig())
        + sound_effects::death(sfx.death.trig()))
    .filter(reverb::default())
    .boxed();
    (sfx, sig)
}
