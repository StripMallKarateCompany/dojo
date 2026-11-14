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

//! Runs `doj-plan-build show` as a shell would. The studio names a build's log
//! from `show <plan-dir> --field name`, so a field prints bare, on one line.

use std::{path::{Path,
                 PathBuf},
          process::{Command,
                    Output}};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
                                         .join(name)
}

fn run(command: &str, dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doj-plan-build")).arg(command)
                                                      .arg(dir)
                                                      .args(args)
                                                      .output()
                                                      .expect("doj-plan-build runs")
}

fn stdout(output: &Output) -> &str {
    assert!(output.status.success(),
            "doj-plan-build failed: {}",
            String::from_utf8_lossy(&output.stderr));
    std::str::from_utf8(&output.stdout).expect("UTF-8 output")
}

#[test]
fn a_string_field_prints_bare() {
    let output = run("show", &fixture("minimal"), &["--field", "name"]);
    assert_eq!(stdout(&output), "hello\n");
}

#[test]
fn a_plan_in_a_dojo_directory_shows_its_name() {
    // The studio passes a build's context, which holds its plan in `dojo/`
    // when the plan is not at the top, as this package's does.
    let output = run("show",
                     Path::new(env!("CARGO_MANIFEST_DIR")),
                     &["--field", "name"]);
    assert_eq!(stdout(&output), "doj-plan-build\n");
}

#[test]
fn any_other_field_prints_as_json() {
    let output = run("show",
                     &fixture("overrides"),
                     &["--target", "aarch64-darwin", "--field", "deps"]);
    assert_eq!(stdout(&output), "[\"karate/zlib\"]\n");
}

#[test]
fn upstream_keys_print_as_json() {
    let dir = fixture("upstream-regex");
    let source = run("show", &dir, &["--field", "version_source"]);
    assert_eq!(stdout(&source),
               "{\"source\":\"regex\",\"exclude_regex\":\"1[.]2[.].*\",\
                \"regex\":\"zlib-([0-9.]+)[.]tar[.]xz\",\"url\":\"https://zlib.net/\"}\n");
    let ids = run("show", &dir, &["--field", "advisory_ids"]);
    assert_eq!(stdout(&ids), "{\"purl\":\"pkg:generic/zlib\"}\n");
    let fixes = run("show", &fixture("upstream"), &["--field", "fixes"]);
    assert_eq!(stdout(&fixes),
               "[\"CVE-2024-1234\",\"GHSA-jfh8-c2jp-5v3q\"]\n");
}

#[test]
fn a_missing_field_fails() {
    let output = run("show", &fixture("minimal"), &["--field", "nmae"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no `nmae`"));
}

#[test]
fn field_belongs_to_show_only() {
    let output = run("build", &fixture("minimal"), &["--field", "name"]);
    assert_eq!(output.status.code(), Some(2));
}
