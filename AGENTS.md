# AGENTS.md — Dojo

Guidance for coding agents changing this repository. README.md says what it does; this says how to
change it.

## What this is

Dojo builds software into signed packages and runs them under a Supervisor. The Launcher
(`doj-launch`) is PID 1, and a dojo is a ring of machines whose Supervisors gossip. Rust, pinned by
`rust-toolchain`. An smk project (`smkproj.toml`): `smk proj` renders the dev shell and the checks,
and the Makefile, build scripts and `support/` are the tree's own.

The tree descends from the 0.79.1 release of an Apache-2.0 project (`NOTICE`, `LICENSE`). Nothing
from that project's later releases is taken, whatever it fixes: re-implement. Its old name appears
nowhere in the tree, commit subjects included. The product is Dojo, the command `doj`, the binaries
`doj-sup` and `doj-launch`, the variables `DOJ_*`, the root `/doj`, the package origin `karate`.

## Check

`make ci`. Rust is excluded from it (`smkproj.toml` says why) and is formatted, tested and proved in
the builder VM: `support/vm/builder verify` is the run a pull request reports, and `support/vm/boot
--check` and `--check-reboot` pass before a change to the Launcher, the Supervisor's startup or
`support/vm` opens one.

## Landing

Work in a tree (`trees add <name>`), commit on its `worktree/<name>` branch and open the pull request
with `smk pr create`. The host is GitHub (`StripMallKarateCompany/dojo`); `main` requires no status
checks and no CI runs there, so the merge is left to the owner, as the pr skill says for such a base.
Pushed history is never rewritten (no rebase, squash or amend, no author changes), pull requests
land as merge commits, and commit bodies say why and carry no AI attribution or co-author lines.
Live-state changes outside a merge need explicit authorization.

## Where the rest lives

Per-skill facts (the builder VM, acceptance checks, expected test failures, x86_64 proof, naming and
license headers) are in `.agents/smk-context.md`. `docs/decisions.md` is the dated record; the workq
wiki page "Dojo" (`smk wiki list --match Dojo`) tells the longer story.
