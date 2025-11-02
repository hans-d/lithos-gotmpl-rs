// SPDX-License-Identifier: Apache-2.0 OR MIT
use lithos_gotmpl_core::{FunctionSource, Template};
use lithos_sprig::{install_sprig_functions, sprig_functions};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct JsonFixture {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    function: Option<String>,
    #[serde(default)]
    template: Option<String>,
}

#[test]
fn every_registered_helper_has_fixture() {
    let sprig_registry = {
        let mut builder = lithos_gotmpl_core::FunctionRegistryBuilder::new();
        install_sprig_functions(&mut builder);
        builder.build()
    };
    let registered: BTreeSet<String> = sprig_registry.function_names().into_iter().collect();

    let covered = collect_fixture_helpers(&sprig_registry, test_cases_dir());
    let missing: Vec<String> = registered
        .difference(&covered)
        .cloned()
        .collect();

    assert!(
        missing.is_empty(),
        "fixtures missing for helpers: {:?}",
        missing
    );
}

fn collect_fixture_helpers(registry: &lithos_gotmpl_core::FunctionRegistry, root: PathBuf) -> BTreeSet<String> {
    let mut covered = BTreeSet::new();
    let combined_registry = sprig_functions();

    let json_path = root.join("lithos-sprig.json");
    if let Ok(file) = fs::File::open(&json_path) {
        let reader = std::io::BufReader::new(file);
        let fixtures: Vec<JsonFixture> =
            serde_json::from_reader(reader).expect("failed to parse lithos-sprig.json");
        for fixture in fixtures {
            if let Some(name) = fixture.function {
                if registry.get(&name).is_some() {
                    covered.insert(name.clone());
                }
            }
            if let Some(template_src) = fixture.template {
                collect_from_template(
                    registry,
                    &combined_registry,
                    fixture.name.as_deref().unwrap_or("fixture"),
                    &template_src,
                    &mut covered,
                );
            }
        }
    }

    let sprig_dir = root.join("sprig");
    if sprig_dir.exists() {
        for entry in fs::read_dir(sprig_dir).expect("failed to read sprig fixture directory") {
            let entry = entry.expect("failed to read directory entry");
            if entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false) {
                let template_path = entry.path().join("input.tmpl");
                if template_path.exists() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let source = fs::read_to_string(&template_path)
                        .unwrap_or_else(|err| panic!("failed to read {}: {err}", template_path.display()));
                    collect_from_template(registry, &combined_registry, &name, &source, &mut covered);
                }
            }
        }
    }

    covered
}

fn collect_from_template(
    sprig_registry: &lithos_gotmpl_core::FunctionRegistry,
    combined_registry: &lithos_gotmpl_core::FunctionRegistry,
    name: &str,
    source: &str,
    covered: &mut BTreeSet<String>,
) {
    if source.trim().is_empty() {
        return;
    }

    let template = Template::parse_with_functions(name, source, combined_registry.clone())
        .unwrap_or_else(|err| panic!("failed to parse template {name}: {err}"));
    let analysis = template.analyze();
    for call in analysis.functions {
        if matches!(call.source, FunctionSource::Registered) && sprig_registry.get(&call.name).is_some() {
            covered.insert(call.name);
        }
    }
}

fn test_cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-cases")
}
