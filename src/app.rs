use std::io::Stdout;

use crossterm::{
    event::{self, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{Screen, ScreenAction};

pub struct App {
    screens: Vec<Box<dyn Screen>>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    running: bool,
}

impl App {
    pub fn new(initial: impl Screen + 'static) -> anyhow::Result<Self> {
        terminal::enable_raw_mode()?;

        let mut stdout = std::io::stdout();

        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            screens: vec![Box::new(initial)],
            terminal,
            running: true,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        while self.running {
            self.terminal.draw(|frame| {
                if let Some(screen) = self.screens.last_mut() {
                    screen.draw(frame);
                }
            })?;

            let event = event::read()?;

            if let Some(screen) = self.screens.last_mut() {
                let action = screen.handle(event)?;
                self.handle(action);
            }
        }

        Ok(())
    }

    fn handle(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Quit => self.running = false,
            ScreenAction::Push(screen) => self.screens.push(screen),
            ScreenAction::Pop => {
                self.screens.pop();

                if self.screens.is_empty() {
                    self.running = false;
                }
            }
            ScreenAction::Replace(screen) => {
                self.screens.pop();
                self.screens.push(screen);
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();

        let _ = execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        );

        let _ = self.terminal.show_cursor();
    }
}
