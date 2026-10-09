//! Renders versions and packages as styled text. Output contains ANSI codes;
//! print it through `anstream`, which strips them when color is off.

use std::collections::BTreeSet;
use std::fmt::Write;

use anstyle::{AnsiColor, Style};

use crate::compare::{Diff, Found};
use crate::nvm::{Entry, GlobalPackage};

const HEADER: Style = Style::new().bold();
const MARK: Style = AnsiColor::Green.on_default().bold();
const VERSION: Style = AnsiColor::Cyan.on_default();
const DIM: Style = Style::new().dimmed();
const WARN: Style = AnsiColor::Yellow.on_default();
const MISSING: Style = AnsiColor::Red.on_default();

/// `text` styled and left-aligned in `width` columns (padding stays unstyled).
fn padded(style: Style, text: &str, width: usize) -> String {
    let pad = width.saturating_sub(text.chars().count());
    format!("{style}{text}{style:#}{:pad$}", "")
}

fn styled(style: Style, text: &str) -> String {
    format!("{style}{text}{style:#}")
}

fn version_cell(version: Option<&str>) -> (Style, &str) {
    match version {
        Some(v) => (VERSION, v),
        None => (MISSING, "?"),
    }
}

/// Each version followed by a tree of its packages.
pub fn tree(entries: &[Entry]) -> String {
    let mut out = String::new();
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out += &styled(HEADER, &entry.version.name);
        if let Some(label) = entry.label {
            out += &format!(" {}", styled(MARK, &format!("({label})")));
        }
        out.push('\n');

        let packages = &entry.packages;
        if packages.is_empty() {
            let _ = writeln!(
                out,
                "  {} {}",
                styled(DIM, "└─"),
                styled(DIM, "(no global packages)")
            );
            continue;
        }

        let name_width = packages.iter().map(|p| p.name.chars().count()).max();
        let version_width = packages
            .iter()
            .map(|p| version_cell(p.version.as_deref()).1.chars().count())
            .max();
        for (j, pkg) in packages.iter().enumerate() {
            let branch = if j + 1 == packages.len() {
                "└─"
            } else {
                "├─"
            };
            let (style, version) = version_cell(pkg.version.as_deref());
            let has_extra = !pkg.bins.is_empty() || pkg.linked;

            let _ = write!(
                out,
                "  {} {}  ",
                styled(DIM, branch),
                padded(Style::new(), &pkg.name, name_width.unwrap_or(0))
            );
            if has_extra {
                out += &padded(style, version, version_width.unwrap_or(0));
            } else {
                out += &styled(style, version);
            }
            if !pkg.bins.is_empty() {
                out += &format!("  {}", styled(DIM, &format!("→ {}", pkg.bins.join(", "))));
            }
            if pkg.linked {
                out += &format!("  {}", styled(WARN, "(linked)"));
            }
            out.push('\n');
        }
    }
    out
}

/// A matrix: one row per package, one column per Node version.
pub fn table(entries: &[Entry]) -> String {
    let names: BTreeSet<&str> = entries
        .iter()
        .flat_map(|e| e.packages.iter().map(|p| p.name.as_str()))
        .collect();
    if names.is_empty() {
        return styled(DIM, "(no global packages)\n");
    }

    // Plain-text header labels; the marked version gets a `*`.
    let headers: Vec<String> = entries
        .iter()
        .map(|e| match e.label {
            Some(_) => format!("{}*", e.version.name),
            None => e.version.name.clone(),
        })
        .collect();

    let cell = |entry: &Entry, name: &str| -> (Style, String) {
        match entry.packages.iter().find(|p| p.name == name) {
            Some(p) => {
                let (style, v) = version_cell(p.version.as_deref());
                (style, v.to_string())
            }
            None => (DIM, "-".to_string()),
        }
    };

    let first_width = names
        .iter()
        .map(|n| n.chars().count())
        .chain(["package".len()])
        .max()
        .unwrap_or(0);
    let widths: Vec<usize> = entries
        .iter()
        .zip(&headers)
        .map(|(entry, header)| {
            names
                .iter()
                .map(|n| cell(entry, n).1.chars().count())
                .chain([header.chars().count()])
                .max()
                .unwrap_or(0)
        })
        .collect();

    let mut out = String::new();
    let mut row = vec![padded(HEADER, "package", first_width)];
    for ((entry, header), &width) in entries.iter().zip(&headers).zip(&widths) {
        let style = if entry.label.is_some() { MARK } else { HEADER };
        row.push(padded(style, header, width));
    }
    out += row.join("  ").trim_end();
    out.push('\n');

    for name in &names {
        let mut row = vec![padded(Style::new(), name, first_width)];
        for (entry, &width) in entries.iter().zip(&widths) {
            let (style, text) = cell(entry, name);
            row.push(padded(style, &text, width));
        }
        out += row.join("  ").trim_end();
        out.push('\n');
    }

    if let Some(label) = entries.iter().find_map(|e| e.label) {
        out += &styled(DIM, &format!("* {label}\n"));
    }
    out
}

