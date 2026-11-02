//! The TS transformer's failure assertions as one table (DESIGN §10.6, Q-F7): every build of
//! `packages/transformer/tests/*.test.ts` (monorepo 2.0.0-beta.0) that must fail with a message containing some text,
//! as data in `tests/diagnostics/cases.json`, built here on `tests/diagnostics/fixture`, the TS tests' own fixture
//! (its sources as build inputs; its `fieldInfoPlugin.cjs` is `support::fixture_macros`). The table was made from a
//! recorded run of the TS tests (each build's probe files, and each expectation on a failed build's output), so its
//! texts are TS5's.
//!
//! A case adds or replaces files of the fixture (`files`; `null` removes one), sets variables the fixture's
//! `flamework.config.json` reads (`env`, appended to its `.env`) and may build another tsconfig (`tsconfig`). The
//! build must fail, and its output must hold every `contains` text and none of the `excludes`, and a line holding the
//! parts of each `line` in order, and no line holding those of a `not_line`, and each `count` text (`[text, n]`)
//! exactly `n` times (where a TS test counts, as the size of its map of refused calls). `<ROOT>` in a text stands for
//! the project folder. The output is what `rbxtsc-fw` prints, without colours: the warnings, each diagnostic as
//! TypeScript's pretty reporter prints it (`src/a.ts:1:2 - error TS @flamework-experimental/transformer: ...`), then
//! the fatal errors.
//!
//! - `marked`: the message or behaviour differs from TS5's on purpose, for the reason given; the runner checks that it
//!   still differs, so a mark cannot outlive the difference.
//! - `covered`: the case needs what a single build cannot give (an incremental build's state, a TS transformer);
//!   the test named there covers it, and the runner skips it.
//!
//! The cases are built in parallel, each in a temporary copy of the fixture with a `node_modules` made as
//! `support::project` makes it. Without a monorepo the test is skipped, saying so. `FLAMEWORK_RS_DIAGNOSTICS=<text>`
//! runs only the cases whose name contains the text, and prints their output.

#![cfg(windows)]

mod support;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rbxts_plugin::erased::{DynPlugin, erase};
use rbxts_project::{ProjectOptionLayer, STACK_SIZE};
use rbxts_transform::{CompileOptions, CompileReport, PluginOptions, compile_project_with};
use serde_json::Value;
use support::fixture_macros::flamework_with_fixture_macros;
use support::project::{Project, fixture_dir};
use tsrs_diagnostics::Category;

const FIXTURE: &str = "diagnostics/fixture";

struct Case {
    name: String,
    files: Vec<(String, Option<String>)>,
    env: Vec<(String, String)>,
    tsconfig: Option<String>,
    contains: Vec<String>,
    excludes: Vec<String>,
    lines: Vec<Vec<String>>,
    not_lines: Vec<Vec<String>>,
    counts: Vec<(String, usize)>,
    marked: Option<String>,
    covered: Option<String>,
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_else(|| panic!("expected a string, got {value}"))
}

fn strings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items.iter().map(|v| text(v).to_owned()).collect(),
        None => Vec::new(),
        Some(other) => panic!("expected an array of strings, got {other}"),
    }
}

fn string_lists(value: Option<&Value>) -> Vec<Vec<String>> {
    match value {
        Some(Value::Array(lists)) => lists.iter().map(|l| strings(Some(l))).collect(),
        None => Vec::new(),
        Some(other) => panic!("expected an array of arrays, got {other}"),
    }
}

fn counts(value: Option<&Value>) -> Vec<(String, usize)> {
    match value {
        Some(Value::Array(pairs)) => pairs
            .iter()
            .map(|pair| match pair.as_array().map(Vec::as_slice) {
                Some([text_value, Value::Number(n)]) => {
                    (text(text_value).to_owned(), n.as_u64().expect("a count") as usize)
                }
                _ => panic!("expected [text, count], got {pair}"),
            })
            .collect(),
        None => Vec::new(),
        Some(other) => panic!("expected an array of [text, count], got {other}"),
    }
}

fn cases() -> Vec<Case> {
    let path = fixture_dir("diagnostics").join("cases.json");
    let source = std::fs::read_to_string(&path).unwrap();
    let data: Value = serde_json::from_str(&source).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let items = data["cases"].as_array().expect("cases");
    items
        .iter()
        .map(|item| {
            let pairs = |key: &str| -> Vec<(String, Option<String>)> {
                match item.get(key) {
                    Some(Value::Object(entries)) => entries
                        .iter()
                        .map(|(k, v)| (k.clone(), if v.is_null() { None } else { Some(text(v).to_owned()) }))
                        .collect(),
                    None => Vec::new(),
                    Some(other) => panic!("{key}: expected an object, got {other}"),
                }
            };
            Case {
                name: text(&item["name"]).to_owned(),
                files: pairs("files"),
                env: pairs("env").into_iter().map(|(k, v)| (k, v.unwrap_or_default())).collect(),
                tsconfig: item.get("tsconfig").map(|v| text(v).to_owned()),
                contains: strings(item.get("contains")),
                excludes: strings(item.get("excludes")),
                lines: string_lists(item.get("line")),
                not_lines: string_lists(item.get("not_line")),
                counts: counts(item.get("count")),
                marked: item.get("marked").map(|v| text(v).to_owned()),
                covered: item.get("covered").map(|v| text(v).to_owned()),
            }
        })
        .collect()
}

