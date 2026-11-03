//! IT2: `rust/docs/screen-parity.md` has exactly one row per full path of the web app's router
//! (`FileRoutesByFullPath` in `flowtrace/src/routeTree.gen.ts`, read as text), and every row
//! says where the screen lives on the desktop: backticked [`NavEntry::name`]s that exist, an
//! `n/a: <reason>`, or a `gap: <task>` naming the task that owns it. The Web nav cell is the
//! rail item `isNavItemActive` lights for the path (`flowtrace/src/lib/navigation.ts:46-50`), or
//! `none`. The web files are read, never written.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::collections::BTreeSet;
use std::path::PathBuf;

use desktop::model::NavEntry;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(repo().join(path)).unwrap()
}

/// Every key of `export interface FileRoutesByFullPath { ... }`: lines `  '/path': typeof X`.
fn web_routes() -> Vec<String> {
    let text = read("flowtrace/src/routeTree.gen.ts");
    let body = text
        .split("export interface FileRoutesByFullPath {")
        .nth(1)
        .unwrap();
    let body = &body[..body.find('}').unwrap()];
    body.lines()
        .filter_map(|l| l.trim().strip_prefix('\'')?.split('\'').next())
        .map(str::to_owned)
        .collect()
}

/// `(label, href)` of every rail item in `navSections`, in order: lines
/// `{ label: 'Dashboard', href: '/dashboard', icon: House },`.
fn nav_items() -> Vec<(String, String)> {
    let text = read("flowtrace/src/lib/navigation.ts");
    text.lines()
        .filter_map(|l| {
            let label = l.split("label: '").nth(1)?.split('\'').next()?;
            let href = l.split("href: '").nth(1)?.split('\'').next()?;
            Some((label.to_owned(), href.to_owned()))
        })
        .collect()
}

/// The rail item lit on `path`, as `isNavItemActive` decides: the first item whose href equals
/// the path or, Dashboard aside, prefixes it.
fn active_nav(items: &[(String, String)], path: &str) -> Option<String> {
    items
        .iter()
        .find(|(_, href)| path == href || (href != "/dashboard" && path.starts_with(href.as_str())))
        .map(|(label, _)| label.clone())
}

/// The table's rows: the cells of every line whose first cell is a backticked web path.
fn table_rows() -> Vec<Vec<String>> {
    read("rust/docs/screen-parity.md")
        .lines()
        .filter_map(|l| {
            let cells: Vec<String> = l.split('|').map(|c| c.trim().to_owned()).collect();
            cells
                .get(1)?
                .starts_with("`/")
                .then(|| cells[1..cells.len() - 1].to_vec())
        })
        .collect()
}

/// The backticked words of a cell.
fn ticked(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

#[test]
fn the_screen_parity_table_covers_every_web_route() {
    let routes = web_routes();
    let rows = table_rows();
    let want: BTreeSet<_> = routes.iter().cloned().collect();
    let have: BTreeSet<_> = rows
        .iter()
        .map(|r| r[0].trim_matches('`').to_owned())
        .collect();
    assert_eq!(want.len(), routes.len(), "a web route is found twice");
    assert_eq!(have.len(), rows.len(), "a row appears twice");
    let missing: Vec<_> = want.difference(&have).collect();
    let unknown: Vec<_> = have.difference(&want).collect();
    assert!(
        missing.is_empty() && unknown.is_empty(),
        "rows missing: {missing:?}; rows naming no web route: {unknown:?}"
    );
    assert_eq!(
        rows.len(),
        39,
        "the row count in screen-parity.md and the report"
    );
}

#[test]
fn every_row_names_a_desktop_entry_a_reason_or_the_owning_task() {
    let names: BTreeSet<String> = NavEntry::ALL.into_iter().map(NavEntry::name).collect();
    let items = nav_items();
    assert!(!items.is_empty(), "navigation.ts gave no rail items");
    let mut used = BTreeSet::new();
    for row in table_rows() {
        assert_eq!(row.len(), 5, "five cells: {row:?}");
        let path = row[0].trim_matches('`');
        let nav = active_nav(&items, path).unwrap_or_else(|| "none".to_owned());
        assert_eq!(row[1], nav, "Web nav of {path}");
        let entry = &row[2];
        if let Some(reason) = entry.strip_prefix("n/a: ") {
            assert!(!reason.trim().is_empty(), "{path}: n/a without a reason");
        } else if let Some(task) = entry.strip_prefix("gap: ") {
            let id = task.split_whitespace().next().unwrap_or("");
            let well_formed = id.len() > 1
                && id.starts_with(|c: char| c.is_ascii_uppercase())
                && id.ends_with(|c: char| c.is_ascii_digit());
            assert!(well_formed, "{path}: a gap names its task, as `gap: DT4`");
        } else {
            let entries = ticked(entry);
            assert!(
                !entries.is_empty(),
                "{path}: the Desktop entry cell is empty"
            );
            for e in entries {
                assert!(names.contains(e), "{path}: no desktop entry named `{e}`");
                used.insert(e.to_owned());
            }
            assert!(!row[3].is_empty(), "{path}: Proved by is empty");
        }
    }
    let unmapped: Vec<_> = names.difference(&used).collect();
    assert!(
        unmapped.is_empty(),
        "desktop entries no web route maps to: {unmapped:?}"
    );
}
