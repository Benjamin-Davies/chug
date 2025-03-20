use std::{collections::BTreeMap, sync::OnceLock};

use anyhow::Context;
use serde::Deserialize;

use crate::{dirs::cache_dir, disk_cache};

const FORMULA_API: &str = "https://formulae.brew.sh/api/formula.json";

#[derive(Debug, Deserialize)]
pub struct Formula {
    pub name: String,
    pub aliases: Vec<String>,
    pub dependencies: Vec<String>,
    pub versions: Versions,
}

#[derive(Debug, Deserialize)]
pub struct Versions {
    pub stable: String,
    pub bottle: bool,
}

impl Formula {
    pub fn all() -> &'static [Formula] {
        static CACHE: OnceLock<Vec<Formula>> = OnceLock::new();
        CACHE.get_or_init(|| {
            let disk_cache_path = cache_dir().join("formula.json");
            if let Ok(formulae) = disk_cache::load_json(&disk_cache_path) {
                return formulae;
            }

            println!("Downloading fresh formula list...");
            let json = reqwest::blocking::get(FORMULA_API).unwrap().text().unwrap();
            let formulae = serde_json::from_str(&json).unwrap();

            disk_cache::store(&disk_cache_path, &json).unwrap();

            formulae
        })
    }

    pub fn get(name: &str) -> Option<&'static Formula> {
        Formula::get_exact(name).or_else(|| Formula::get_by_alias(name))
    }

    pub fn get_exact(name: &str) -> Option<&'static Formula> {
        let formulae = Formula::all();
        formulae
            .binary_search_by_key(&name, |f| &f.name)
            .ok()
            .map(|i| &formulae[i])
    }

    pub fn get_by_alias(alias: &str) -> Option<&'static Formula> {
        static CACHE: OnceLock<Vec<(&str, &Formula)>> = OnceLock::new();
        let aliases = CACHE.get_or_init(|| {
            let formulae = Formula::all();
            let mut aliases = formulae
                .iter()
                .flat_map(|f| f.aliases.iter().map(move |a| (a.as_str(), f)))
                .collect::<Vec<(&str, &Formula)>>();
            aliases.sort_by_key(|&(a, _)| a);
            aliases
        });
        aliases
            .binary_search_by_key(&alias, |(a, _)| a)
            .ok()
            .map(|i| aliases[i].1)
    }

    pub fn resolve_dependencies(
        roots: Vec<&str>,
    ) -> anyhow::Result<BTreeMap<&'static str, &'static Formula>> {
        let mut result = BTreeMap::new();
        let mut stack = roots;
        while let Some(name) = stack.pop() {
            let formula =
                Formula::get(name).with_context(|| format!("Unable to find formula: {name:?}"))?;

            if !result.contains_key(&formula.name.as_str()) {
                result.insert(formula.name.as_str(), formula);

                for dependency in &formula.dependencies {
                    if !result.contains_key(&dependency.as_str()) {
                        stack.push(dependency);
                    }
                }
            }
        }

        Ok(result)
    }
}
