//! Lists the routes the game offers from where the player stands. The screen holds
//! only a cursor: routes and times are read from the game, so it cannot offer a walk
//! the engine would refuse.

use engine::clock::Span;
use engine::game::Game;
use engine::shared::LocationId;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::shared::input::Command;
use crate::shared::menu::{self, Item, Menu};
use crate::shared::theme;

/// What the player's command led to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The screen stays open.
    Stay,
    /// The player chose to walk to this place.
    Go(LocationId),
    /// The player is done.
    Leave,
}

/// An open travel screen.
#[derive(Debug)]
pub struct Travel {
    menu: Menu,
}

/// "45 min" or "1 h 12 min": a walk as the player reads it.
#[must_use]
pub fn walk_time(span: Span) -> String {
    let minutes = span.as_minutes();
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

impl Travel {
    /// Opens the screen over the routes the game offers now.
    #[must_use]
    pub fn open(game: &Game) -> Self {
        Self {
            menu: Menu::new(game.routes().len()),
        }
    }

    /// Moves the cursor, walks on a choice, or leaves on Back.
    pub fn update(&mut self, command: Command, game: &Game) -> Next {
        if command == Command::Back {
            return Next::Leave;
        }
        let Some(index) = self.menu.handle(command) else {
            return Next::Stay;
        };
        game.routes()
            .into_iter()
            .nth(index)
            .map_or(Next::Stay, |route| Next::Go(route.to))
    }

    /// Draws where the player is and the ways out.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let routes = game.routes();
        let mut lines = vec![
            Line::styled("Where do you go?", theme::heading()),
            Line::default(),
        ];
        if routes.is_empty() {
            lines.push(Line::styled("There is no way out from here.", theme::dim()));
        } else {
            let items: Vec<Item> = routes
                .iter()
                .map(|route| Item::new(format!("{} ({})", route.name, walk_time(route.span))))
                .collect();
            let height = usize::from(area.height.saturating_sub(2)).saturating_sub(lines.len());
            lines.extend(menu::window(
                self.menu.lines(&items),
                self.menu.selected(),
                height,
            ));
        }
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Travel ");
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    /// The key help for the footer.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        "Up/Down move | Enter go | 1-9 pick | Esc back | q quit"
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    fn minutes(n: u32) -> Span {
        Span::minutes(NonZeroU32::new(n).expect("not zero"))
    }

    #[test]
    fn a_walk_reads_in_hours_and_minutes() {
        assert_eq!(walk_time(minutes(1)), "1 min");
        assert_eq!(walk_time(minutes(45)), "45 min");
        assert_eq!(walk_time(minutes(60)), "1 h");
        assert_eq!(walk_time(minutes(72)), "1 h 12 min");
        assert_eq!(walk_time(minutes(600)), "10 h");
    }
}
