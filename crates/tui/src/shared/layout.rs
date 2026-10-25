//! The persistent screen layout: status line on top, the scene and the log on the
//! left, the quest panel on the right, the available keys at the bottom.

use ratatui::layout::{Constraint, Layout, Rect};

/// Narrowest terminal the layout is laid out for.
pub const MIN_WIDTH: u16 = 70;
/// Shortest terminal the layout is laid out for.
pub const MIN_HEIGHT: u16 = 22;

const LOG_HEIGHT: u16 = 8;

/// Where each persistent region goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regions {
    /// One line: where the player is.
    pub status: Rect,
    /// The current screen's own content.
    pub scene: Rect,
    /// What was said and done recently.
    pub log: Rect,
    /// The active quests.
    pub quests: Rect,
    /// The keys that work right now.
    pub footer: Rect,
}

/// Splits `area` into the regions, or `None` when it is too small to be usable.
#[must_use]
pub fn regions(area: Rect) -> Option<Regions> {
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        return None;
    }
    let [status, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    let [left, quests] =
        Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(body);
    let [scene, log] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(LOG_HEIGHT)]).areas(left);
    Some(Regions {
        status,
        scene,
        log,
        quests,
        footer,
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn contained(inner: Rect, outer: Rect) -> bool {
        outer.intersection(inner) == inner
    }

    proptest! {
        #[test]
        fn every_region_fits_in_the_area_and_none_overlap(
            x in 0u16..20, y in 0u16..20,
            width in MIN_WIDTH..300, height in MIN_HEIGHT..120,
        ) {
            let area = Rect::new(x, y, width, height);
            let r = regions(area).expect("large enough");
            let all = [r.status, r.scene, r.log, r.quests, r.footer];
            for (i, a) in all.iter().enumerate() {
                prop_assert!(contained(*a, area));
                prop_assert!(a.area() > 0);
                for b in &all[i + 1..] {
                    prop_assert!(!a.intersects(*b));
                }
            }
        }

        #[test]
        fn a_terminal_below_the_minimum_gets_no_layout(
            width in 0u16..MIN_WIDTH, height in 0u16..400,
        ) {
            prop_assert!(regions(Rect::new(0, 0, width, height)).is_none());
        }
    }
}
