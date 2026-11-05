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

//! Resolves dependencies through a fake `doj` that records its calls.

use std::{fs,
          os::unix::fs::PermissionsExt,
          path::{Path,
                 PathBuf}};

use dojo_plan::{DojPkgPath,
                Resolve};

/// A `doj` whose `pkg path` prints the contents of `installed` when that
/// file exists, and whose `pkg install` copies `installable` to
/// `installed`, failing when there is none. Each call is a line in `calls`.
struct FakeDoj {
    dir: PathBuf,
}

impl FakeDoj {
    fn new(test: &str) -> FakeDoj {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("deps")
                                                        .join(test);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the test directory");
        let doj = dir.join("doj");
        fs::write(&doj,
                  format!("#!/bin/sh\nd='{}'\necho \"$*\" >> \"$d/calls\"\ncase \"$1 $2\" in\n  \
                           'pkg path') [ -f \"$d/installed\" ] && exec cat \
                           \"$d/installed\"; exit 1 ;;\n  'pkg install') [ -f \
                           \"$d/installable\" ] || exit 1; cp \"$d/installable\" \
                           \"$d/installed\" ;;\n  *) exit 64 ;;\nesac\n",
                          dir.display())).expect("write doj");
        fs::set_permissions(&doj, fs::Permissions::from_mode(0o755)).expect("make doj runnable");
        FakeDoj { dir }
    }

    fn resolver(&self) -> DojPkgPath { DojPkgPath::new(self.dir.join("doj"), false) }

    fn offline_resolver(&self) -> DojPkgPath { DojPkgPath::new(self.dir.join("doj"), true) }

    fn set(&self, file: &str, path: &str) {
        fs::write(self.dir.join(file), format!("{path}\n")).expect("write the fake's state");
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("calls")).unwrap_or_default()
                                                  .lines()
                                                  .map(String::from)
                                                  .collect()
    }
}

#[test]
fn an_installed_package_is_not_installed_again() {
    let doj = FakeDoj::new("installed");
    doj.set("installed", "/doj/pkgs/karate/zlib/1.3.2/20261028181816");
    assert_eq!(doj.resolver().resolve("karate/zlib"),
               Ok(PathBuf::from("/doj/pkgs/karate/zlib/1.3.2/20261028181816")));
    assert_eq!(doj.calls(), ["pkg path karate/zlib"]);
}

#[test]
fn a_missing_package_is_installed_then_resolved() {
    let doj = FakeDoj::new("installable");
    doj.set("installable", "/doj/pkgs/karate/zlib/1.3.2/20261028181816");
    assert_eq!(doj.resolver().resolve("karate/zlib"),
               Ok(PathBuf::from("/doj/pkgs/karate/zlib/1.3.2/20261028181816")));
    assert_eq!(doj.calls(),
               ["pkg path karate/zlib",
                "pkg install karate/zlib",
                "pkg path karate/zlib"]);
}

#[test]
fn an_offline_resolver_installs_offline() {
    let doj = FakeDoj::new("offline");
    doj.set("installable", "/doj/pkgs/karate/zlib/1.3.2/20261028181816");
    assert_eq!(doj.offline_resolver().resolve("karate/zlib"),
               Ok(PathBuf::from("/doj/pkgs/karate/zlib/1.3.2/20261028181816")));
    assert_eq!(doj.calls(),
               ["pkg path karate/zlib",
                "pkg install --offline karate/zlib",
                "pkg path karate/zlib"]);
}

#[test]
fn a_package_that_cannot_be_installed_fails_to_resolve() {
    let doj = FakeDoj::new("uninstallable");
    let error = doj.resolver()
                   .resolve("karate/zlib")
                   .expect_err("nothing installs karate/zlib");
    assert!(error.contains("could not install"), "{error}");
    assert_eq!(doj.calls(),
               ["pkg path karate/zlib", "pkg install karate/zlib"]);
}

#[test]
fn an_install_that_leaves_nothing_to_resolve_fails() {
    let doj = FakeDoj::new("vanishing");
    doj.set("installable", "");
    let error = doj.resolver()
                   .resolve("karate/zlib")
                   .expect_err("doj pkg path still finds nothing");
    assert!(error.contains("finds no package"), "{error}");
}
