use std::{
    collections::{BTreeMap, btree_map::Entry},
    io::Read,
};

use serde::Deserialize;

use crate::{bottles::Bottles, status::Progress};

const FORMULA_API: &str = "https://formulae.brew.sh/api/formula.json";

#[derive(Debug, Deserialize)]
pub struct Formula {
    pub name: String,
    pub aliases: Vec<String>,
    pub dependencies: Vec<String>,
    pub versions: Versions,
    pub bottle: Bottles,
}

#[derive(Debug, Deserialize)]
pub struct Versions {
    pub stable: String,
    pub bottle: bool,
}

impl Formula {
    pub fn all() -> anyhow::Result<&'static [Formula]> {
        let formulae = cache!(Vec<Formula>)
            .with_file("formula.json")
            .get_or_init_json(|| {
                let progress = Progress::new();
                let progress = progress.start("Formula List".to_owned())?;

                let response = reqwest::blocking::get(FORMULA_API)?;

                let mut tracked = progress.track(response);
                let mut json = String::new();
                tracked.read_to_string(&mut json)?;

                progress.finish()?;

                Ok(json)
            })?;
        Ok(formulae)
    }

    pub fn get(name: &str) -> anyhow::Result<&'static Formula> {
        Formula::get_exact(name).or_else(|_| Formula::get_by_alias(name))
    }

    pub fn get_exact(name: &str) -> anyhow::Result<&'static Formula> {
        let formulae = Formula::all()?;
        let Ok(index) = formulae.binary_search_by_key(&name, |f| &f.name) else {
            anyhow::bail!("Unable to find formula with exact name: {name:?}");
        };
        Ok(&formulae[index])
    }

    fn get_by_alias(alias: &str) -> anyhow::Result<&'static Formula> {
        let aliases = cache!(Vec<(&str, &Formula)>).get_or_init(|| {
            let formulae = Formula::all()?;
            let mut aliases = formulae
                .iter()
                .flat_map(|f| f.aliases.iter().map(move |a| (a.as_str(), f)))
                .collect::<Vec<_>>();
            aliases.sort_by_key(|&(a, _)| a);
            Ok(aliases)
        })?;

        let Ok(index) = aliases.binary_search_by_key(&alias, |(a, _)| a) else {
            anyhow::bail!("Unable to find formula: {alias:?}");
        };
        Ok(aliases[index].1)
    }

    pub fn resolve_dependencies(
        roots: Vec<&str>,
    ) -> anyhow::Result<BTreeMap<&'static str, &'static Formula>> {
        let mut result = BTreeMap::<&str, &Formula>::new();
        let mut stack = roots;
        while let Some(name) = stack.pop() {
            let formula = Formula::get(name)?;

            if let Entry::Vacant(entry) = result.entry(formula.name.as_str()) {
                entry.insert(formula);

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
