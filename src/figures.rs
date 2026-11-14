//! Dependency-free SVG figures.
//!
//! Three forms are enough for this report: a labelled heat table (a rate
//! per adapter and scenario family), horizontal bars with an interval (one
//! rate per adapter), and a labelled scatter (two rates per adapter). Every
//! figure uses one hue, carries its numbers as text, and restyles itself
//! for a dark colour scheme.

use std::fmt::Write;

const FONT: &str = "system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif";

/// Light and dark values for every colour the figures use. The six `q`
/// steps are one sequential blue ramp, lowest to highest.
const STYLE: &str = "\
svg{--surface:#fcfcfb;--ink:#1f1f1e;--ink2:#5c5c59;--grid:#e4e4e0;--mark:#2a78d6;\
--q0:#eef4fc;--q1:#cfe1f7;--q2:#a3c6ef;--q3:#6fa4e3;--q4:#3f82d3;--q5:#1f5fae;\
--on-low:#1f1f1e;--on-high:#ffffff}\
@media (prefers-color-scheme:dark){svg{--surface:#1a1a19;--ink:#f1f1ee;--ink2:#a8a8a3;\
--grid:#34342f;--mark:#3987e5;\
--q0:#1f2a38;--q1:#24405f;--q2:#295a8c;--q3:#2f74b8;--q4:#4a90dc;--q5:#86b8ee;\
--on-low:#f1f1ee;--on-high:#10100f}}\
.bg{fill:var(--surface)}.title{fill:var(--ink);font-size:15px;font-weight:600}\
.sub{fill:var(--ink2);font-size:12px}.lab{fill:var(--ink);font-size:12px}\
.tick{fill:var(--ink2);font-size:11px}.note{fill:var(--ink2);font-size:11px}\
.grid{stroke:var(--grid);stroke-width:1}.axis{stroke:var(--ink2);stroke-width:1}\
.mark{fill:var(--mark)}.whisker{stroke:var(--ink);stroke-width:1.5}\
.ring{stroke:var(--surface);stroke-width:2}.na{fill:var(--grid)}\
.q0{fill:var(--q0)}.q1{fill:var(--q1)}.q2{fill:var(--q2)}\
.q3{fill:var(--q3)}.q4{fill:var(--q4)}.q5{fill:var(--q5)}\
.on-low{fill:var(--on-low);font-size:12px}.on-high{fill:var(--on-high);font-size:12px}";

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Break `text` into lines of at most `width` characters at spaces.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Text shared by every figure.
#[derive(Debug, Clone)]
pub struct Caption {
    pub title: String,
    pub subtitle: String,
    /// Denominators and caveats, shown beneath the plot.
    pub note: String,
}

struct Canvas {
    body: String,
    width: f64,
    /// Y coordinate where the plot area may start.
    top: f64,
}

const WIDTH: f64 = 780.0;
const MARGIN: f64 = 20.0;
const WRAP: usize = 118;

impl Canvas {
    fn new(caption: &Caption) -> Self {
        let mut body = String::new();
        let mut y = 30.0;
        let _ = writeln!(
            body,
            r#"<text class="title" x="{MARGIN}" y="{y}">{}</text>"#,
            escape(&caption.title)
        );
        for line in wrap(&caption.subtitle, WRAP) {
            y += 17.0;
            let _ = writeln!(
                body,
                r#"<text class="sub" x="{MARGIN}" y="{y}">{}</text>"#,
                escape(&line)
            );
        }
        Self {
            body,
            width: WIDTH,
            top: y + 22.0,
        }
    }

    fn push(&mut self, element: std::fmt::Arguments<'_>) {
        let _ = self.body.write_fmt(element);
        self.body.push('\n');
    }

    fn finish(mut self, caption: &Caption, plot_bottom: f64) -> String {
        let mut y = plot_bottom + 24.0;
        for line in wrap(&caption.note, WRAP) {
            let (x, text) = (MARGIN, escape(&line));
            self.push(format_args!(
                r#"<text class="note" x="{x}" y="{y}">{text}</text>"#
            ));
            y += 15.0;
        }
        let height = y + 4.0;
        let width = self.width;
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
             width=\"{width}\" height=\"{height}\" role=\"img\" font-family=\"{FONT}\">\n\
             <title>{}</title>\n<desc>{}</desc>\n<style>{STYLE}</style>\n\
             <rect class=\"bg\" width=\"{width}\" height=\"{height}\"/>\n{}</svg>\n",
            escape(&caption.title),
            escape(&caption.subtitle),
            self.body
        )
    }
}

