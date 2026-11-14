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

//! The keys a plan may set, and how each is checked and reported.
//!
//! Parsing reads the plan's table without running any of its code: tables
//! with metatables are refused, and every read is raw.

use std::collections::{BTreeMap,
                       BTreeSet};

use mlua::{Table,
           Value};
use serde_json::{Map,
                 Value as Json,
                 json};

use crate::Error;

/// The package targets plans build for, Linux and macOS, as
/// `components/core` names them.
pub const TARGETS: &[&str] = &["x86_64-linux",
                               "aarch64-linux",
                               "x86_64-linux-kernel2",
                               "x86_64-darwin",
                               "aarch64-darwin"];

/// The build phases, in the order a build runs them.
pub const PHASES: &[&str] =
    &["download", "verify", "unpack", "prepare", "build", "check", "install", "strip"];

#[derive(Clone, Copy)]
enum Kind {
    Name,
    Text,
    Version,
    Targets,
    Texts,
    Idents,
    Scaffolding,
    Paths,
    Source,
    VersionSource,
    AdvisoryIds,
    Fixes,
    Type,
    Svc,
    Exports,
    Binds,
    On,
}

/// Metadata keys in the order `show` prints them.
const KEYS: &[(&str, Kind)] = &[("name", Kind::Name),
                                ("origin", Kind::Name),
                                ("version", Kind::Version),
                                ("targets", Kind::Targets),
                                ("maintainer", Kind::Text),
                                ("license", Kind::Texts),
                                ("description", Kind::Text),
                                ("upstream_url", Kind::Text),
                                ("source", Kind::Source),
                                ("version_source", Kind::VersionSource),
                                ("advisory_ids", Kind::AdvisoryIds),
                                ("fixes", Kind::Fixes),
                                ("deps", Kind::Idents),
                                ("build_deps", Kind::Idents),
                                ("scaffolding", Kind::Scaffolding),
                                ("type", Kind::Type),
                                ("bin_dirs", Kind::Paths),
                                ("lib_dirs", Kind::Paths),
                                ("include_dirs", Kind::Paths),
                                ("pconfig_dirs", Kind::Paths),
                                ("interpreters", Kind::Paths),
                                ("svc", Kind::Svc),
                                ("exports", Kind::Exports),
                                ("exposes", Kind::Texts),
                                ("binds", Kind::Binds),
                                ("binds_optional", Kind::Binds),
                                ("on", Kind::On)];

const REQUIRED: &[&str] = &["name", "origin", "version", "targets"];

/// Keys a target's `on` entry may not override. Where new versions come
/// from, what advisories call the software and what its patches fix are the
/// plan's, whatever it builds for.
const BASE_ONLY: &[&str] = &["name",
                             "origin",
                             "targets",
                             "version_source",
                             "advisory_ids",
                             "fixes",
                             "on"];

#[derive(Clone, Debug)]
enum Field {
    Json(Json),
    ComputedVersion,
    Phase,
}

