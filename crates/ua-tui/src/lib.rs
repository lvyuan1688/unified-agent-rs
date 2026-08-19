//! ua-tui: terminal UI for unified-agent-rs.
//! Renders the agent loop steps and supports `q` to quit, `↑/↓` to scroll.

use anyhow::Result;
use ua_agent_core::Step;

pub fn render(frame: &mut ratatui::Frame, steps: &[Step], cursor: usize) {
    let area = frame.area();
    let line = steps
        .get(cursor)
        .map(|s| format!("[{}/{}] phase={:?}", cursor + 1, steps.len(), s.phase))
        .unwrap_or_else(|| "(no step)".into());
    frame.render_widget(
        ratatui::widgets::Paragraph::new(line).alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

pub async fn run(steps: Vec<Step>) -> Result<()> {
    let mut terminal = ratatui::init();
    let mut cursor = 0usize;
    loop {
        terminal.draw(|f| render(f, &steps, cursor))?;
        if let ratatui::crossterm::event::Event::Key(key) = ratatui::crossterm::event::read()? {
            match key.code {
                ratatui::crossterm::event::KeyCode::Char('q') => break,
                ratatui::crossterm::event::KeyCode::Down => {
                    cursor = (cursor + 1).min(steps.len().saturating_sub(1))
                }
                ratatui::crossterm::event::KeyCode::Up => cursor = cursor.saturating_sub(1),
                _ => {}
            }
        }
    }
    ratatui::restore();
    Ok(())
}
