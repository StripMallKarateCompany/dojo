// Copyright (c) 2026 Strip Mall Karate Company
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Loads every plan under `tests/fixtures`. A `fail-*` plan must be refused
//! with the message listed here, and every other plan must load.

use std::{fs,
          path::{Path,
                 PathBuf}};

use dojo_plan::Plan;
use serde_json::json;

/// Each refused plan and a piece of the message it must be refused with.
const REFUSED: &[(&str, &str)] =
    &[("fail-advisory-ids-cpe",
       "`advisory_ids.cpe`: `cpe:2.3:a:zlib:zlib:1.3.1` is not a CPE 2.3 prefix"),
      ("fail-advisory-ids-empty", "`advisory_ids` names no identifier"),
      ("fail-advisory-ids-none-beside", "`advisory_ids` sets `none` beside other keys"),
      ("fail-advisory-ids-osv-git",
       "`advisory_ids.osv_git`: `github.com/madler/zlib` is not a repository URL"),
      ("fail-advisory-ids-purl",
       "`advisory_ids.purl`: `pkg:generic/zlib@1.3.1` is not a purl without a version"),
      ("fail-advisory-ids-tag-prefix", "`advisory_ids` sets `osv_git` without `tag_prefix`"),
      ("fail-advisory-ids-tag-prefix-alone", "`advisory_ids` sets `tag_prefix` without `osv_git`"),
      ("fail-advisory-ids-unknown", "`advisory_ids` sets unknown key `osv`"),
      ("fail-bad-ident", "`zlib` is not a package ident"),
      ("fail-exposes", "`exposes` names `admin`, which is not in `exports`"),
      ("fail-fixes-empty", "`fixes` is empty"),
      ("fail-fixes-id", "`fixes[1]`: `CVE-24-1` is not a CVE or OSV id"),
      ("fail-fixes-not-list", "`fixes` is a string, not a table"),
      ("fail-fixes-twice", "`fixes` lists `CVE-2024-1234` twice"),
      ("fail-global-table", "unknown identifier: _G"),
      ("fail-import-macros", "extra module not found"),
      ("fail-include", "module not found extra"),
      ("fail-io", "unknown identifier: io"),
      ("fail-loop", "the plan ran past its budget"),
      ("fail-lua-plan", "has plan.lua but no plan.fnl"),
      ("fail-macro-io", "unknown identifier: io"),
      ("fail-macro-loop", "compiling the plan ran past its budget"),
      ("fail-macro-memory", "not enough memory"),
      ("fail-memory", "the plan ran out of memory"),
      ("fail-missing-name", "does not set `name`"),
      ("fail-not-table", "returns a string, not a table"),
      ("fail-on-name", "`on.x86_64-linux` may not override `name`"),
      ("fail-on-version-source", "`on.x86_64-linux` may not override `version_source`"),
      ("fail-pcall", "unknown identifier: pcall"),
      ("fail-phase-not-fn", "`build` is a string, not a function"),
      ("fail-raw-lua", "attempt to index a nil value (global 'os')"),
      ("fail-string-metatable", "unknown identifier: getmetatable"),
      ("fail-unknown-key", "unknown key `nmae`"),
      ("fail-unknown-target", "`aarch64-darwin` is not in `targets`"),
      ("fail-version-source-bare-none", "`version_source` is a string, not a table"),
      ("fail-version-source-bool", "`version_source.use_max_tag` is a string, not a boolean"),
      ("fail-version-source-empty-reason", "`version_source.none` is empty"),
      ("fail-version-source-no-source", "`version_source` names no source"),
      ("fail-version-source-none-beside", "`version_source` sets `none` beside other keys"),
      ("fail-version-source-not-listed",
       "`version_source.include_regex` filters a list of versions"),
      ("fail-version-source-option",
       "`version_source` sets unknown key `use_max_tags` for a `github` source"),
      ("fail-version-source-pattern", "`version_source` sets `from_pattern` without `to_pattern`"),
      ("fail-version-source-regex-url",
       "`version_source` does not set `url`, which a `regex` source needs"),
      ("fail-version-source-repo", "`version_source.github`: `zlib` is not `owner/repo`"),
      ("fail-version-source-token", "`version_source.token`: a plan holds no credentials"),
      ("fail-version-source-two-sources",
       "`version_source` names more than one source: `github`, `gitlab`"),
      ("fail-windows-target", "`targets`: unknown target `x86_64-windows`")];

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut dirs: Vec<PathBuf> = fs::read_dir(root).expect("tests/fixtures is readable")
                                                   .map(|entry| entry.expect("entry").path())
                                                   .filter(|path| path.is_dir())
                                                   .collect();
    dirs.sort();
    dirs
}