/// One table's worth of checked keys: the plan's base, or one `on` entry.
#[derive(Clone, Debug, Default)]
struct Fields(BTreeMap<&'static str, Field>);

/// A plan's checked metadata.
#[derive(Debug)]
pub struct Metadata {
    base: Fields,
    on:   BTreeMap<String, Fields>,
}

impl Metadata {
    /// Checks a plan's returned table.
    ///
    /// # Errors
    ///
    /// [`Error::Invalid`] names the first key that does not fit the schema.
    pub fn parse(table: &Table) -> Result<Metadata, Error> {
        let mut base = Fields::default();
        let mut on = BTreeMap::new();
        for (key, value) in entries(table, "the plan")? {
            match kind_of(&key) {
                Some(("on", _)) => on = parse_on(&value)?,
                Some((name, kind)) => {
                    base.0.insert(name, parse_field(kind, &value, name)?);
                }
                None => return Err(unknown_key(&key, "the plan")),
            }
        }
        for name in REQUIRED {
            if !base.0.contains_key(name) {
                return Err(Error::Invalid(format!("the plan does not set `{name}`")));
            }
        }
        let targets = base.strings("targets");
        for target in on.keys() {
            if !targets.contains(target) {
                return Err(Error::Invalid(format!("`on.{target}`: `{target}` is not \
                                                   in `targets`")));
            }
        }
        let metadata = Metadata { base, on };
        metadata.base.check_exposes("")?;
        for target in &targets {
            metadata.merged(target)
                    .check_exposes(&format!("`on.{target}`: "))?;
        }
        Ok(metadata)
    }

    /// The targets the plan builds for.
    #[must_use]
    pub fn targets(&self) -> Vec<String> { self.base.strings("targets") }

    /// The metadata as JSON: the base keys with each target's overrides
    /// under `on`, or, given a target, the base with that target's overrides
    /// applied.
    #[must_use]
    pub fn to_json(&self, target: Option<&str>) -> Json {
        if let Some(target) = target {
            let mut out = Map::new();
            out.insert("target".into(), json!(target));
            out.extend(self.merged(target).to_json());
            Json::Object(out)
        } else {
            let mut out = self.base.to_json();
            if !self.on.is_empty() {
                let on =
                    self.on
                        .iter()
                        .map(|(target, fields)| (target.clone(), Json::Object(fields.to_json())))
                        .collect();
                out.insert("on".into(), Json::Object(on));
            }
            Json::Object(out)
        }
    }

    fn merged(&self, target: &str) -> Fields {
        let mut fields = self.base.clone();
        if let Some(overrides) = self.on.get(target) {
            fields.0
                  .extend(overrides.0.iter().map(|(key, field)| (*key, field.clone())));
        }
        fields
    }
}

impl Fields {
    fn strings(&self, key: &str) -> Vec<String> {
        match self.0.get(key) {
            Some(Field::Json(Json::Array(items))) => {
                items.iter()
                     .filter_map(|item| item.as_str().map(String::from))
                     .collect()
            }
            _ => Vec::new(),
        }
    }

    fn check_exposes(&self, prefix: &str) -> Result<(), Error> {
        let exports: BTreeSet<String> = match self.0.get("exports") {
            Some(Field::Json(Json::Object(exports))) => exports.keys().cloned().collect(),
            _ => BTreeSet::new(),
        };
        for exposed in self.strings("exposes") {
            if !exports.contains(&exposed) {
                return Err(Error::Invalid(format!("{prefix}`exposes` names \
                                                   `{exposed}`, which is not in \
                                                   `exports`")));
            }
        }
        Ok(())
    }

    fn to_json(&self) -> Map<String, Json> {
        let mut out = Map::new();
        for (name, _) in KEYS {
            match self.0.get(name) {
                Some(Field::Json(value)) => {
                    out.insert((*name).into(), value.clone());
                }
                Some(Field::ComputedVersion) => {
                    out.insert("version".into(), Json::Null);
                    out.insert("version_computed".into(), json!(true));
                }
                Some(Field::Phase) | None => {}
            }
        }
        let phases: Vec<&str> =
            PHASES.iter()
                  .copied()
                  .filter(|phase| matches!(self.0.get(phase), Some(Field::Phase)))
                  .collect();
        if !phases.is_empty() {
            out.insert("phases".into(), json!(phases));
        }
        out
    }
}

fn kind_of(key: &str) -> Option<(&'static str, Option<Kind>)> {
    if let Some((name, kind)) = KEYS.iter().find(|(name, _)| *name == key) {
        return Some((name, Some(*kind)));
    }
    PHASES.iter()
          .find(|phase| **phase == key)
          .map(|phase| (*phase, None))
}

fn unknown_key(key: &str, context: &str) -> Error {
    Error::Invalid(format!("{context} sets unknown key `{key}`"))
}

fn parse_on(value: &Value) -> Result<BTreeMap<String, Fields>, Error> {
    let mut on = BTreeMap::new();
    for (target, value) in entries(&table_of(value, "`on`")?, "`on`")? {
        if !TARGETS.contains(&target.as_str()) {
            return Err(Error::Invalid(format!("`on.{target}`: unknown target `{target}`")));
        }
        let context = format!("`on.{target}`");
        let mut fields = Fields::default();
        for (key, value) in entries(&table_of(&value, &context)?, &context)? {
            match kind_of(&key) {
                Some((name, _)) if BASE_ONLY.contains(&name) => {
                    return Err(Error::Invalid(format!("{context} may not override `{name}`")));
                }
                Some((name, kind)) => {
                    let what = format!("on.{target}.{name}");
                    fields.0.insert(name, parse_field(kind, &value, &what)?);
                }
                None => return Err(unknown_key(&key, &context)),
            }
        }
        on.insert(target, fields);
    }
    Ok(on)
}

/// Parses one key. `kind` is `None` for a phase.
fn parse_field(kind: Option<Kind>, value: &Value, what: &str) -> Result<Field, Error> {
    let what = format!("`{what}`");
    let Some(kind) = kind else {
        return match value {
            Value::Function(_) => Ok(Field::Phase),
            other => Err(wrong_type(&what, "a function", other)),
        };
    };
    let json = match kind {
        Kind::Name => json!(name_of(value, &what)?),
        Kind::Text => json!(string_of(value, &what)?),
        Kind::Version => {
            return match value {
                Value::Function(_) => Ok(Field::ComputedVersion),
                Value::String(_) => Ok(Field::Json(json!(nonempty(value, &what)?))),
                other => Err(wrong_type(&what, "a string or a function", other)),
            };
        }
        Kind::Targets => {
            let targets = strings_of(value, &what)?;
            if targets.is_empty() {
                return Err(Error::Invalid(format!("{what} is empty")));
            }
            let mut seen = BTreeSet::new();
            for target in &targets {
                if !TARGETS.contains(&target.as_str()) {
                    return Err(Error::Invalid(format!("{what}: unknown target `{target}`")));
                }
                if !seen.insert(target) {
                    return Err(Error::Invalid(format!("{what} lists `{target}` twice")));
                }
            }
            json!(targets)
        }
        Kind::Texts => json!(strings_of(value, &what)?),
        Kind::Idents => {
            let idents = strings_of(value, &what)?;
            for ident in &idents {
                check_ident(ident, &what)?;
            }
            json!(idents)
        }
        Kind::Paths => {
            let paths = strings_of(value, &what)?;
            for path in &paths {
                check_relative(path, &what)?;
            }
            json!(paths)
        }
        Kind::Source => {
            let fields = record(value, &what, &["url", "sha256"], &["filename", "dirname"])?;
            let sha = fields["sha256"].as_str().unwrap_or_default();
            if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::Invalid(format!("{what}: `sha256` is not 64 \
                                                   hexadecimal digits")));
            }
            Json::Object(fields)
        }
        Kind::VersionSource => version_source_of(value, &what)?,
        Kind::AdvisoryIds => advisory_ids_of(value, &what)?,
        Kind::Fixes => fixes_of(value, &what)?,
        Kind::Scaffolding => scaffolding_of(value, &what)?,
        Kind::Type => {
            let kind = string_of(value, &what)?;
            if kind != "standalone" && kind != "composite" {
                return Err(Error::Invalid(format!("{what} is `{kind}`, not \
                                                   `standalone` or `composite`")));
            }
            json!(kind)
        }
        // `run` may be left out when the plan has a run hook (hooks/run), which
        // starts the service instead; the build checks that one of them is there.
        Kind::Svc => Json::Object(record(value, &what, &[], &["run", "user", "group"])?),
        Kind::Exports => {
            let mut exports = Map::new();
            for (key, value) in entries(&table_of(value, &what)?, &what)? {
                exports.insert(key.clone(),
                               json!(nonempty(&value, &format!("{what}.{key}"))?));
            }
            Json::Object(exports)
        }
        Kind::Binds => {
            let mut binds = Map::new();
            for (key, value) in entries(&table_of(value, &what)?, &what)? {
                binds.insert(key.clone(),
                             json!(strings_of(&value, &format!("{what}.{key}"))?));
            }
            Json::Object(binds)
        }
        Kind::On => unreachable!("`on` is parsed by parse_on"),
    };
    Ok(Field::Json(json))
}

