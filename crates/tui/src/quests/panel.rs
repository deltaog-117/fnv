//! The persistent panel listing the active quests and what to do next.

use engine::game::Game;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::theme;

/// The panel's lines: each active quest's name over its current journal entry.
/// Sorted by name, because the log keeps quests in no particular order and the
/// panel must not shuffle between frames.
fn lines(game: &Game) -> Vec<Line<'static>> {
    let log = game.quests();
    let mut active: Vec<_> = log
        .active()
        .map(|(quest, stage)| {
            let name = log.book().name(quest).unwrap_or(quest.as_str());
            let entry = stage
                .and_then(|stage| log.book().stage(quest, stage))
                .map(|stage| stage.description.trim())
                .filter(|text| !text.is_empty());
            (name.to_owned(), entry.map(str::to_owned))
        })
        .collect();
    active.sort();
    if active.is_empty() {
        return vec![Line::styled("No active quests.", theme::dim())];
    }
    active
        .into_iter()
        .flat_map(|(name, entry)| {
            [
                Line::styled(name, theme::heading()),
                Line::styled(
                    entry.unwrap_or_else(|| "(no journal entry)".to_owned()),
                    theme::text(),
                ),
                Line::default(),
            ]
        })
        .collect()
}

/// Draws the panel.
pub fn render(frame: &mut Frame, area: Rect, game: &Game) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(theme::dim())
        .title(" Active quests ");
    frame.render_widget(
        Paragraph::new(lines(game))
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}
