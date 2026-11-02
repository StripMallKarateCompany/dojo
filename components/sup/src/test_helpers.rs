// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
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

use std::{fs::{self,
               File},
          io::Read,
          path::{Path,
                 PathBuf}};

use serde_json;

/// What validating JSON against a schema found.
pub struct Validation {
    pub errors: Vec<String>,
}

impl Validation {
    pub fn is_valid(&self) -> bool { self.errors.is_empty() }
}

/// Asserts that `json_string` is valid according to the specified JSON schema.
///
/// In the case of invalid input, all validation errors are neatly
/// rendered, along with a pretty-printed formatting of the
/// offending JSON.
pub fn assert_valid(json_string: &str, schema: &str) {
    let result = validate_string(json_string, schema);
    if !result.is_valid() {
        let error_string = result.errors
                                 .into_iter()
                                 .map(|x| format!("  {}", x))
                                 .collect::<Vec<String>>()
                                 .join("\n");
        let json: serde_json::Value =
            serde_json::from_str(json_string).expect("JSON should parse if we get this far");
        let pretty_json = serde_json::to_string_pretty(&json).expect("JSON values serialize");
        assert!(
                false,
                r#"
JSON does not validate!
Errors:
{}

JSON:
{}
"#,
                error_string, pretty_json
        );
    }
}

/// Compares the incoming JSON string against the JSON schema
/// and returns what validation found.
///
/// In general, you should prefer using `assert_valid` directly.
pub fn validate_string(input: &str, schema: &str) -> Validation {
    let doc_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("doc");
    // The schemas refer to each other by file name, so each is registered under one base URI
    // and the schema compiled from there. The domain is reserved and never resolves, and
    // offline validation fails on any reference the registry does not hold.
    let mut registry = jsonschema::Registry::new();
    for entry in fs::read_dir(&doc_dir).expect("could not list the schema directory") {
        let path = entry.expect("could not read the schema directory").path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let name = path.file_name()
                           .and_then(|name| name.to_str())
                           .expect("schema file names are UTF-8");
            registry = registry.add(format!("{}{}", SCHEMA_BASE_URI, name), read_schema(&path))
                               .expect("could not register the schema");
        }
    }
    let registry = registry.prepare()
                           .expect("could not resolve the schemas' references");
    let parsed_schema = read_schema(&doc_dir.join(schema));
    // Unknown keywords are ignored, which allows the `$comment` keyword, as well as our own
    // `$deprecated` and `$since` keywords.
    let validator = jsonschema::options().offline()
                                         .with_base_uri(format!("{}{}", SCHEMA_BASE_URI, schema))
                                         .with_registry(&registry)
                                         .build(&parsed_schema)
                                         .expect("Could not compile the schema");

    let input_json: serde_json::Value =
        serde_json::from_str(input).expect("Could not parse input as JSON");
    Validation { errors: validator.iter_errors(&input_json)
                                  .map(|e| format!("{} at {}", e, e.instance_path()))
                                  .collect(), }
}

/// Where the schemas in `doc` are registered for resolving references between them.
const SCHEMA_BASE_URI: &str = "https://schemas.dojo.invalid/sup/";

fn read_schema(path: &Path) -> serde_json::Value {
    let mut raw_schema = String::new();
    File::open(path).and_then(|mut file| file.read_to_string(&mut raw_schema))
                    .unwrap_or_else(|e| panic!("could not read schema {}: {}", path.display(), e));
    serde_json::from_str(&raw_schema).unwrap_or_else(|e| {
                                         panic!("could not parse schema {} as JSON: {}",
                                                path.display(),
                                                e)
                                     })
}