/// The keys a scaffolding's `metadata` function may give the plans that use
/// it.
pub const CONTRIBUTED: &[&str] = &["maintainer",
                                   "license",
                                   "description",
                                   "upstream_url",
                                   "deps",
                                   "build_deps",
                                   "bin_dirs",
                                   "lib_dirs",
                                   "include_dirs",
                                   "pconfig_dirs",
                                   "interpreters",
                                   "svc",
                                   "exports",
                                   "exposes",
                                   "binds",
                                   "binds_optional"];

/// Checks what a scaffolding's `metadata` function returned, key by key as
/// a plan's own.
///
/// # Errors
///
/// [`Error::Invalid`] names the first key that is not one of [`CONTRIBUTED`]
/// or does not fit the schema.
pub fn parse_contribution(value: &Value) -> Result<Map<String, Json>, Error> {
    let what = "the scaffolding's metadata";
    let mut out = Map::new();
    for (key, value) in entries(&table_of(value, what)?, what)? {
        match kind_of(&key) {
            Some((name, Some(kind))) if CONTRIBUTED.contains(&name) => {
                if let Field::Json(json) =
                    parse_field(Some(kind), &value, &format!("scaffolding metadata.{name}"))?
                {
                    out.insert(name.into(), json);
                }
            }
            _ => {
                return Err(Error::Invalid(format!("{what} sets `{key}`; a \
                                                   scaffolding may set {}",
                                                  CONTRIBUTED.join(", "))));
            }
        }
    }
    Ok(out)
}