fn version_heading(entry: &Entry) -> String {
    let mut out = styled(HEADER, &entry.version.name);
    if let Some(label) = entry.label {
        out += &format!(" {}", styled(MARK, &format!("({label})")));
    }
    out
}

fn package_lines(out: &mut String, packages: &[&GlobalPackage]) {
    let width = packages.iter().map(|p| p.name.chars().count()).max();
    for pkg in packages {
        let (style, version) = version_cell(pkg.version.as_deref());
        let _ = write!(
            out,
            "  {}  {}",
            padded(Style::new(), &pkg.name, width.unwrap_or(0)),
            styled(style, version)
        );
        if pkg.linked {
            out.push_str(&format!("  {}", styled(WARN, "(linked)")));
        }
        out.push('\n');
    }
}

/// The differences between two versions, plus how to install what `to` lacks.
pub fn diff(from: &Entry, to: &Entry, d: &Diff, install_command: Option<&str>) -> String {
    let mut out = format!(
        "{} {} {}\n",
        version_heading(from),
        styled(DIM, "→"),
        version_heading(to)
    );

    if d.only_from.is_empty() && d.only_to.is_empty() && d.changed.is_empty() {
        out += &format!(
            "\nNo differences ({} identical package{}).\n",
            d.same,
            if d.same == 1 { "" } else { "s" }
        );
        return out;
    }

    for (packages, entry) in [(&d.only_from, from), (&d.only_to, to)] {
        if !packages.is_empty() {
            let _ = writeln!(out, "\nOnly in {}:", styled(HEADER, &entry.version.name));
            package_lines(&mut out, packages);
        }
    }

    if !d.changed.is_empty() {
        out += "\nDifferent versions:\n";
        let width = d.changed.iter().map(|(f, _)| f.name.chars().count()).max();
        for (f, t) in &d.changed {
            let (fs, fv) = version_cell(f.version.as_deref());
            let (ts, tv) = version_cell(t.version.as_deref());
            let _ = writeln!(
                out,
                "  {}  {} {} {}",
                padded(Style::new(), &f.name, width.unwrap_or(0)),
                styled(fs, fv),
                styled(DIM, "→"),
                styled(ts, tv)
            );
        }
    }

    if d.same > 0 {
        let _ = writeln!(
            out,
            "\n{}",
            styled(
                DIM,
                &format!(
                    "{} identical package{} not shown.",
                    d.same,
                    if d.same == 1 { "" } else { "s" }
                )
            )
        );
    }

    let linked: Vec<&str> = d
        .only_from
        .iter()
        .filter(|p| p.linked)
        .map(|p| p.name.as_str())
        .collect();
    if let Some(command) = install_command {
        let _ = writeln!(
            out,
            "\nTo install the missing packages in {}:",
            styled(HEADER, &to.version.name)
        );
        for line in command.lines() {
            let _ = writeln!(out, "  {}", styled(MARK, line));
        }
    }
    if !linked.is_empty() {
        let _ = writeln!(
            out,
            "{}",
            styled(
                WARN,
                &format!(
                    "Linked packages are not included, re-run `npm link` for: {}",
                    linked.join(", ")
                )
            )
        );
    }
    out
}

