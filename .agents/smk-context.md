# smk context: dojo

Project facts the smk skills read, by the skill that needs them. AGENTS.md owns policy.

## Tools-engineer

- The Dojo VM (`support/vm`, README "The Dojo VM") is the development loop: an arm64 Linux guest
  under Apple's hypervisor, never emulation. `support/vm/builder` runs the Linux VM that builds the
  binaries: `builder start`, then `builder payload` (the arm64 binaries into
  `support/vm/out/payload.cpio.gz`), `builder test <package>...` (tests and the format check),
  `builder fmt <package>...`, `builder run <command>...`, `builder verify`, and `builder stop` when
  done. `support/vm/boot --payload support/vm/out/payload.cpio.gz` boots them with `doj-launch` as
  PID 1.
- One builder VM runs per state directory (`DOJO_BUILDER_STATE`) and ssh port (`DOJO_BUILDER_PORT`);
  parallel trees need a builder each. Set both on every `builder` command: without them it finds the
  default VM, which may be another session's.
- Every `builder` command but `ssh` copies the tree over the `dojo` directory in the VM's home, so a
  file the VM generated is gone at the next command unless one copied it back. `builder run` returns a changed
  `Cargo.lock`, and `builder fmt` returns the files rustfmt rewrote. `.rustfmt.toml` needs the
  nightly rustfmt `support/vm/guest/env.sh` provides, so format with `builder fmt` rather than by
  hand.
- `rust-toolchain` pins the Rust the tree builds with; `support/vm/guest/env.sh` supplies the C
  libraries through Nix. Read both before changing a dependency; the lock is built `--locked`.
- The `support/vm/guest` scripts run on any Linux with Nix, not only in the builder VM.

## Test-engineer

- `support/vm/builder verify` is the run a pull request reports: every test against
  `support/vm/expected-failures`, the payload, and `boot --check` and `--check-reboot`. It counts the
  binaries that ran against the ones cargo built, so a loader error (exit 127) is a binary that never
  ran, not a failed test; `docs/decisions.md` (2026-11-01) records why.
- Expected failures that are not regressions: the Supervisor fixtures carry `TARGET x86_64-linux` and
  cannot load on arm64; `compile_hook_table` and the Supervisor integration tests install
  `karate/busybox-static` from Builder, which does not exist yet; `dojo_common`'s templating tests
  want a real, writable `/doj` with packages installed. `support/vm/expected-failures` names them; a
  change that makes one pass removes it.
- The Dojo VM proves arm64. x86_64 builds and tests run on smk-1, the Linux machine on the local
  network, with the same `support/vm/guest` scripts. smk-1 is shared: work in a throwaway checkout
  with a throwaway `GIT_CONFIG_GLOBAL`, run builds and tests under `nice -n 15`, delete what you
  created, and confirm none of your processes remain.

## Qa

- Acceptance: `support/vm/boot --check --payload support/vm/out/payload.cpio.gz` and
  `--check-reboot`. `boot --check` without a payload checks the base image.
- Builder is `https://bldr.karate.company`, not running yet; `DOJ_BLDR_URL` overrides it.

## Review

- The source project's name appears nowhere in the tree: not in code, comments, docs, fixtures or
  commit subjects. `NOTICE` shows how to credit its authors without naming it.
- Inherited files keep their `Copyright (c) 20xx Chef Software Inc.` headers unchanged. New files
  start with the Apache 2.0 header in `components/launcher/src/init.rs`, with Strip Mall Karate
  Company's copyright line. Everything is under the Apache License 2.0 (`LICENSE`, `NOTICE`).
- A mechanical rename can mint a name a stranger can register (a GitHub owner, a Docker Hub
  namespace, a package feed). Any renamed hostname or namespace is checked before it ships.

## Docs

- `docs/decisions.md` is the dated record of decisions; the longer account is the workq wiki page
  "Dojo" (`smk wiki list --match Dojo`). The VM's user documentation is README "The Dojo VM".
