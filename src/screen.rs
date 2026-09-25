use std::time::Duration;

use crossterm::event::Event;
use ratatui::Frame;

pub enum ScreenAction {
    None,
    Quit,
    Push(Box<dyn Screen>),
    Pop,
    Replace(Box<dyn Screen>),
}

pub trait Screen {
    fn draw(&mut self, frame: &mut Frame);
    fn handle(&mut self, event: Event) -> anyhow::Result<ScreenAction>;

    /// Called once per tick, whether or not input arrived. Only runs while `tick_rate` returns
    /// `Some`.
    fn update(&mut self) -> anyhow::Result<ScreenAction> {
        Ok(ScreenAction::None)
    }

    /// How often `update` is called (and the UI redrawn) while this screen is on top. `None`
    /// means no ticks: the app sleeps until input arrives and only redraws after it.
    fn tick_rate(&self) -> Option<Duration> {
        None
    }
}