/// What `rbxtsc-fw` prints for the build, without colours.
fn output(report: &CompileReport, cwd: &str) -> String {
    let mut out = String::new();
    for warning in &report.warnings {
        out.push_str(warning);
        out.push('\n');
    }
    let mut diagnostics = report.diagnostics.clone();
    for file in &report.files {
        diagnostics.extend(file.diagnostics.iter().copied());
    }
    for d in diagnostics {
        out.push_str(&rbxts_cli::format_diagnostic(d, cwd));
        out.push('\n');
    }
    for fatal in &report.fatal {
        out.push_str(fatal);
        out.push('\n');
    }
    strip_colours(&out).replace("\r\n", "\n")
}

fn strip_colours(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn failed(report: &CompileReport) -> bool {
    !report.fatal.is_empty()
        || report
            .diagnostics
            .iter()
            .chain(report.files.iter().flat_map(|f| f.diagnostics.iter()))
            .any(|d| d.category() == Category::Error)
}

/// Whether a line of `printed` holds `parts` in order.
fn has_line(printed: &str, parts: &[String]) -> bool {
    printed.lines().any(|line| {
        let mut rest = line;
        parts.iter().all(|part| match rest.find(part.as_str()) {
            Some(at) => {
                rest = &rest[at + part.len()..];
                true
            }
            None => false,
        })
    })
}

/// Builds one case: its output, and what in it is not as the case expects.
fn run(case: &Case, index: usize) -> (String, Vec<String>) {
    let project = Project::new(FIXTURE, &format!("c{index}")).expect("checked by the caller");
    for (name, contents) in &case.files {
        match contents {
            Some(contents) => project.write(name, contents),
            None => std::fs::remove_file(project.root.join(name)).unwrap_or_else(|e| panic!("{name}: {e}")),
        }
    }
    if !case.env.is_empty() {
        let mut env = std::fs::read_to_string(project.root.join(".env")).unwrap_or_default();
        for (name, value) in &case.env {
            env.push_str(&format!("\n{name}={value}\n"));
        }
        project.write(".env", &env);
    }
    let cwd = project.root.to_string_lossy().into_owned();
    let tsconfig = case.tsconfig.clone().unwrap_or_else(|| "tsconfig.json".into());
    let report = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, || {
                let plugins: Vec<Box<dyn DynPlugin>> = vec![erase(flamework_with_fixture_macros())];
                compile_project_with(
                    &CompileOptions {
                        tsconfig,
                        cwd: cwd.clone(),
                        project_options: ProjectOptionLayer {
                            optimized_loops: Some(true),
                            allow_comment_directives: Some(true),
                            ..ProjectOptionLayer::default()
                        },
                        out_dir_override: None,
                        write_partial: false,
                        no_write: false,
                        checkers: None,
                    },
                    &PluginOptions { plugins: &plugins, ..PluginOptions::default() },
                )
            })
            .unwrap()
            .join()
            .unwrap()
    });
    let printed = output(&report, &cwd);
    let root = |text: &str| text.replace("<ROOT>", &cwd.replace('/', "\\"));
    let mut wrong = Vec::new();
    if !failed(&report) {
        wrong.push("the build did not fail".to_owned());
    }
    for text in &case.contains {
        if !printed.contains(&root(text)) {
            wrong.push(format!("no {text:?}"));
        }
    }
    for text in &case.excludes {
        if printed.contains(&root(text)) {
            wrong.push(format!("has {text:?}"));
        }
    }
    for parts in &case.lines {
        if !has_line(&printed, parts) {
            wrong.push(format!("no line with {parts:?}"));
        }
    }
    for parts in &case.not_lines {
        if has_line(&printed, parts) {
            wrong.push(format!("a line with {parts:?}"));
        }
    }
    for (text, expected) in &case.counts {
        let found = printed.matches(root(text).as_str()).count();
        if found != *expected {
            wrong.push(format!("{text:?} {found} times, not {expected}"));
        }
    }
    (printed, wrong)
}

#[test]
fn transformer_test_failures() {
    if Project::new(FIXTURE, "probe").is_none() {
        return;
    }
    let filter = std::env::var("FLAMEWORK_RS_DIAGNOSTICS").ok();
    let cases: Vec<Case> = cases()
        .into_iter()
        .filter(|c| c.covered.is_none() && filter.as_deref().is_none_or(|f| c.name.contains(f)))
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 8);
    let next = AtomicUsize::new(0);
    let results: Mutex<BTreeMap<usize, (String, Vec<String>)>> = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(case) = cases.get(index) else { break };
                    let result = run(case, index);
                    results.lock().unwrap().insert(index, result);
                }
            });
        }
    });
    let mut failures = Vec::new();
    for (index, (printed, wrong)) in results.into_inner().unwrap() {
        let case = &cases[index];
        if filter.is_some() {
            eprintln!("=== {}\n{}\n--- output\n{printed}", case.name, wrong.join("\n"));
        }
        match &case.marked {
            None if !wrong.is_empty() => {
                failures.push(format!("{}:\n{}\n--- output\n{printed}", case.name, wrong.join("\n")))
            }
            Some(reason) if wrong.is_empty() => {
                failures.push(format!("{}: marked ({reason}), but fails as TS5 does now: remove the mark", case.name))
            }
            _ => {}
        }
    }
    assert!(failures.is_empty(), "{} of {} cases:\n\n{}", failures.len(), cases.len(), failures.join("\n\n"));
}
