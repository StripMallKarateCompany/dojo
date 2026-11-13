# Decisions

The dated record. `AGENTS.md` and `.agents/smk-context.md` carry the rules these produced; the workq
wiki page "Dojo" tells the longer story.

## 2026-10-25

- **Fork point: 0.79.1** (`edcd8544a`, 10 April 2019), the last release of the source project
  before its binaries went under an end-user license (0.80.0) and, later, a license key. Nothing
  from later releases is taken.
- **History is kept.** The people who built the source project stay in the log; authors are never
  rewritten. The separate core crates repository was subtree-merged with its history; ipc-channel's
  Windows branch is vendored as a snapshot.
- **Rename:** product Dojo; `hab` becomes `doj` everywhere (`doj`, `doj-sup`, `doj-launch`,
  `DOJ_*`, `/doj`). The old name appears nowhere in the tree; Chef Software's copyright lines stay.
- **Everything under karate.company:** package origin `karate`, Builder `bldr.karate.company`,
  Strip Mall Karate Company as maintainer and copyright holder, repository
  `StripMallKarateCompany/dojo` on GitHub, public.
- **Removed:** the old CI and release automation, governance files, website, changelog, CLI
  analytics, and exporters and publishers for retired services (Bintray, ACI, Mesos, Cloud
  Foundry, Kubernetes, Helm). No Expeditor, no Dependabot, no third-party package feeds.
- **A dojo is a primary object: a ring of machines.** Machines install into a dojo; services,
  configuration and keys scope to it.
- **Focus:** PID 1 (grow the Launcher) and a full-machine installer. A container target is
  secondary.
- **License:** Apache 2.0 for everything, inherited files and new work alike (`LICENSE`,
  `NOTICE`); new files carry Strip Mall Karate Company's copyright line.
- **First machines:** a VM on the owner's Apple silicon laptop first (hardware virtualization,
  arm64 guest, never emulation), the physical machine smk-2 (x86_64 UEFI, same hardware profile as
  smk-1) deferred.
- **GUI:** KDE Plasma on Wayland, later; dbus-broker and elogind will run as Supervisor services.
- **PID 1 policy:** as init the Launcher never exits. It restarts a Supervisor that exits for any
  reason, with a delay that starts at one second and doubles to thirty while it keeps exiting
  within a minute of starting. SIGTERM to PID 1 powers the machine off; SIGINT (the kernel's
  Ctrl-Alt-Del) reboots it. `doj sup term` restarts the Supervisor (SIGHUP) when the Launcher is
  PID 1 rather than powering the machine off.
- **Stage-0 bootstrap:** the VM's kernel and userland come from nixpkgs until `karate` packages
  replace them.
- **Toolchain:** the tree ships on Rust 1.33.0 and OpenSSL 1.0.2 and is being moved to current
  stable Rust, Launcher first.

## 2026-10-26

- **No Windows.** Dojo neither runs on Windows nor builds packages for it. Plans build for Linux
  (`x86_64-linux`, `aarch64-linux`, `x86_64-linux-kernel2`) and macOS (`x86_64-darwin`,
  `aarch64-darwin`), and core no longer knows `x86_64-windows`. The Windows code is gone: the
  PowerShell builder and every `plan.ps1`, the Windows service and `win-users`, the Windows
  studio and Docker containers, the Windows branches of core, the Supervisor, the Launcher and
  `doj`, and ipc-channel's Windows platform.
- **macOS is a package platform**, Apple silicon included. Homebrew and Nix are the models for how
  macOS packages are built and relocated.
- **The Supervisor always handles its signals.** `DOJ_FEAT_IGNORE_SIGNALS` is removed: only the
  Windows service set it, and with the Launcher as PID 1 the Supervisor must answer SIGHUP.
- **Real `karate` base packages come before the Launcher proof.** The Fennel-built Launcher is
  compared with the bash-built one using real dependencies (`karate/glibc`, `karate/rust` and the
  rest), not stubs over nixpkgs, so the stage-0 bootstrap of the base packages comes first.
- **Everything moves to current tooling.** The whole tree builds with the newest stable Rust
  (1.99.1, read from `rust-toolchain`), then moves to current nixpkgs and OpenSSL 3, current
  versions of every crate, and edition 2024, a phase at a time, each phase passing the boot checks.
- **Templates render on Handlebars 6 with their old output.** Configuration and hook templates
  were written for handlebars 0.28, which never removed a line holding only a block tag, so
  authors put a tilde on one side of such tags (`{{~#if x}}`, `{{#if x ~}}`) to manage newlines.
  Handlebars 6 removes those lines as the Handlebars spec says, and the tilde then joins lines.
  The renderer rewrites a one-sided tilde on a standalone block tag so the output is what 0.28
  produced, and plan authors keep writing templates as they do now. Rendered against every
  template in `dojo-plans`, output is byte-identical or differs only in blank lines, except that
  `toYaml` writes plain YAML without a leading `---` (same data) and an array printed directly
  loses the comma and space after its last element. Two things break: `foo[0]` must be written `foo.[0]` (nothing uses
  it), and pest stays at 2.8 until the Supervisor's actix-web 0.7, whose ring 0.13 holds cc at
  1.0.95, is replaced.
- **Old dependencies are replaced, not only upgraded.** Where a dependency reflects what Rust
  offered when the tree was written (zmq, futures 0.1, tokio 0.1, actix-web 0.7), it gives way
  to what a new project would choose today, at its latest release.