/// Adds a scaffolding's metadata to a plan's, as JSON. A list gains the
/// scaffolding's entries it does not hold, after its own; a table gains the
/// keys it does not set; any other value the plan sets is the plan's.
///
/// # Errors
///
/// [`Error::Invalid`] when `exposes` then names a key `exports` does not.
pub fn merge_contribution(plan: &mut Json, added: Map<String, Json>) -> Result<(), Error> {
    let Json::Object(plan) = plan else {
        return Ok(());
    };
    for (key, value) in added {
        match (plan.get_mut(&key), value) {
            (Some(Json::Array(own)), Json::Array(more)) => {
                for item in more {
                    if !own.contains(&item) {
                        own.push(item);
                    }
                }
            }
            (Some(Json::Object(own)), Json::Object(more)) => {
                for (name, item) in more {
                    own.entry(name).or_insert(item);
                }
            }
            (Some(_), _) => {}
            (None, value) => {
                plan.insert(key, value);
            }
        }
    }
    let exports: BTreeSet<&str> = plan.get("exports")
                                      .and_then(Json::as_object)
                                      .map(|exports| exports.keys().map(String::as_str).collect())
                                      .unwrap_or_default();
    for exposed in plan.get("exposes")
                       .and_then(Json::as_array)
                       .into_iter()
                       .flatten()
                       .filter_map(Json::as_str)
    {
        if !exports.contains(exposed) {
            return Err(Error::Invalid(format!("`exposes` names `{exposed}`, \
                                               which is not in `exports` once \
                                               the scaffolding's metadata is \
                                               added")));
        }
    }
    Ok(())
}