/// One cell of a heat table.
#[derive(Debug, Clone)]
pub struct Cell {
    /// A rate in 0..=1, or `None` when the denominator was zero.
    pub value: Option<f64>,
    /// Text drawn in the cell.
    pub text: String,
    /// Hover text: numerator, denominator, interval.
    pub detail: String,
}

/// Which of the six ramp steps a rate falls in. Exactly zero has its own
/// step so that "none" is visually distinct from "a few".
fn step(value: f64) -> usize {
    if value <= 0.0 {
        0
    } else {
        (1 + (value * 5.0).floor() as usize).min(5)
    }
}

/// A rate per row and column, drawn as a table of shaded, labelled cells.
pub fn heat_table(caption: &Caption, columns: &[String], rows: &[(String, Vec<Cell>)]) -> String {
    let mut canvas = Canvas::new(caption);
    let label_width = 196.0;
    let cell_width = (canvas.width - 2.0 * MARGIN - label_width) / columns.len() as f64;
    let cell_height = 26.0;
    let header_y = canvas.top + 4.0;
    for (c, column) in columns.iter().enumerate() {
        let x = MARGIN + label_width + cell_width * (c as f64 + 0.5);
        canvas.push(format_args!(
            r#"<text class="lab" x="{x:.1}" y="{header_y}" text-anchor="middle">{}</text>"#,
            escape(column)
        ));
    }
    let grid_top = header_y + 10.0;
    for (r, (label, cells)) in rows.iter().enumerate() {
        let y = grid_top + r as f64 * (cell_height + 2.0);
        let text_y = y + cell_height / 2.0 + 4.0;
        canvas.push(format_args!(
            r#"<text class="lab" x="{:.1}" y="{text_y:.1}" text-anchor="end">{}</text>"#,
            MARGIN + label_width - 10.0,
            escape(label)
        ));
        for (c, cell) in cells.iter().enumerate() {
            let x = MARGIN + label_width + cell_width * c as f64;
            let (fill, ink) = match cell.value {
                None => ("na".to_string(), "on-low"),
                Some(v) => {
                    let s = step(v);
                    (format!("q{s}"), if s >= 4 { "on-high" } else { "on-low" })
                }
            };
            canvas.push(format_args!(
                r#"<g><title>{}</title><rect class="{fill}" x="{:.1}" y="{y:.1}" width="{:.1}" height="{cell_height}" rx="3"/><text class="{ink}" x="{:.1}" y="{text_y:.1}" text-anchor="middle">{}</text></g>"#,
                escape(&cell.detail),
                x + 1.0,
                cell_width - 2.0,
                x + cell_width / 2.0,
                escape(&cell.text)
            ));
        }
    }
    let grid_bottom = grid_top + rows.len() as f64 * (cell_height + 2.0);

    // Ramp key.
    let key_y = grid_bottom + 12.0;
    let labels = ["0%", ">0-20%", "20-40%", "40-60%", "60-80%", "80-100%"];
    for (s, label) in labels.iter().enumerate() {
        let x = MARGIN + label_width + s as f64 * 92.0;
        canvas.push(format_args!(
            r#"<rect class="q{s}" x="{x:.1}" y="{key_y}" width="14" height="14" rx="3"/><text class="tick" x="{:.1}" y="{:.1}">{label}</text>"#,
            x + 19.0,
            key_y + 11.0
        ));
    }
    canvas.finish(caption, key_y + 14.0)
}

/// One bar with an optional interval.
#[derive(Debug, Clone)]
pub struct Bar {
    pub label: String,
    pub value: Option<f64>,
    pub interval: Option<(f64, f64)>,
    /// Text drawn after the bar.
    pub text: String,
    pub detail: String,
}