- **Gossip travels over TCP on tokio, not ZeroMQ.** Each gossip round opens one connection per
  member and sends each rumor as a frame of a 4-byte big-endian length and its `Wire` message
  (`tokio_util`'s length-delimited codec), with frames capped at 8 MiB. Gossip runs on a tokio
  runtime butterfly owns until the Supervisor moves to tokio 1 and shares its own. SWIM stays
  on its UDP threads: its loops are timers over one socket, and async buys them nothing. A
  Supervisor with this change cannot exchange rumors with one using ZeroMQ, although SWIM
  membership still works between them. Nothing is deployed, so nothing is lost. libzmq leaves
  the build and the payload.
- **The Supervisor runs on one tokio 1 runtime.** Its main loop stays a synchronous loop on the
  main thread, and the runtime (`DOJ_TOKIO_THREAD_COUNT` workers) carries the ctl gateway, the
  HTTP gateway, gossip, the event stream and service operations. Stopping a service, its hooks
  and each ctl command run on the runtime's blocking threads, since they wait on processes or
  install packages. Shutdown waits for every service operation and command still running, as the
  old runtime's `shutdown_on_idle` did. Butterfly gossips on this runtime instead of its own.
  `DOJ_SUP_HTTP_THREADS` is gone: the HTTP gateway has no threads of its own to size.
- **The HTTP gateway is axum.** It is a thin layer over hyper 1 and tower that runs on the
  Supervisor's runtime, where actix-web 4 would bring runtimes of its own. Routes, responses,
  bearer-token authentication and `DOJ_FEAT_REDACT_HTTP` behave as before. TLS is rustls 0.23
  with the ring provider, and the key may now be PKCS#8 or SEC1 as well as PKCS#1 RSA.
- **`doj` talks to the Supervisor with a blocking client.** The ctl gateway serves one request
  per connection, so `doj` connects, sends, reads replies until the last and closes. `doj` and
  `sup-client` no longer use tokio or futures, and the codec is `tokio_util`'s on bytes 1. A
  connection whose request the Supervisor drops unanswered now closes instead of hanging.
- **Events go to NATS through async-nats.** NATS Streaming, which nitox spoke, has reached its
  end of life. Events are published on the subject `dojo` to `nats://127.0.0.1:4222`; a
  JetStream stream on that subject keeps them if the server has one. The prototype's cluster ID
  and port 4223 are gone.
- **pest moves to 2.9 and cc to its current release** now that ring 0.13 left with actix-web 0.7.

## 2026-11-01

- **A test run counts the binaries that started.** While the tree linked OpenSSL 1.0.2 from an old
  nixpkgs, every test binary that loaded libcrypto died at load with exit 127
  (`libdl.so.2: undefined symbol: _dl_catch_error`), so seven test binaries silently never ran for
  the first months of this fork. A loader error is not a failed test: `support/vm/builder verify`
  counts the binaries that ran against the ones cargo built before calling a run green, and it is
  the run a pull request reports.

## 2026-11-02

- **The Rust is in `smk format` and `smk typecheck`, not yet `smk lint`.** The builder VM, smk-1
  and the dev shell format with one nightly rustfmt, built from the nixpkgs `.nix/flake.lock`
  pins (`support/vm/guest/rustfmt.nix`), so a lock update moves them together; the VM's own
  nixos-26.05 pin had given it rustfmt 1.95 against the dev shell's 1.99. Clippy finds some 2,760
  problems in the inherited code, so `**/*.rs` stays under `Format.SkipLint` until DOJ-31 works
  them off. Vendored ipc-channel stays a workspace member, so `builder verify` still tests the
  tree's patches to it, but `.rustfmt.toml` ignores `vendor` and its nightly-only bench needs the
  crate's `unstable` feature.

## 2026-11-04

- **The live image is written by the live system itself.** `boot --write-image` boots the live
  initramfs once in QEMU and `doj-write-image` partitions the image and installs grub with
  karate's own sfdisk, dosfstools and grub, as `doj-install` does for a disk. The image so
  needs nothing from nixpkgs but QEMU and builds on a Mac, where no grub for `i386-pc` runs.
  The stick is a GPT disk whose EFI system partition holds the kernel and initramfs, not an ISO
  9660 hybrid: `grub-mkrescue` would need xorriso and mtools, which are not karate packages, and
  the item's USB stick needs only the disk. The live system runs from RAM, so the stick is not
  read after grub loads it.

## 2026-11-05

- **The modernized tree is proved on x86_64.** `support/vm/guest/verify.sh` on smk-1 (Ubuntu
  26.04, x86_64) at main 4cefd83b6 built 32 test binaries and ran all 32 to a result, with 17
  doc-test runs, no loader errors and 11 failures, every one listed in
  `support/vm/expected-failures`; `builder verify` on the same commit in the arm64 VM ran 32 of
  32 with its 21 expected failures. The ten that pass only on smk-1 are the six whose
  `TARGET x86_64-linux` fixtures load there and the four hook tests that find smk-1's
  `/usr/bin/busybox` on `PATH`, so the list stays the builder VM's, and on smk-1 the report notes
  them without failing. The nine Supervisor integration tests fail on x86_64 for a reason the list
  had not named: they want `target/debug/doj-launch`, which `cargo test` does not build.

## 2026-11-13

- **No `bldr`.** The 2026-10-25 entry's "Builder `bldr.karate.company`" is withdrawn. bldr is the
  old software that inspired buildq: it inspires what comes next, and nothing of it is reused, its
  names, hostnames, URL shapes and API included. No `bldr` hostname is served, and the name leaves
  the tree as the source project's own name did: the compiled-in default, `DOJ_BLDR_URL`, the
  `doj bldr` subcommand, `bldr.toml` and the depot API client (DOJ-17). buildq hosts packages in
  its own shape (BLD-119). Until DOJ-17 lands, `README.md` and `.agents/smk-context.md` still
  describe the default the code has.
