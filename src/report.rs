//! Small formatting helpers shared by tables and CSV files.

use crate::stats::Summary;

/// Quote a CSV field when it needs it.
pub fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn csv_row<S: AsRef<str>>(fields: &[S]) -> String {
    let mut row: String = fields
        .iter()
        .map(|f| csv_field(f.as_ref()))
        .collect::<Vec<_>>()
        .join(",");
    row.push('\n');
    row
}

/// A fixed-precision number for CSV files.
pub fn num(value: f64) -> String {
    format!("{value:.6}")
}

pub fn num_opt(value: Option<f64>) -> String {
    value.map(num).unwrap_or_default()
}

/// A rate as a percentage with a sensible number of digits.
pub fn pct(rate: f64) -> String {
    let scaled = rate * 100.0;
    if scaled == 0.0 {
        "0%".to_string()
    } else if scaled == 100.0 {
        "100%".to_string()
    } else if scaled.abs() < 1.0 {
        format!("{scaled:.2}%")
    } else {
        format!("{scaled:.1}%")
    }
}

pub fn pct_opt(rate: Option<f64>) -> String {
    rate.map(pct).unwrap_or_else(|| "n/a".to_string())
}

/// `k/n (rate)`, or `0/0 (n/a)`.
pub fn fraction(numerator: u64, denominator: u64) -> String {
    let rate = (denominator > 0).then(|| numerator as f64 / denominator as f64);
    format!("{numerator}/{denominator} ({})", pct_opt(rate))
}

/// `mean [low, high]` as percentages.
pub fn interval(summary: &Summary) -> String {
    format!(
        "{} [{}, {}]",
        pct(summary.mean),
        pct(summary.ci_low),
        pct(summary.ci_high)
    )
}

pub fn interval_opt(summary: Option<&Summary>) -> String {
    summary.map(interval).unwrap_or_else(|| "n/a".to_string())
}

/// A percentage-point difference with its interval.
pub fn points(summary: &Summary) -> String {
    let pp = |v: f64| format!("{:+.1}", v * 100.0);
    format!(
        "{} pp [{}, {}]",
        pp(summary.mean),
        pp(summary.ci_low),
        pp(summary.ci_high)
    )
}

/// A GitHub-flavoured markdown table.
pub fn md_table<S: AsRef<str>>(headers: &[S], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    let line = |cells: Vec<&str>| format!("| {} |\n", cells.join(" | "));
    out.push_str(&line(headers.iter().map(AsRef::as_ref).collect()));
    out.push_str(&line(headers.iter().map(|_| "---").collect()));
    for row in rows {
        out.push_str(&line(row.iter().map(String::as_str).collect()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_fields_are_quoted_only_when_needed() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_row(&["a", "b,c"]), "a,\"b,c\"\n");
    }

    #[test]
    fn percentages_keep_small_values_visible() {
        assert_eq!(pct(0.0), "0%");
        assert_eq!(pct(1.0), "100%");
        assert_eq!(pct(0.0087), "0.87%");
        assert_eq!(pct(0.183), "18.3%");
        assert_eq!(pct_opt(None), "n/a");
        assert_eq!(fraction(1, 4), "1/4 (25.0%)");
        assert_eq!(fraction(0, 0), "0/0 (n/a)");
    }

    #[test]
    fn intervals_and_point_differences_render() {
        let s = Summary {
            n: 40,
            mean: 0.25,
            sd: 0.1,
            min: 0.0,
            max: 0.5,
            ci_low: 0.2,
            ci_high: 0.3,
        };
        assert_eq!(interval(&s), "25.0% [20.0%, 30.0%]");
        assert_eq!(points(&s), "+25.0 pp [+20.0, +30.0]");
        assert_eq!(interval_opt(None), "n/a");
    }

    #[test]
    fn markdown_tables_have_a_separator_row() {
        let table = md_table(&["a", "b"], &[vec!["1".to_string(), "2".to_string()]]);
        assert_eq!(table, "| a | b |\n| --- | --- |\n| 1 | 2 |\n");
    }
}
