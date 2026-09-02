//! bastion (INSPECTOR-M2): the READABLE inspector -- a scrollable,
//! collapsible conrod panel over the INSPECTOR-M1 data model.
//!
//! M1 built the model (a section registry, rows that name their
//! producer, a two-clock header) and drew it as one flat `Text` block,
//! which at a full lane table was a 40-line wall the owner called "just
//! a giant list". This module is the promised next row: every section
//! is a heading you can fold, the body scrolls, alarm rows are coloured,
//! and a FOLDED section is still a heading. Folding also stops the
//! subscription asking the server for that section, so folding is
//! cheaper, not just tidier.
//!
//! The model is not touched: `build` is a pure projection of the same
//! `SectionedInspectV1` reply that `lines()` rendered, and the row text
//! comes from the same `row_lines`, so the two views can never disagree
//! on a number.

use common::comp::bastion_inspect::{
    RowSeverityV1, SectionIdV1, SectionSetV1, SectionedInspectV1,
};
use conrod_core::{
    Borderable, Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget, color,
    position::{Place, Relative},
    widget::{self, Button, Rectangle, Scrollbar, Text},
    widget_ids,
};

use super::{header_lines, render, row_lines};
use crate::ui::fonts::Fonts;

/// The panel model for one frame: the two-clock header and EVERY
/// section id in registry order, folded or not.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectPanel {
    pub header: Vec<String>,
    pub sections: Vec<PanelSection>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelSection {
    pub id: SectionIdV1,
    /// The section title, carrying its own age when it is a
    /// carried-forward answer older than the header clocks (see
    /// `RenderedSection`).
    pub title: String,
    pub expanded: bool,
    /// Empty when folded or not yet answered.
    pub rows: Vec<PanelRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelRow {
    pub text: String,
    pub alarm: bool,
}

/// Project a reply into the panel model. `expanded` is the
/// subscription's fold state; a folded section keeps its heading and
/// loses its rows.
pub fn build(
    reply: &SectionedInspectV1,
    age: impl Fn(SectionIdV1) -> Option<u64>,
    expanded: SectionSetV1,
    verbose: bool,
) -> InspectPanel {
    let header = header_lines(&reply.frames, reply.loaded, verbose);
    let rendered = render(reply, age);
    let sections = SectionIdV1::ALL
        .iter()
        .map(|&id| {
            let answered = rendered.iter().find(|s| s.id == id);
            let is_open = expanded.contains(id);
            let title = match answered.map(|s| s.age_ticks) {
                Some(Some(age)) if age > 0 => {
                    format!("{}   (as of {age} server ticks ago)", id.title())
                },
                _ => id.title().to_string(),
            };
            let rows = if is_open {
                answered
                    .map(|s| {
                        s.rows
                            .iter()
                            .zip(row_lines(&s.rows, verbose))
                            .map(|(row, text)| PanelRow {
                                text,
                                alarm: matches!(row.severity(), RowSeverityV1::Alarm),
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            PanelSection {
                id,
                title,
                expanded: is_open,
                rows,
            }
        })
        .collect();
    InspectPanel { header, sections }
}

widget_ids! {
    pub struct Ids {
        bg,
        title,
        scroll_align,
        scrollbar,
        header_txts[],
        section_btns[],
        row_txts[],
    }
}

pub const PANEL_W: f64 = 400.0;
const PAD: f64 = 8.0;

/// Draw the panel at the top-left of the window. Returns the sections
/// whose heading was clicked this frame, for the session to fold.
pub fn draw(
    ids: &mut Ids,
    panel: &InspectPanel,
    fonts: &Fonts,
    ui: &mut UiCell,
) -> Vec<SectionIdV1> {
    let mut toggles = Vec::new();
    let font = fonts.cyri.conrod_id;
    let dim = Color::Rgba(0.72, 0.72, 0.72, 1.0);
    let plain = Color::Rgba(0.95, 0.95, 0.95, 1.0);
    let alarm = Color::Rgba(1.0, 0.55, 0.25, 1.0);
    let heading = Color::Rgba(1.0, 0.85, 0.5, 1.0);

    // The panel stays inside the window whatever the resolution: it is
    // as tall as the window allows, and the body scrolls.
    let h = (ui.win_h - 140.0).clamp(160.0, 760.0);
    Rectangle::fill_with([PANEL_W, h], Color::Rgba(0.0, 0.0, 0.0, 0.72))
        .top_left_with_margins_on(ui.window, 60.0, 10.0)
        .set(ids.bg, ui);
    Text::new("INSPECTOR    click a heading to fold it")
        .font_id(font)
        .font_size(11)
        .color(dim)
        .top_left_with_margins_on(ids.bg, 5.0, PAD)
        .set(ids.title, ui);
    Rectangle::fill_with([PANEL_W - 2.0 * PAD, h - 24.0], color::TRANSPARENT)
        .top_left_with_margins_on(ids.bg, 20.0, PAD)
        .scroll_kids_vertically()
        .set(ids.scroll_align, ui);
    Scrollbar::y_axis(ids.scroll_align)
        .rgba(0.6, 0.6, 0.6, 0.5)
        .thickness(5.0)
        .set(ids.scrollbar, ui);

    let n_rows: usize = panel.sections.iter().map(|s| s.rows.len()).sum();
    if ids.header_txts.len() < panel.header.len() {
        ids.header_txts
            .resize(panel.header.len(), &mut ui.widget_id_generator());
    }
    if ids.section_btns.len() < panel.sections.len() {
        ids.section_btns
            .resize(panel.sections.len(), &mut ui.widget_id_generator());
    }
    if ids.row_txts.len() < n_rows {
        ids.row_txts
            .resize(n_rows, &mut ui.widget_id_generator());
    }

    // Every child sits under the previous one, left-aligned to it, so
    // the column never drifts; the first sits at the top of the scroll
    // area.
    let mut prev: Option<widget::Id> = None;
    let text_w = PANEL_W - 2.0 * PAD - 16.0;
    for (i, line) in panel.header.iter().enumerate() {
        let t = Text::new(line)
            .font_id(font)
            .font_size(11)
            .color(dim)
            .w(text_w)
            .wrap_by_word()
            .parent(ids.scroll_align);
        let t = match prev {
            Some(p) => t.down_from(p, 1.0).align_left_of(p),
            None => t.top_left_with_margins_on(ids.scroll_align, 2.0, 2.0),
        };
        t.set(ids.header_txts[i], ui);
        prev = Some(ids.header_txts[i]);
    }
    let mut r = 0;
    for (i, s) in panel.sections.iter().enumerate() {
        let label = format!(
            "{} {}",
            if s.expanded { "[-]" } else { "[+]" },
            s.title
        );
        let b = Button::new()
            .label(&label)
            .label_font_id(font)
            .label_font_size(12)
            .label_color(heading)
            .label_x(Relative::Place(Place::Start(Some(4.0))))
            .color(Color::Rgba(1.0, 1.0, 1.0, 0.06))
            .border(0.0)
            .w_h(text_w, 18.0)
            .parent(ids.scroll_align);
        let b = match prev {
            Some(p) => b.down_from(p, 5.0).align_left_of(p),
            None => b.top_left_with_margins_on(ids.scroll_align, 2.0, 2.0),
        };
        if b.set(ids.section_btns[i], ui).was_clicked() {
            toggles.push(s.id);
        }
        prev = Some(ids.section_btns[i]);
        for row in &s.rows {
            let p = prev.expect("a section heading precedes its rows");
            Text::new(&row.text)
                .font_id(font)
                .font_size(11)
                .color(if row.alarm { alarm } else { plain })
                .w(text_w)
                .wrap_by_word()
                .parent(ids.scroll_align)
                .down_from(p, 2.0)
                .align_left_of(p)
                .set(ids.row_txts[r], ui);
            prev = Some(ids.row_txts[r]);
            r += 1;
        }
    }
    toggles
}
