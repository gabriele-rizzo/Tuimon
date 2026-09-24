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

    /// Called once per tick, whether or not input arrived.
    fn update(&mut self) -> anyhow::Result<ScreenAction> {
        Ok(ScreenAction::None)
    }
}
