use std::{
    fmt,
    io::Stdout,
    sync::{
        Once,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crossterm::{
    Command,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste,
        EnableMouseCapture,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{Screen, ScreenAction};

/// Maximum number of queued events handled before a frame is drawn, so a flood of input can't
/// starve drawing.
const MAX_EVENTS_PER_FRAME: usize = 1024;

/// Whether the terminal is currently set up by an `App` and needs restoring.
static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK: Once = Once::new();

/// Which mouse events the terminal reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseMode {
    /// No mouse events.
    Off,
    /// Clicks, drags and the scroll wheel, but not plain movement.
    ClickDrag,
    /// Every mouse event, including movement with no button held.
    #[default]
    AllMotion,
}

pub struct App {
    screens: Vec<Box<dyn Screen>>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    running: bool,
}

impl App {
    pub fn new(initial: impl Screen + 'static) -> anyhow::Result<Self> {
        install_panic_hook();

        terminal::enable_raw_mode()?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);

        let setup = || -> anyhow::Result<_> {
            let mut stdout = std::io::stdout();

            execute!(
                stdout,
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableBracketedPaste
            )?;

            let backend = CrosstermBackend::new(stdout);
            Ok(Terminal::new(backend)?)
        };

        let terminal = setup().inspect_err(|_| restore_terminal())?;

        Ok(Self {
            screens: vec![Box::new(initial)],
            terminal,
            running: true,
        })
    }

    /// Chooses which mouse events are reported. Defaults to [`MouseMode::AllMotion`].
    pub fn with_mouse(mut self, mode: MouseMode) -> anyhow::Result<Self> {
        let backend = self.terminal.backend_mut();

        execute!(backend, DisableMouseCapture)?;

        match mode {
            MouseMode::Off => {}
            MouseMode::ClickDrag => execute!(backend, EnableClickDragMouseCapture)?,
            MouseMode::AllMotion => execute!(backend, EnableMouseCapture)?,
        }

        Ok(self)
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut last_tick = Instant::now();
        let mut dirty = true;

        while self.running {
            if dirty {
                self.terminal.draw(|frame| {
                    if let Some(screen) = self.screens.last_mut() {
                        screen.draw(frame);
                    }
                })?;

                dirty = false;
            }

            let tick_rate = self.screens.last().and_then(|screen| screen.tick_rate());

            // Wait for input until the next tick is due. Without ticks, `event::read` below blocks
            // until input arrives.
            let has_event = match tick_rate {
                Some(tick_rate) => event::poll(tick_rate.saturating_sub(last_tick.elapsed()))?,
                None => true,
            };

            if has_event {
                // Handle everything already queued, then draw once.
                for _ in 0..MAX_EVENTS_PER_FRAME {
                    let event = event::read()?;

                    if let Some(screen) = self.screens.last_mut() {
                        let action = screen.handle(event)?;
                        self.handle(action);
                    }

                    if !self.running || !event::poll(Duration::ZERO)? {
                        break;
                    }
                }

                dirty = true;
            }

            if let Some(tick_rate) = tick_rate
                && last_tick.elapsed() >= tick_rate
            {
                if let Some(screen) = self.screens.last_mut() {
                    let action = screen.update()?;
                    self.handle(action);
                }

                last_tick = Instant::now();
                dirty = true;
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
        restore_terminal();
    }
}

/// Undoes everything `App::new` did to the terminal. Does nothing if it's already been undone.
fn restore_terminal() {
    if !TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }

    let _ = terminal::disable_raw_mode();

    let _ = execute!(
        std::io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}

/// Restores the terminal before the panic message is printed, so it isn't lost in the alternate
/// screen.
fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            previous(info);
        }));
    });
}

/// Like `EnableMouseCapture`, but without any-event tracking (`?1003h`), so plain mouse movement
/// isn't reported.
struct EnableClickDragMouseCapture;

impl Command for EnableClickDragMouseCapture {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str(concat!(
            // Normal tracking: button press and release
            "\x1b[?1000h",
            // Button-event tracking: motion while a button is held (dragging)
            "\x1b[?1002h",
            // SGR mouse mode: coordinates above 223
            "\x1b[?1006h",
        ))
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        EnableMouseCapture.execute_winapi()
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}
