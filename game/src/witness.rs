use crate::{
    ActionError, Config, ExternalEvent, GameControlFlow, GameOverReason, Input, Menu as GameMenu,
};
use coord_2d::Coord;
use direction::CardinalDirection;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct Game {
    inner_game: crate::Game,
}

#[derive(Serialize, Deserialize)]
pub struct RunningGame {
    game: crate::Game,
}

impl RunningGame {
    pub fn new(game: Game, running: Running) -> Self {
        let _ = running;
        Self {
            game: game.inner_game,
        }
    }

    pub fn into_game(self) -> (Game, Running) {
        (
            Game {
                inner_game: self.game,
            },
            Running(Private),
        )
    }
}

#[derive(Debug)]
struct Private;

#[derive(Debug)]
pub struct Running(Private);

#[derive(Debug)]
pub struct Win(Private);

#[derive(Debug)]
pub struct Menu {
    private: Private,
    pub menu: GameMenu,
}

#[derive(Debug)]
pub struct FireEquipped(Private);

#[derive(Debug)]
pub struct FireBody(Private);

#[derive(Debug)]
pub enum Witness {
    Running(Running),
    GameOver(GameOverReason),
    Win(Win),
    Menu(Menu),
    FireEquipped(FireEquipped),
    FireBody(FireBody),
}

impl Witness {
    fn running(private: Private) -> Self {
        Self::Running(Running(private))
    }
}

impl Menu {
    pub fn cancel(self) -> Witness {
        let Self { private, .. } = self;
        Witness::running(private)
    }
    pub fn commit(self, game: &mut Game, choice: crate::MenuChoice) -> Witness {
        let Self { private, .. } = self;
        game.witness_handle_choice(choice, private)
    }
}

pub enum ControlInput {
    Walk(CardinalDirection),
    Wait,
}

pub fn new_game<R: Rng>(
    config: &Config,
    victories: Vec<crate::Victory>,
    base_rng: &mut R,
) -> (Game, Running) {
    let g = Game {
        inner_game: crate::Game::new(config, victories, base_rng),
    };
    (g, Running(Private))
}

impl Win {
    pub fn into_running(self) -> Running {
        Running(self.0)
    }
}

impl Running {
    pub fn new_panics() -> Self {
        panic!("this constructor is meant for temporary use during debugging to get the code to compile")
    }

    /// Call this method with the knowledge that you have sinned
    pub fn cheat() -> Self {
        Self(Private)
    }

    pub fn into_witness(self) -> Witness {
        Witness::Running(self)
    }

    pub fn tick(self, game: &mut Game, since_last_tick: Duration, config: &Config) -> Witness {
        let Self(private) = self;
        game.witness_handle_tick(since_last_tick, config, private)
    }

    pub fn walk(
        self,
        game: &mut Game,
        direction: CardinalDirection,
    ) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        game.witness_handle_input(Input::Walk(direction), private)
    }

    pub fn wait(self, game: &mut Game) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        game.witness_handle_input(Input::Wait, private)
    }

    pub fn unequip(self, game: &mut Game) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        game.witness_handle_input(Input::Unequip, private)
    }

    pub fn reload(self, game: &mut Game) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        game.witness_handle_input(Input::Reload, private)
    }

    pub fn get(self, game: &mut Game) -> (Witness, Result<(), ActionError>) {
        let Self(private) = self;
        game.witness_handle_input(Input::Get, private)
    }

    pub fn fire_equipped(self) -> Witness {
        Witness::FireEquipped(FireEquipped(self.0))
    }

    pub fn fire_body(self) -> Witness {
        Witness::FireBody(FireBody(self.0))
    }

    pub fn menu(self, menu: GameMenu) -> Witness {
        Witness::Menu(Menu {
            private: self.0,
            menu,
        })
    }
}

impl Game {
    fn witness_handle_input(
        &mut self,
        input: Input,
        private: Private,
    ) -> (Witness, Result<(), ActionError>) {
        match self.inner_game.handle_input(input) {
            Err(e) => (Witness::running(private), Err(e)),
            Ok(None) => (Witness::running(private), Ok(())),
            Ok(Some(GameControlFlow::GameOver(reason))) => (Witness::GameOver(reason), Ok(())),
            Ok(Some(GameControlFlow::Menu(menu))) => {
                (Witness::Menu(Menu { private, menu }), Ok(()))
            }
            Ok(Some(GameControlFlow::Win)) => (Witness::Win(Win(Private)), Ok(())),
        }
    }

    fn handle_control_flow(
        &mut self,
        control_flow: Option<GameControlFlow>,
        private: Private,
    ) -> Witness {
        match control_flow {
            None => Witness::running(private),
            Some(GameControlFlow::GameOver(reason)) => Witness::GameOver(reason),
            Some(GameControlFlow::Win) => Witness::Win(Win(private)),
            Some(GameControlFlow::Menu(menu)) => Witness::Menu(Menu { private, menu }),
        }
    }

    fn witness_handle_tick(
        &mut self,
        since_last_tick: Duration,
        config: &Config,
        private: Private,
    ) -> Witness {
        let control_flow = self.inner_game.handle_tick(since_last_tick, config);
        self.handle_control_flow(control_flow, private)
    }

    fn witness_handle_choice(&mut self, choice: crate::MenuChoice, private: Private) -> Witness {
        let control_flow = self.inner_game.handle_choice(choice);
        self.handle_control_flow(control_flow, private)
    }

    pub fn inner_ref(&self) -> &crate::Game {
        &self.inner_game
    }

    pub fn into_running_game(self, running: Running) -> RunningGame {
        RunningGame::new(self, running)
    }

    pub fn take_external_events(&mut self) -> Vec<ExternalEvent> {
        self.inner_game.take_external_events()
    }
}

impl FireEquipped {
    pub fn cancel(self) -> Witness {
        Witness::Running(Running(self.0))
    }

    pub fn commit(self, game: &mut Game, coord: Coord) -> (Witness, Result<(), ActionError>) {
        game.witness_handle_input(Input::FireEquipped(coord), self.0)
    }
}

impl FireBody {
    pub fn cancel(self) -> Witness {
        Witness::Running(Running(self.0))
    }

    pub fn commit(self, game: &mut Game, coord: Coord) -> (Witness, Result<(), ActionError>) {
        game.witness_handle_input(Input::FireBody(coord), self.0)
    }
}
