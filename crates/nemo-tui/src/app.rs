//! TUI application with crossterm event loop.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use nemo_layout::BuiltComponent;
use nemo_ui::runtime::NemoRuntime;
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::renderer;

/// The main TUI application.
pub struct TuiApp {
    runtime: Arc<NemoRuntime>,
    should_quit: bool,
}

impl TuiApp {
    pub fn new(runtime: Arc<NemoRuntime>) -> Self {
        Self {
            runtime,
            should_quit: false,
        }
    }

    /// Run the TUI event loop.
    pub fn run(&mut self) -> Result<()> {
        let mut terminal = ratatui::init();

        // Install a panic hook that restores the terminal
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = ratatui::restore();
            original_hook(panic_info);
        }));

        let result = self.event_loop(&mut terminal);

        ratatui::restore();
        result
    }

    fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            // Apply any pending data updates
            self.runtime.apply_pending_data_updates();

            terminal.draw(|frame| self.render(frame))?;

            // Poll for events with 50ms timeout
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            if self.should_quit {
                self.runtime.shutdown();
                break;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let (root_id, components) = self.snapshot_components();

        if let Some(root_id) = root_id {
            if let Some(root) = components.get(&root_id) {
                renderer::render_component(frame, frame.area(), root, &components);
                return;
            }
        }

        // Fallback: show a message
        use ratatui::widgets::Paragraph;
        let msg = Paragraph::new("No layout configured. Check your app.xml.");
        frame.render_widget(msg, frame.area());
    }

    /// Snapshot the current layout components from the runtime.
    fn snapshot_components(&self) -> (Option<String>, HashMap<String, BuiltComponent>) {
        let layout_manager = self
            .runtime
            .layout_manager
            .read()
            .expect("layout_manager lock poisoned");
        let root_id = layout_manager.root_id();
        let components: HashMap<String, BuiltComponent> = layout_manager
            .component_ids()
            .into_iter()
            .filter_map(|id| layout_manager.get_component(&id).cloned().map(|c| (id, c)))
            .collect();
        (root_id, components)
    }
}
