use currawong::{prelude::*, signal_player::SignalPlayer};
use std::{cell::RefCell, rc::Rc};

mod level;
mod menu;

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
    signal: Sf64,
    // This starts as `None` becuse when running in a browser, an audio context can only be created
    // in response to IO.
    signal_player: Option<SignalPlayer>,
}

fn make_signal_player() -> SignalPlayer {
    let mut signal_player = SignalPlayer::new().unwrap();
    signal_player.set_buffer_padding_sample_rate_ratio(0.25);
    signal_player
}

impl MusicState {
    pub fn new() -> Self {
        let control = Rc::new(RefCell::new(Control::new()));
        let signal = Signal::from_fn({
            let control = Rc::clone(&control);
            move |ctx| {
                let control = control.borrow();
                let sample = control.signal.sample(ctx);
                sample * control.volume
            }
        });
        Self {
            control,
            signal,
            signal_player: None,
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
    }
}
