//! Components (F6) end to end, through the real compiler on `tests/fixture_f6` and `tests/component_errors`.
//!
//! `tests/fixture_f6/expected` is rbxts-rs's full-mode build, checked against the TS5 pipeline (roblox-ts master with
//! the 2.0.0-beta.0 TS transformer) on the same sources. `shared/components.luau` is the monorepo transformer
//! fixture's `components.ts`, `shared/recursiveLink.luau` its `RecursiveLink`. Every file and `flamework.build` equal
//! TS5's but for:
//! - the order of union members, which follows TS7 (DESIGN §10.4): `isA = { "Decal", "Texture" }` in an instance
//!   shape, the members of `t.union` / `t.literal` in guards (`components.luau`, `config.luau`).
//!
//! The project is built in a temporary copy with a `node_modules` made as `e2e.rs` makes it. Without a monorepo the
//! tests are skipped, saying so.

#![cfg(windows)]

mod support;

use std::path::Path;

use flamework_rs::Flamework;
use rbxts_plugin::erased::{DynPlugin, erase};
use rbxts_plugin::ts::{Diagnostic, P};
use rbxts_project::{ProjectOptionLayer, STACK_SIZE};
use rbxts_transform::{CompileOptions, CompileReport, PluginOptions, compile_project_with};
use support::project::{Project, assert_clean, compare_expected, fixture_dir, luau};
use rbxts_project::flatten_diagnostic_message;
use tsrs_diagnostics::Category;

const FIXTURE: &str = "fixture_f6";

/// Builds with this many checkers (`None`: the default).
fn build(project: &Project, checkers: Option<usize>) -> CompileReport {
    let plugins: Vec<Box<dyn DynPlugin>> = vec![erase(Flamework::new())];
    let cwd = project.root.to_string_lossy().into_owned();
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, move || {
                compile_project_with(
                    &CompileOptions {
                        tsconfig: "tsconfig.json".into(),
                        cwd,
                        project_options: ProjectOptionLayer {
                            optimized_loops: Some(true),
                            allow_comment_directives: Some(true),
                            ..ProjectOptionLayer::default()
                        },
                        out_dir_override: None,
                        write_partial: false,
                        no_write: false,
                        checkers,
                    },
                    &PluginOptions { plugins: &plugins, ..PluginOptions::default() },
                )
            })
            .unwrap()
            .join()
            .unwrap()
    })
}

/// `UPDATE_EXPECTED=1`: writes the build as `tests/<fixture>/expected` instead of comparing.
fn update_expected(project: &Project, report: &CompileReport, fixture: &str) -> bool {
    if std::env::var_os("UPDATE_EXPECTED").is_none() {
        return false;
    }
    let expected = fixture_dir(fixture).join("expected");
    let _ = std::fs::remove_dir_all(&expected);
    for file in &report.files {
        let relative = Path::new(&file.out_path).strip_prefix(&report.out_dir).unwrap().to_path_buf();
        let path = expected.join("out").join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, file.luau.as_deref().unwrap()).unwrap();
    }
    let artifacts = [
        "flamework.build",
        "include/flamework/globs.json",
        "include/flamework/paths.json",
        "include/flamework/config.json",
    ];
    for artifact in artifacts {
        if project.exists(artifact) {
            let path = expected.join(artifact);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, project.read(artifact)).unwrap();
        }
    }
    true
}

/// Component configs (links, attribute guards, instance shapes and guards), attribute setters, key obfuscation and
/// the component ids in `flamework.build`: the expected outputs.
#[test]
fn fixture_matches_the_expected_outputs() {
    let Some(project) = Project::new(FIXTURE, "full") else { return };
    let report = build(&project, None);
    assert_clean(&report);
    if update_expected(&project, &report, FIXTURE) {
        return;
    }
    let failures = compare_expected(&project, &report, FIXTURE);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // No macro call is left to a later milestone.
    let messages: Vec<String> =
        support::project::diagnostics(&report).into_iter().chain(report.warnings.clone()).collect();
    assert!(messages.iter().all(|m| !m.contains("need features")), "{messages:?}");
}

/// One checker and four give the same bytes: outputs and `flamework.build`.
#[test]
fn checker_counts_give_the_same_bytes() {
    let Some(one) = Project::new(FIXTURE, "checkers1") else { return };
    let Some(four) = Project::new(FIXTURE, "checkers4") else { return };
    let a = build(&one, Some(1));
    let b = build(&four, Some(4));
    assert_clean(&a);
    assert_clean(&b);
    assert_eq!(a.files.len(), b.files.len());
    for file in &a.files {
        let relative = Path::new(&file.out_path).strip_prefix(&a.out_dir).unwrap().to_string_lossy().replace('\\', "/");
        assert_eq!(file.luau.as_deref(), Some(luau(&b, &relative)), "{relative}");
    }
    assert_eq!(one.read("flamework.build"), four.read("flamework.build"));
}