/// A "nice" upper bound for a rate axis, with the tick step that divides
/// it into round percentages.
fn axis(largest: f64) -> (f64, f64) {
    [
        (0.05, 0.01),
        (0.10, 0.02),
        (0.20, 0.05),
        (0.40, 0.10),
        (0.50, 0.10),
        (1.0, 0.25),
    ]
    .into_iter()
    .find(|(bound, _)| largest <= *bound)
    .unwrap_or((1.0, 0.25))
}

fn percent_tick(value: f64) -> String {
    let scaled = value * 100.0;
    if (scaled - scaled.round()).abs() < 1e-9 {
        format!("{scaled:.0}%")
    } else {
        format!("{scaled:.2}%")
    }
}

/// One rate per row, as horizontal bars from a zero baseline.
pub fn bars(caption: &Caption, axis_label: &str, rows: &[Bar]) -> String {
    let mut canvas = Canvas::new(caption);
    let label_width = 196.0;
    let text_room = 150.0;
    let left = MARGIN + label_width;
    let plot_width = canvas.width - left - MARGIN - text_room;
    let row_height = 34.0;
    let thickness = 18.0;
    let largest = rows
        .iter()
        .flat_map(|b| [b.value, b.interval.map(|i| i.1)])
        .flatten()
        .fold(0.0, f64::max);
    let (max, tick) = axis(largest);
    let ticks = (max / tick).round() as u32;
    let x_of = |v: f64| left + plot_width * (v / max).clamp(0.0, 1.0);
    let top = canvas.top;
    let bottom = top + rows.len() as f64 * row_height;

    for i in 0..=ticks {
        let v = tick * f64::from(i);
        let x = x_of(v);
        canvas.push(format_args!(
            r#"<line class="grid" x1="{x:.1}" y1="{top:.1}" x2="{x:.1}" y2="{bottom:.1}"/><text class="tick" x="{x:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
            bottom + 15.0,
            percent_tick(v)
        ));
    }
    canvas.push(format_args!(
        r#"<line class="axis" x1="{left:.1}" y1="{top:.1}" x2="{left:.1}" y2="{bottom:.1}"/>"#
    ));
    for (r, bar) in rows.iter().enumerate() {
        let y = top + r as f64 * row_height + (row_height - thickness) / 2.0;
        let mid = y + thickness / 2.0;
        canvas.push(format_args!(
            r#"<text class="lab" x="{:.1}" y="{:.1}" text-anchor="end">{}</text>"#,
            left - 10.0,
            mid + 4.0,
            escape(&bar.label)
        ));
        let mut text_x = left + 8.0;
        if let Some(value) = bar.value {
            let end = x_of(value);
            let length = end - left;
            if length > 0.5 {
                // Square at the baseline, rounded at the data end.
                let r = 4.0_f64.min(length);
                canvas.push(format_args!(
                    r#"<path class="mark" d="M {left:.1} {y:.1} H {:.1} Q {end:.1} {y:.1} {end:.1} {:.1} V {:.1} Q {end:.1} {:.1} {:.1} {:.1} H {left:.1} Z"><title>{}</title></path>"#,
                    end - r,
                    y + r,
                    y + thickness - r,
                    y + thickness,
                    end - r,
                    y + thickness,
                    escape(&bar.detail)
                ));
            }
            text_x = end + 8.0;
            if let Some((low, high)) = bar.interval {
                let (x1, x2) = (x_of(low), x_of(high));
                if x2 - x1 > 0.5 {
                    canvas.push(format_args!(
                        r#"<line class="whisker" x1="{x1:.1}" y1="{mid:.1}" x2="{x2:.1}" y2="{mid:.1}"/><line class="whisker" x1="{x1:.1}" y1="{:.1}" x2="{x1:.1}" y2="{:.1}"/><line class="whisker" x1="{x2:.1}" y1="{:.1}" x2="{x2:.1}" y2="{:.1}"/>"#,
                        mid - 4.0,
                        mid + 4.0,
                        mid - 4.0,
                        mid + 4.0
                    ));
                    text_x = x2 + 8.0;
                }
            }
        }
        canvas.push(format_args!(
            r#"<text class="lab" x="{text_x:.1}" y="{:.1}">{}</text>"#,
            mid + 4.0,
            escape(&bar.text)
        ));
    }
    canvas.push(format_args!(
        r#"<text class="tick" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
        left + plot_width / 2.0,
        bottom + 32.0,
        escape(axis_label)
    ));
    canvas.finish(caption, bottom + 34.0)
}