/// Where each found package is installed and where it is missing.
pub fn found(found: &[Found]) -> String {
    let mut out = String::new();
    for (i, f) in found.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out += &styled(HEADER, f.name);
        if !f.bins.is_empty() {
            out += &format!("  {}", styled(DIM, &format!("→ {}", f.bins.join(", "))));
        }
        out.push('\n');

        let headings: Vec<String> = f
            .installed
            .iter()
            .map(|(e, _)| version_heading(e))
            .collect();
        let width = f
            .installed
            .iter()
            .map(|(e, _)| {
                let label = e.label.map_or(0, |l| l.len() + 3);
                e.version.name.chars().count() + label
            })
            .max()
            .unwrap_or(0);
        for ((entry, pkg), heading) in f.installed.iter().zip(&headings) {
            let visible =
                entry.version.name.chars().count() + entry.label.map_or(0, |l| l.len() + 3);
            let (style, version) = version_cell(pkg.version.as_deref());
            let _ = write!(
                out,
                "  {heading}{:pad$}  {}",
                "",
                styled(style, version),
                pad = width - visible
            );
            if pkg.linked {
                out += &format!("  {}", styled(WARN, "(linked)"));
            }
            out.push('\n');
        }
        if !f.missing.is_empty() {
            let missing: Vec<String> = f.missing.iter().map(|e| version_heading(e)).collect();
            let _ = writeln!(out, "  {} {}", styled(DIM, "not in:"), missing.join(", "));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nvm::{GlobalPackage, NodeVersion};
    use std::path::PathBuf;

    fn pkg(name: &str, version: Option<&str>, bins: &[&str], linked: bool) -> GlobalPackage {
        GlobalPackage {
            name: name.to_string(),
            version: version.map(str::to_string),
            linked,
            bins: bins.iter().map(|b| b.to_string()).collect(),
        }
    }

    fn entry(name: &str, label: Option<&'static str>, packages: Vec<GlobalPackage>) -> Entry {
        Entry {
            version: NodeVersion {
                name: name.to_string(),
                parts: crate::nvm::parse_version(name).unwrap(),
                path: PathBuf::new(),
            },
            label,
            packages,
        }
    }

    fn sample() -> Vec<Entry> {
        vec![
            entry(
                "v26.1.0",
                None,
                vec![pkg(
                    "typescript",
                    Some("5.9.2"),
                    &["tsc", "tsserver"],
                    false,
                )],
            ),
            entry(
                "v24.2.0",
                Some("default"),
                vec![
                    pkg("@shopify/cli", Some("3.80.0"), &["shopify"], false),
                    pkg("mytool", None, &[], true),
                    pkg("yoinks", Some("0.3.1"), &[], false),
                ],
            ),
            entry("v18.0.0", None, vec![]),
        ]
    }

    fn plain(s: &str) -> String {
        anstream::adapter::strip_str(s).to_string()
    }

    #[test]
    fn renders_tree() {
        assert_eq!(
            plain(&tree(&sample())),
            "\
v26.1.0
  └─ typescript  5.9.2  → tsc, tsserver

v24.2.0 (default)
  ├─ @shopify/cli  3.80.0  → shopify
  ├─ mytool        ?       (linked)
  └─ yoinks        0.3.1

v18.0.0
  └─ (no global packages)
"
        );
    }

    #[test]
    fn renders_table() {
        assert_eq!(
            plain(&table(&sample())),
            "\
package       v26.1.0  v24.2.0*  v18.0.0
@shopify/cli  -        3.80.0    -
mytool        -        ?         -
typescript    5.9.2    -         -
yoinks        -        0.3.1     -
* default
"
        );
    }

    #[test]
    fn table_without_packages() {
        let entries = vec![entry("v18.0.0", None, vec![])];
        assert_eq!(plain(&table(&entries)), "(no global packages)\n");
    }

    #[test]
    fn renders_diff() {
        let entries = sample();
        let (to, from) = (&entries[0], &entries[1]);
        let d = crate::compare::diff(&from.packages, &to.packages);
        let out = diff(
            from,
            to,
            &d,
            Some("nvm exec 26.1.0 npm i -g @shopify/cli yoinks"),
        );
        assert_eq!(
            plain(&out),
            "\
v24.2.0 (default) → v26.1.0

Only in v24.2.0:
  @shopify/cli  3.80.0
  mytool        ?  (linked)
  yoinks        0.3.1

Only in v26.1.0:
  typescript  5.9.2

To install the missing packages in v26.1.0:
  nvm exec 26.1.0 npm i -g @shopify/cli yoinks
Linked packages are not included, re-run `npm link` for: mytool
"
        );
    }

    #[test]
    fn renders_identical_diff() {
        let entries = sample();
        let d = crate::compare::diff(&entries[1].packages, &entries[1].packages);
        let out = diff(&entries[1], &entries[1], &d, None);
        assert!(plain(&out).ends_with("No differences (3 identical packages).\n"));
    }

    #[test]
    fn renders_found() {
        let mut entries = sample();
        entries[1]
            .packages
            .push(pkg("typescript", Some("5.0.0"), &["tsc"], false));
        let out = found(&crate::compare::find(&entries, "tsc"));
        assert_eq!(
            plain(&out),
            "\
typescript  → tsc, tsserver
  v26.1.0            5.9.2
  v24.2.0 (default)  5.0.0
  not in: v18.0.0
"
        );
    }

    #[test]
    fn output_is_styled() {
        let out = tree(&sample());
        assert!(out.contains(&styled(MARK, "(default)")));
        assert!(out.contains(&styled(WARN, "(linked)")));
    }
}