/// The setter, its import and the postfix flag, in every position (a spot check beside the expected file).
#[test]
fn attribute_writes_go_through_the_setter() {
    let Some(project) = Project::new(FIXTURE, "setters") else { return };
    let report = build(&project, None);
    assert_clean(&report);
    let text = luau(&report, "shared/attributes.luau");
    assert!(text.contains(".SYMBOL_ATTRIBUTE_SETTER\n"), "{text}");
    for line in [
        "self[SYMBOL_ATTRIBUTE_SETTER](self, \"count\", self.attributes.count + 1, true)",
        "self[SYMBOL_ATTRIBUTE_SETTER](self, \"count\", self.attributes.count - 1)",
        "self[SYMBOL_ATTRIBUTE_SETTER](self, \"label\", self.attributes.label .. \"suffix\")",
        "self[SYMBOL_ATTRIBUTE_SETTER](self, \"count\", bit32.arshift(self.attributes.count, 1))",
        "self[SYMBOL_ATTRIBUTE_SETTER](self, \"note\", nil)",
        "self[SYMBOL_ATTRIBUTE_SETTER](self, \"two words\", self.attributes[\"two words\"] + 7)",
        "self[SYMBOL_ATTRIBUTE_SETTER](self, pickKey(), 4)",
        "local post = self[SYMBOL_ATTRIBUTE_SETTER](self, \"count\", self.attributes.count + 1, true)",
        "local _fn = self[SYMBOL_ATTRIBUTE_SETTER]",
        "local _self = components:getComponent(part, \"shared/attributes@AttributeComponent\")",
        "_self[SYMBOL_ATTRIBUTE_SETTER](_self, \"label\", \"remote\")",
    ] {
        assert!(text.contains(line), "missing `{line}`:\n{text}");
    }
    // Reads are left alone.
    assert!(text.contains("local count = self.attributes.count"), "{text}");
}

/// Where a diagnostic starts, as `file:line:column` (UTF-16 columns from 1).
fn position(d: P<Diagnostic>) -> String {
    let Some(file) = d.file() else { return String::new() };
    let text = file.get().text();
    let pos = (d.pos().unwrap_or(0) as usize).min(text.len());
    let line = text[..pos].matches('\n').count() + 1;
    let line_start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    let column = text[line_start..pos].encode_utf16().count() + 1;
    let name = file.get().file_name().rsplit('/').next().unwrap_or_default().to_owned();
    format!("{name}:{line}:{column}")
}

/// The refused component types and keys: the TS transformer's messages, at its nodes, with its notes (checked against
/// a TS5 build of the same sources). The accessor of `AccessorConfig` (kept in the rebuilt literal) is roblox-ts's own
/// error, which the compiler leaves out of a file with Flamework errors, as TS5 does; alone in a file it is reported.
#[test]
fn component_errors_are_reported() {
    let Some(project) = Project::new("component_errors", "errors") else { return };
    let report = build(&project, None);
    let mut all = report.diagnostics.clone();
    for file in &report.files {
        all.extend(file.diagnostics.iter().copied());
    }
    let mut lines = Vec::new();
    for d in all {
        let category = if d.category() == Category::Error { "error" } else { "warning" };
        // The code slot as the reporter prints it: `TS1117` for a plugin diagnostic with a TypeScript code.
        let code = rbxts_transform::plugins::backend::diagnostic_code(d).map(|c| format!(" TS{c}")).unwrap_or_default();
        lines.push(format!("{category}{code} {}: {}", position(d), flatten_diagnostic_message(d, "\n")));
        for &related in d.related_information() {
            lines.push(format!("    {}: {}", position(related), flatten_diagnostic_message(related, "\n")));
        }
    }
    let ours = lines.join("\n") + "\n";
    let expected_path = fixture_dir("component_errors").join("expected-diagnostics.txt");
    if std::env::var_os("UPDATE_EXPECTED").is_some() {
        std::fs::write(&expected_path, &ours).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&expected_path).unwrap().replace("\r\n", "\n");
    assert!(ours == expected, "diagnostics differ\n--- ours\n{ours}");
}