/// One labelled point with an interval on each axis.
#[derive(Debug, Clone)]
pub struct Point {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub x_interval: (f64, f64),
    pub y_interval: (f64, f64),
    pub detail: String,
}

/// Two rates per adapter on shared 0-100% axes, each point labelled.
pub fn scatter(caption: &Caption, x_label: &str, y_label: &str, points: &[Point]) -> String {
    let mut canvas = Canvas::new(caption);
    let left = MARGIN + 64.0;
    let top = canvas.top + 6.0;
    let size = 360.0;
    let bottom = top + size;
    let x_of = |v: f64| left + size * v.clamp(0.0, 1.0);
    let y_of = |v: f64| bottom - size * v.clamp(0.0, 1.0);

    for i in 0..=4 {
        let v = f64::from(i) / 4.0;
        let (x, y) = (x_of(v), y_of(v));
        canvas.push(format_args!(
            r#"<line class="grid" x1="{x:.1}" y1="{top:.1}" x2="{x:.1}" y2="{bottom:.1}"/><line class="grid" x1="{left:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}"/><text class="tick" x="{x:.1}" y="{:.1}" text-anchor="middle">{}</text><text class="tick" x="{:.1}" y="{:.1}" text-anchor="end">{}</text>"#,
            left + size,
            bottom + 16.0,
            percent_tick(v),
            left - 8.0,
            y + 4.0,
            percent_tick(v)
        ));
    }
    canvas.push(format_args!(
        r#"<text class="tick" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
        left + size / 2.0,
        bottom + 36.0,
        escape(x_label)
    ));
    canvas.push(format_args!(
        r#"<text class="tick" transform="translate({:.1} {:.1}) rotate(-90)" text-anchor="middle">{}</text>"#,
        MARGIN + 6.0,
        top + size / 2.0,
        escape(y_label)
    ));

    // Labels sit in a column to the right, joined to their points by order,
    // so that points sharing a position do not hide each other's names.
    let key_x = left + size + 34.0;
    for (i, point) in points.iter().enumerate() {
        let (x, y) = (x_of(point.x), y_of(point.y));
        let (x1, x2) = (x_of(point.x_interval.0), x_of(point.x_interval.1));
        let (y1, y2) = (y_of(point.y_interval.0), y_of(point.y_interval.1));
        canvas.push(format_args!(
            r#"<line class="whisker" x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}"/><line class="whisker" x1="{x:.1}" y1="{y1:.1}" x2="{x:.1}" y2="{y2:.1}"/>"#
        ));
        canvas.push(format_args!(
            r#"<circle class="mark ring" cx="{x:.1}" cy="{y:.1}" r="7"><title>{}</title></circle><text class="on-high" x="{x:.1}" y="{:.1}" text-anchor="middle" font-weight="600" style="font-size:10px">{}</text>"#,
            escape(&point.detail),
            y + 3.5,
            i + 1
        ));
        let key_y = top + 14.0 + i as f64 * 44.0;
        canvas.push(format_args!(
            r#"<circle class="mark" cx="{:.1}" cy="{:.1}" r="7"/><text class="on-high" x="{:.1}" y="{:.1}" text-anchor="middle" font-weight="600" style="font-size:10px">{}</text><text class="lab" x="{:.1}" y="{:.1}">{}</text><text class="tick" x="{:.1}" y="{:.1}">{}</text>"#,
            key_x,
            key_y - 4.0,
            key_x,
            key_y - 0.5,
            i + 1,
            key_x + 14.0,
            key_y,
            escape(&point.label),
            key_x + 14.0,
            key_y + 15.0,
            escape(&point.detail)
        ));
    }
    canvas.finish(caption, bottom + 38.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caption() -> Caption {
        Caption {
            title: "A <title> & more".to_string(),
            subtitle: "subtitle ".repeat(40),
            note: "numerator / denominator".to_string(),
        }
    }

    fn well_formed(svg: &str) {
        assert!(svg.starts_with("<svg xmlns"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("prefers-color-scheme:dark"));
        assert!(svg.contains("A &lt;title&gt; &amp; more"));
        assert!(!svg.contains("NaN") && !svg.contains("inf"));
        assert_eq!(svg.matches("<g>").count(), svg.matches("</g>").count());
    }

    #[test]
    fn wrap_breaks_at_spaces_within_the_width() {
        let lines = wrap("aaa bbb ccc ddd", 7);
        assert_eq!(lines, ["aaa bbb", "ccc ddd"]);
        assert!(wrap("", 10).is_empty());
        assert_eq!(wrap("unbreakable", 3), ["unbreakable"]);
    }

    #[test]
    fn zero_has_its_own_ramp_step() {
        assert_eq!(step(0.0), 0);
        assert_eq!(step(0.001), 1);
        assert_eq!(step(0.2), 2);
        assert_eq!(step(0.999), 5);
        assert_eq!(step(1.0), 5);
    }

    #[test]
    fn heat_table_draws_every_cell_and_marks_missing_ones() {
        let cell = |value: Option<f64>| Cell {
            value,
            text: value.map_or("n/a".to_string(), |v| format!("{:.0}%", v * 100.0)),
            detail: "3/4".to_string(),
        };
        let rows = vec![
            (
                "family-a".to_string(),
                vec![cell(Some(0.0)), cell(Some(1.0))],
            ),
            ("family-b".to_string(), vec![cell(None), cell(Some(0.5))]),
        ];
        let svg = heat_table(&caption(), &["x".to_string(), "y".to_string()], &rows);
        well_formed(&svg);
        assert_eq!(svg.matches("<g>").count(), 4);
        assert!(svg.contains(r#"class="na""#));
        assert!(svg.contains(r#"class="q0""#) && svg.contains(r#"class="q5""#));
        assert!(svg.contains("family-b") && svg.contains("n/a"));
    }

    #[test]
    fn bars_scale_to_a_nice_axis_and_skip_missing_values() {
        let rows = vec![
            Bar {
                label: "a".into(),
                value: Some(0.12),
                interval: Some((0.10, 0.14)),
                text: "12%".into(),
                detail: "d".into(),
            },
            Bar {
                label: "b".into(),
                value: Some(0.0),
                interval: Some((0.0, 0.0)),
                text: "0%".into(),
                detail: "d".into(),
            },
            Bar {
                label: "c".into(),
                value: None,
                interval: None,
                text: "n/a".into(),
                detail: "d".into(),
            },
        ];
        let svg = bars(&caption(), "rate", &rows);
        well_formed(&svg);
        assert_eq!(svg.matches("<path").count(), 1);
        assert!(svg.contains(">20%<") && !svg.contains(">100%<"));
        assert_eq!(axis(0.03), (0.05, 0.01));
        assert_eq!(axis(0.7), (1.0, 0.25));
        // Every axis divides into whole-percent ticks.
        for largest in [0.01, 0.07, 0.15, 0.33, 0.45, 0.9] {
            let (max, tick) = axis(largest);
            let steps = max / tick;
            assert!((steps - steps.round()).abs() < 1e-9, "{largest}");
            assert!(((tick * 100.0) - (tick * 100.0).round()).abs() < 1e-9);
        }
    }

    #[test]
    fn scatter_numbers_every_point_and_lists_it_in_the_key() {
        let points: Vec<Point> = (0..4)
            .map(|i| Point {
                label: format!("adapter-{i}"),
                x: 0.25 * f64::from(i),
                y: 1.0 - 0.25 * f64::from(i),
                x_interval: (0.0, 1.0),
                y_interval: (0.0, 1.0),
                detail: "x, y".into(),
            })
            .collect();
        let svg = scatter(&caption(), "x axis", "y axis", &points);
        well_formed(&svg);
        assert_eq!(svg.matches("<circle").count(), 8);
        assert!(svg.contains("adapter-3") && svg.contains("rotate(-90)"));
    }
}
