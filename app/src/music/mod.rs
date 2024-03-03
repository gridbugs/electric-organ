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
    signal_player: SignalPlayer,
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
        let mut signal_player = SignalPlayer::new().unwrap();
        signal_player.set_buffer_padding_sample_rate_ratio(0.25);
        Self {
            control,
            signal,
            signal_player,
        }
    }

    pub fn set_track(&self, track: Option<Track>) {
        let mut control = self.control.borrow_mut();
        control.signal = match track {
            None => const_(0.0),
            Some(Track::Level) => level::signal(),
            Some(Track::Menu) => menu::signal(),
        } * 0.0
    }

    pub fn set_volume(&self, volume: f64) {
        self.control.borrow_mut().volume = volume;
    }

    pub fn tick(&mut self) {
        self.signal_player.send_signal(&mut self.signal);
    }
}