fn name(dir: &Path) -> &str {
    dir.file_name()
       .and_then(|n| n.to_str())
       .expect("UTF-8 name")
}

#[test]
fn every_failing_fixture_is_listed() {
    let failing: Vec<&str> = REFUSED.iter().map(|(name, _)| *name).collect();
    for dir in fixtures() {
        let name = name(&dir);
        if name.starts_with("fail-") {
            assert!(failing.contains(&name),
                    "{name} has no expected message in REFUSED");
        }
    }
}

#[test]
fn refused_plans_fail_with_their_reason() {
    let mut wrong = Vec::new();
    for (fixture, expected) in REFUSED {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
                                                       .join(fixture);
        match Plan::load(&dir) {
            Ok(_) => {
                wrong.push(format!("{fixture} loaded, but should be refused with `{expected}`"));
            }
            Err(error) => {
                let message = error.to_string();
                if !message.contains(expected) {
                    wrong.push(format!("{fixture} was refused with `{message}`, not `{expected}`"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn other_plans_load() {
    for dir in fixtures() {
        if !name(&dir).starts_with("fail-")
           && let Err(error) = Plan::load(&dir)
        {
            panic!("{} did not load: {error}", name(&dir));
        }
    }
}

#[test]
fn a_target_override_merges_into_that_target_only() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/overrides");
    let plan = Plan::load(dir).expect("the plan loads");
    let linux = plan.metadata().to_json(Some("x86_64-linux"));
    let macos = plan.metadata().to_json(Some("aarch64-darwin"));
    assert_eq!(linux["deps"], json!(["karate/glibc", "karate/zlib"]));
    assert_eq!(linux["phases"], json!(["build"]));
    assert_eq!(macos["deps"], json!(["karate/zlib"]));
    assert_eq!(macos["name"], json!("hello"));
    assert_eq!(macos["phases"], json!(["build", "install"]));
    assert!(macos.get("on").is_none());
}

#[test]
fn metadata_is_the_same_every_load() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/launcher");
    let first = Plan::load(&dir).expect("loads").metadata().to_json(None);
    for _ in 0..5 {
        assert_eq!(Plan::load(&dir).expect("loads").metadata().to_json(None),
                   first);
    }
}

#[test]
fn upstream_keys_are_metadata_and_a_source_names_its_kind() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upstream");
    let plan = Plan::load(dir).expect("the plan loads");
    let base = plan.metadata().to_json(None);
    assert_eq!(base["version_source"],
               json!({"source": "github",
                      "github": "tcltk/tcl",
                      "include_regex": "core-8-6-[0-9]+",
                      "prefix": "core-",
                      "use_max_tag": true}));
    assert_eq!(base["advisory_ids"],
               json!({"cpe": "cpe:2.3:a:tcl:tcl",
                      "osv_git": "https://github.com/tcltk/tcl",
                      "tag_prefix": "core-"}));
    assert_eq!(base["fixes"],
               json!(["CVE-2024-1234", "GHSA-jfh8-c2jp-5v3q"]));
    // A target's metadata carries the plan's, which no target overrides.
    let arm = plan.metadata().to_json(Some("aarch64-linux"));
    assert_eq!(arm["version_source"], base["version_source"]);
    assert_eq!(arm["advisory_ids"], base["advisory_ids"]);
    assert_eq!(arm["fixes"], base["fixes"]);
}

#[test]
fn no_upstream_is_a_table_with_its_reason() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upstream-none");
    let json = Plan::load(dir).expect("the plan loads")
                              .metadata()
                              .to_json(None);
    assert_eq!(json["version_source"],
               json!({"none": "vendored source, no upstream release"}));
    assert_eq!(json["advisory_ids"],
               json!({"none": "built from this repository"}));
    assert!(json.get("fixes").is_none());
}