/// Plain data: strings, numbers, booleans, lists and tables with string
/// keys, as JSON.
fn data_of(value: &Value, what: &str) -> Result<Json, Error> {
    match value {
        Value::String(_) => Ok(json!(string_of(value, what)?)),
        Value::Integer(i) => Ok(json!(i)),
        Value::Number(n) => {
            serde_json::Number::from_f64(*n).map(Json::Number)
                                            .ok_or_else(|| {
                                                Error::Invalid(format!("{what} is not a finite \
                                                                        number"))
                                            })
        }
        Value::Boolean(b) => Ok(json!(b)),
        Value::Table(_) => {
            let table = table_of(value, what)?;
            let length = table.raw_len();
            if length == 0 {
                let mut out = Map::new();
                for (key, value) in entries(&table, what)? {
                    let item = data_of(&value, &format!("{what}.{key}"))?;
                    out.insert(key, item);
                }
                return Ok(Json::Object(out));
            }
            if table.pairs::<Value, Value>().count() != length {
                return Err(Error::Invalid(format!("{what} mixes a list with named keys")));
            }
            (1..=length).map(|i| {
                            let item = table.raw_get::<Value>(i)
                                            .map_err(|e| Error::Invalid(format!("{what}: {e}")))?;
                            data_of(&item, &format!("{what}[{i}]"))
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map(Json::Array)
        }
        other => {
            Err(wrong_type(what,
                           "a string, number, boolean or table",
                           other))
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Opt {
    Text,
    Flag,
}

/// Whether a source returns a list of versions, which `include_regex` and
/// `exclude_regex` filter. nvchecker applies them only to a source that
/// returns a list, so a pin on any other would hold nothing.
#[derive(Clone, Copy)]
enum Lists {
    Always,
    Never,
    /// With any of these flags set.
    With(&'static [&'static str]),
    /// Unless this flag is set.
    Without(&'static str),
}

struct VersionSource {
    name:     &'static str,
    /// What the source's own key holds.
    holds:    Holds,
    options:  &'static [(&'static str, Opt)],
    required: &'static [&'static str],
    lists:    Lists,
}

#[derive(Clone, Copy, PartialEq)]
enum Holds {
    Repo,
    Text,
}

/// The sources `version_source` may name, with nvchecker's names and options
/// (its "Usage" page), so its configuration is the table as written plus
/// `source`. Options that reach a service with a credential (`token`,
/// `httptoken`) are the poller's, never a plan's.
const VERSION_SOURCES: &[VersionSource] =
    &[VersionSource { name:     "github",
                      holds:    Holds::Repo,
                      options:  &[("branch", Opt::Text),
                                  ("path", Opt::Text),
                                  ("host", Opt::Text),
                                  ("query", Opt::Text),
                                  ("use_latest_release", Opt::Flag),
                                  ("use_max_release", Opt::Flag),
                                  ("use_release_name", Opt::Flag),
                                  ("include_prereleases", Opt::Flag),
                                  ("use_latest_tag", Opt::Flag),
                                  ("use_max_tag", Opt::Flag)],
                      required: &[],
                      lists:    Lists::With(&["use_max_tag", "use_max_release"]), },
      VersionSource { name:     "gitlab",
                      holds:    Holds::Repo,
                      options:  &[("branch", Opt::Text),
                                  ("host", Opt::Text),
                                  ("use_max_tag", Opt::Flag)],
                      required: &[],
                      lists:    Lists::With(&["use_max_tag"]), },
      VersionSource { name:     "gitea",
                      holds:    Holds::Repo,
                      options:  &[("branch", Opt::Text),
                                  ("host", Opt::Text),
                                  ("use_max_tag", Opt::Flag),
                                  ("use_latest_release", Opt::Flag)],
                      required: &[],
                      lists:    Lists::With(&["use_max_tag"]), },
      VersionSource { name:     "git",
                      holds:    Holds::Text,
                      options:  &[("branch", Opt::Text), ("use_commit", Opt::Flag)],
                      required: &[],
                      lists:    Lists::Without("use_commit"), },
      VersionSource { name:     "regex",
                      holds:    Holds::Text,
                      options:  &[("url", Opt::Text),
                                  ("encoding", Opt::Text),
                                  ("post_data", Opt::Text),
                                  ("post_data_type", Opt::Text)],
                      required: &["url"],
                      lists:    Lists::Always, },
      VersionSource { name:     "pypi",
                      holds:    Holds::Text,
                      options:  &[("use_pre_release", Opt::Flag)],
                      required: &[],
                      lists:    Lists::Always, },
      VersionSource { name:     "cratesio",
                      holds:    Holds::Text,
                      options:  &[("use_pre_release", Opt::Flag)],
                      required: &[],
                      lists:    Lists::Always, },
      VersionSource { name:     "gems",
                      holds:    Holds::Text,
                      options:  &[],
                      required: &[],
                      lists:    Lists::Always, },
      VersionSource { name:     "npm",
                      holds:    Holds::Text,
                      options:  &[],
                      required: &[],
                      lists:    Lists::Never, },
      VersionSource { name:     "cran",
                      holds:    Holds::Text,
                      options:  &[],
                      required: &[],
                      lists:    Lists::Never, }];

/// Options every source takes. `prefix` is stripped from each version, and
/// `from_pattern` rewrites it to `to_pattern`; the two list options are
/// Python regexes a whole version must match.
const SOURCE_OPTIONS: &[&str] = &["prefix",
                                  "from_pattern",
                                  "to_pattern",
                                  "include_regex",
                                  "exclude_regex"];
const LIST_OPTIONS: &[&str] = &["include_regex", "exclude_regex"];
const CREDENTIALS: &[&str] = &["token", "httptoken"];

/// A key inside the table `what` names, as `table.key`.
fn nested(what: &str, key: &str) -> String { format!("`{}.{key}`", what.trim_matches('`')) }

/// `{:none "<reason>"}`: the reason a plan has nothing to declare.
fn none_of(entries: &[(String, Value)], what: &str) -> Result<Option<Json>, Error> {
    let Some((_, reason)) = entries.iter().find(|(key, _)| key == "none") else {
        return Ok(None);
    };
    if entries.len() > 1 {
        return Err(Error::Invalid(format!("{what} sets `none` beside other keys; \
                                           `none` stands alone with its reason")));
    }
    Ok(Some(json!({"none": nonempty(reason, &nested(what, "none"))?})))
}

/// Where new upstream versions are published, as one nvchecker source and its
/// options, or `{:none "<reason>"}`. The JSON names the source as `source`
/// first, as nvchecker's configuration does.
fn version_source_of(value: &Value, what: &str) -> Result<Json, Error> {
    let entries = entries(&table_of(value, what)?, what)?;
    if let Some(none) = none_of(&entries, what)? {
        return Ok(none);
    }
    let named: Vec<&VersionSource> =
        VERSION_SOURCES.iter()
                       .filter(|source| entries.iter().any(|(key, _)| key == source.name))
                       .collect();
    let source = match named.as_slice() {
        [source] => *source,
        [] => {
            let names: Vec<&str> = VERSION_SOURCES.iter().map(|source| source.name).collect();
            return Err(Error::Invalid(format!("{what} names no source; set one of {}, or \
                                               `none` with the reason",
                                              names.join(", "))));
        }
        _ => {
            let names: Vec<String> = named.iter()
                                          .map(|source| format!("`{}`", source.name))
                                          .collect();
            return Err(Error::Invalid(format!("{what} names more than one source: {}",
                                              names.join(", "))));
        }
    };
    let mut out = Map::new();
    out.insert("source".into(), json!(source.name));
    let mut flags = BTreeSet::new();
    for (key, value) in &entries {
        let item = nested(what, key);
        let opt = if key == source.name {
            None
        } else if CREDENTIALS.contains(&key.as_str()) {
            return Err(Error::Invalid(format!("{item}: a plan holds no credentials; the \
                                               poller supplies them")));
        } else if SOURCE_OPTIONS.contains(&key.as_str()) {
            Some(Opt::Text)
        } else if let Some((_, opt)) = source.options.iter().find(|(name, _)| name == key) {
            Some(*opt)
        } else {
            let mut takes: Vec<&str> = source.options.iter().map(|(name, _)| *name).collect();
            takes.extend(SOURCE_OPTIONS);
            return Err(Error::Invalid(format!("{what} sets unknown key `{key}` for a `{}` \
                                               source, which takes {}",
                                              source.name,
                                              takes.join(", "))));
        };
        let json = match opt {
            None => {
                let held = nonempty(value, &item)?;
                if source.holds == Holds::Repo && !is_repo(&held) {
                    return Err(Error::Invalid(format!("{item}: `{held}` is not `owner/repo`")));
                }
                json!(held)
            }
            Some(Opt::Text) => json!(nonempty(value, &item)?),
            Some(Opt::Flag) => {
                match value {
                    Value::Boolean(flag) => {
                        if *flag {
                            flags.insert(key.as_str());
                        }
                        json!(flag)
                    }
                    other => return Err(wrong_type(&item, "a boolean", other)),
                }
            }
        };
        out.insert(key.clone(), json);
    }
    for key in source.required {
        if !out.contains_key(*key) {
            return Err(Error::Invalid(format!("{what} does not set `{key}`, which a `{}` \
                                               source needs",
                                              source.name)));
        }
    }
    for (one, other) in [("from_pattern", "to_pattern"),
                         ("to_pattern", "from_pattern")]
    {
        if out.contains_key(one) && !out.contains_key(other) {
            return Err(Error::Invalid(format!("{what} sets `{one}` without `{other}`")));
        }
    }
    let lists = match source.lists {
        Lists::Always => true,
        Lists::Never => false,
        Lists::With(any) => any.iter().any(|flag| flags.contains(flag)),
        Lists::Without(flag) => !flags.contains(flag),
    };
    if !lists && let Some(key) = LIST_OPTIONS.iter().find(|key| out.contains_key(**key)) {
        let how = match source.lists {
            Lists::With(any) => {
                let any: Vec<String> = any.iter().map(|flag| format!("`{flag}`")).collect();
                format!("; set {}", any.join(" or "))
            }
            Lists::Without(flag) => format!("; leave out `{flag}`"),
            Lists::Always | Lists::Never => String::new(),
        };
        return Err(Error::Invalid(format!("{} filters a list of versions, and this `{}` \
                                           source returns one version{how}",
                                          nested(what, key),
                                          source.name)));
    }
    Ok(Json::Object(out))
}

/// `owner/repo`, or a group path on hosts that nest them.
fn is_repo(text: &str) -> bool {
    let parts: Vec<&str> = text.split('/').collect();
    parts.len() >= 2
    && parts.iter()
            .all(|part| !part.is_empty() && !part.contains(char::is_whitespace))
}

/// The identifiers advisory databases know the software by, or
/// `{:none "<reason>"}`. `tag_prefix` spells a version as the upstream
/// repository's tag (`v` for `v1.3.1`, empty when the tag is the version),
/// which matching against `osv_git`'s records needs.
fn advisory_ids_of(value: &Value, what: &str) -> Result<Json, Error> {
    let entries = entries(&table_of(value, what)?, what)?;
    if let Some(none) = none_of(&entries, what)? {
        return Ok(none);
    }
    let mut out = Map::new();
    for (key, value) in &entries {
        let item = nested(what, key);
        let text = match key.as_str() {
            "cpe" => {
                let cpe = nonempty(value, &item)?;
                if !is_cpe_prefix(&cpe) {
                    return Err(Error::Invalid(format!("{item}: `{cpe}` is not a CPE 2.3 \
                                                       prefix like \
                                                       `cpe:2.3:a:vendor:product`")));
                }
                cpe
            }
            "osv_git" => {
                let url = nonempty(value, &item)?;
                if !is_repository_url(&url) {
                    return Err(Error::Invalid(format!("{item}: `{url}` is not a repository \
                                                       URL like \
                                                       `https://github.com/owner/repo`")));
                }
                url
            }
            "purl" => {
                let purl = nonempty(value, &item)?;
                if !is_purl(&purl) {
                    return Err(Error::Invalid(format!("{item}: `{purl}` is not a purl \
                                                       without a version, like \
                                                       `pkg:generic/zlib`")));
                }
                purl
            }
            "tag_prefix" => {
                let prefix = string_of(value, &item)?;
                if prefix.contains(char::is_whitespace) {
                    return Err(Error::Invalid(format!("{item}: `{prefix}` holds whitespace")));
                }
                prefix
            }
            _ => return Err(unknown_key(key, what)),
        };
        out.insert(key.clone(), json!(text));
    }
    if !["cpe", "osv_git", "purl"].iter()
                                  .any(|key| out.contains_key(*key))
    {
        return Err(Error::Invalid(format!("{what} names no identifier; set `cpe`, \
                                           `osv_git` or `purl`, or `none` with the reason")));
    }
    match (out.contains_key("osv_git"), out.contains_key("tag_prefix")) {
        (true, false) => {
            Err(Error::Invalid(format!("{what} sets `osv_git` without `tag_prefix`, which \
                                        spells a version as that repository's tag (`\"\"` \
                                        when the tag is the version)")))
        }
        (false, true) => {
            Err(Error::Invalid(format!("{what} sets `tag_prefix` without `osv_git`, the \
                                        repository whose tags it spells")))
        }
        _ => Ok(Json::Object(out)),
    }
}

/// `cpe:2.3:<part>:<vendor>:<product>`: the plan's `version` completes it.
fn is_cpe_prefix(text: &str) -> bool {
    let parts: Vec<&str> = text.split(':').collect();
    let named = |part: &str| {
        !part.is_empty() && part != "*" && part != "-" && !part.contains(char::is_whitespace)
    };
    matches!(parts.as_slice(),
             ["cpe", "2.3", "a" | "o" | "h", vendor, product] if named(vendor) && named(product))
}

fn is_repository_url(text: &str) -> bool {
    let rest = ["https://", "http://", "git://"].iter()
                                                .find_map(|scheme| text.strip_prefix(scheme));
    rest.is_some_and(|rest| {
            let host = rest.split('/').next().unwrap_or_default();
            !host.is_empty() && rest.len() > host.len() + 1
        })
    && !text.contains(char::is_whitespace)
}

/// `pkg:<type>/<name>`, with an optional namespace, and no `@version`.
fn is_purl(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("pkg:") else {
        return false;
    };
    let mut parts = rest.split('/');
    let kind = parts.next().unwrap_or_default();
    let names: Vec<&str> = parts.collect();
    !kind.is_empty()
    && kind.bytes()
           .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"+.-".contains(&b))
    && !names.is_empty()
    && names.iter().all(|name| !name.is_empty())
    && !text.contains(['@', '?', '#'])
    && !text.contains(char::is_whitespace)
}

/// The CVE or OSV ids the plan's own patches fix, so an advisory against its
/// upstream version is not reported for it.
fn fixes_of(value: &Value, what: &str) -> Result<Json, Error> {
    let ids = strings_of(value, what)?;
    if ids.is_empty() {
        return Err(Error::Invalid(format!("{what} is empty; leave it out")));
    }
    let mut seen = BTreeSet::new();
    for (i, id) in ids.iter().enumerate() {
        if !is_advisory_id(id) {
            return Err(Error::Invalid(format!("`{}[{}]`: `{id}` is not a CVE or OSV id \
                                               like `CVE-2024-1234` or \
                                               `GHSA-jfh8-c2jp-5v3q`",
                                              what.trim_matches('`'),
                                              i + 1)));
        }
        if !seen.insert(id) {
            return Err(Error::Invalid(format!("{what} lists `{id}` twice")));
        }
    }
    Ok(json!(ids))
}

/// `CVE-<year>-<four or more digits>`, or an OSV database's id: an upper-case
/// prefix (`GHSA`, `OSV`, `RUSTSEC`, `PYSEC`, `GO`) and its own suffix.
fn is_advisory_id(id: &str) -> bool {
    let Some((prefix, rest)) = id.split_once('-') else {
        return false;
    };
    if prefix == "CVE" {
        return rest.split_once('-').is_some_and(|(year, number)| {
                                       year.len() == 4
                                       && number.len() >= 4
                                       && year.bytes().all(|b| b.is_ascii_digit())
                                       && number.bytes().all(|b| b.is_ascii_digit())
                                   });
    }
    prefix.bytes()
          .next()
          .is_some_and(|b| b.is_ascii_uppercase())
    && prefix.bytes()
             .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    && !rest.is_empty()
    && rest.bytes()
           .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

/// A package ident, or `{:package <ident> :settings <data>}`: the settings
/// go to the scaffolding's module.
fn scaffolding_of(value: &Value, what: &str) -> Result<Json, Error> {
    match value {
        Value::String(_) => Ok(json!(ident_of(value, what)?)),
        Value::Table(_) => {
            let mut out = Map::new();
            for (key, value) in entries(&table_of(value, what)?, what)? {
                let item = match key.as_str() {
                    "package" => json!(ident_of(&value, &format!("{what}.package"))?),
                    "settings" => data_of(&value, &format!("{what}.settings"))?,
                    _ => return Err(unknown_key(&key, what)),
                };
                out.insert(key, item);
            }
            if !out.contains_key("package") {
                return Err(Error::Invalid(format!("{what} does not set `package`")));
            }
            Ok(Json::Object(out))
        }
        other => Err(wrong_type(what, "a package ident or a table", other)),
    }
}

/// A table with fixed string keys, some required.
fn record(value: &Value,
          what: &str,
          required: &[&str],
          optional: &[&str])
          -> Result<Map<String, Json>, Error> {
    let mut out = Map::new();
    for (key, value) in entries(&table_of(value, what)?, what)? {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(unknown_key(&key, what));
        }
        out.insert(key.clone(),
                   json!(nonempty(&value, &format!("{what}.{key}"))?));
    }
    for key in required {
        if !out.contains_key(*key) {
            return Err(Error::Invalid(format!("{what} does not set `{key}`")));
        }
    }
    Ok(out)
}

fn table_of(value: &Value, what: &str) -> Result<Table, Error> {
    match value {
        Value::Table(table) if table.metatable().is_some() => {
            Err(Error::Invalid(format!("{what} is a table with a metatable")))
        }
        Value::Table(table) => Ok(table.clone()),
        other => Err(wrong_type(what, "a table", other)),
    }
}

/// A table's entries, which must all have string keys, sorted by key.
fn entries(table: &Table, what: &str) -> Result<Vec<(String, Value)>, Error> {
    if table.metatable().is_some() {
        return Err(Error::Invalid(format!("{what} is a table with a metatable")));
    }
    let mut out = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (key, value) = pair.map_err(|e| Error::Invalid(format!("{what}: {e}")))?;
        match key {
            Value::String(key) => {
                let key =
                    key.to_str()
                       .map_err(|_| Error::Invalid(format!("{what} has a key that is not UTF-8")))?;
                out.push((key.to_string(), value));
            }
            other => {
                return Err(Error::Invalid(format!("{what} has a {} key, where only \
                                                   string keys are allowed",
                                                  other.type_name())));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn string_of(value: &Value, what: &str) -> Result<String, Error> {
    match value {
        Value::String(text) => {
            text.to_str()
                .map(|text| text.to_string())
                .map_err(|_| Error::Invalid(format!("{what} is not UTF-8")))
        }
        other => Err(wrong_type(what, "a string", other)),
    }
}

fn nonempty(value: &Value, what: &str) -> Result<String, Error> {
    let text = string_of(value, what)?;
    if text.trim().is_empty() {
        return Err(Error::Invalid(format!("{what} is empty")));
    }
    Ok(text)
}

/// A sequence of strings: keys 1 to n and nothing else.
fn strings_of(value: &Value, what: &str) -> Result<Vec<String>, Error> {
    let table = table_of(value, what)?;
    let length = table.raw_len();
    let mut count = 0;
    for pair in table.pairs::<Value, Value>() {
        let (key, _) = pair.map_err(|e| Error::Invalid(format!("{what}: {e}")))?;
        match key {
            Value::Integer(i) if i >= 1 && usize::try_from(i).is_ok_and(|i| i <= length) => {}
            _ => return Err(Error::Invalid(format!("{what} is not a list"))),
        }
        count += 1;
    }
    if count != length {
        return Err(Error::Invalid(format!("{what} is not a list")));
    }
    (1..=length).map(|i| {
                    let item = table.raw_get::<Value>(i)
                                    .map_err(|e| Error::Invalid(format!("{what}: {e}")))?;
                    nonempty(&item, &format!("{what}[{i}]"))
                })
                .collect()
}

fn is_name(text: &str) -> bool {
    !text.is_empty()
    && text.bytes()
           .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn name_of(value: &Value, what: &str) -> Result<String, Error> {
    let name = string_of(value, what)?;
    if !is_name(&name) {
        return Err(Error::Invalid(format!("{what} is `{name}`; names use \
                                           letters, digits, `_` and `-`")));
    }
    Ok(name)
}

fn ident_of(value: &Value, what: &str) -> Result<String, Error> {
    let ident = string_of(value, what)?;
    check_ident(&ident, what)?;
    Ok(ident)
}

/// `origin/name`, optionally followed by `/version` and `/release`.
fn check_ident(ident: &str, what: &str) -> Result<(), Error> {
    let parts: Vec<&str> = ident.split('/').collect();
    let valid = (2..=4).contains(&parts.len())
                && is_name(parts[0])
                && is_name(parts[1])
                && parts[2..].iter()
                             .all(|part| !part.is_empty() && !part.contains(char::is_whitespace));
    if valid {
        Ok(())
    } else {
        Err(Error::Invalid(format!("{what}: `{ident}` is not a package ident like `origin/name`")))
    }
}

fn check_relative(path: &str, what: &str) -> Result<(), Error> {
    let absolute = path.starts_with('/') || path.starts_with('\\') || path.contains(':');
    let escapes = path.split(['/', '\\']).any(|part| part == "..");
    if absolute || escapes {
        return Err(Error::Invalid(format!("{what}: `{path}` must be a path inside the package")));
    }
    Ok(())
}

fn wrong_type(what: &str, expected: &str, value: &Value) -> Error {
    Error::Invalid(format!("{what} is a {}, not {expected}", value.type_name()))
}
