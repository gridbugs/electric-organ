use currawong::{prelude::*, signal_player::SignalPlayer};
use std::{cell::RefCell, rc::Rc};

pub mod level;
pub mod menu;

#[derive(Clone, Copy, Debug)]
pub enum Track {
    Menu,
    Level,
}

struct Control {
    pub track: Option<Track>,
    pub volume: f64,
}

impl Control {
    fn new() -> Self {
        Self {
            track: None,
            volume: 1.0,
        }
    }
}

pub struct MusicState {
    control: Rc<RefCell<Control>>,
    signal: Sf64,
    signal_player: SignalPlayer,
}

impl MusicState {
    pub fn new() -> Self {
        let control = Rc::new(RefCell::new(Control::new()));
        let signal = Signal::from_fn({
            let menu = menu::signal();
            let level = level::signal();
            let control = Rc::clone(&control);
            move |ctx| {
                let control = control.borrow();
                let sample = match control.track {
                    Some(Track::Level) => level.sample(ctx),
                    Some(Track::Menu) => menu.sample(ctx),
                    None => 0.0,
                };
                sample * control.volume
            }
        });
        let mut signal_player = SignalPlayer::new().unwrap();
        signal_player.set_buffer_padding_sample_rate_ratio(0.25);
        Self {
            control,
            signal,
            signal_player,
        }
    }

    pub fn set_track(&self, track: Option<Track>) {
        self.control.borrow_mut().track = track;
    }

    pub fn set_volume(&self, volume: f64) {
        self.control.borrow_mut().volume = volume;
    }

    pub fn tick(&mut self) {
        self.signal_player.send_signal(&mut self.signal);
    }
}
