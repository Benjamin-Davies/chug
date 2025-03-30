use std::collections::{BTreeMap, BTreeSet};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    db::models::{Dependency, DownloadedBottle},
    formulae::Formula,
};

#[derive(Debug)]
pub struct BottleForestSnapshot {
    bottles: BTreeMap<i32, DownloadedBottle>,
    dependencies: Vec<Dependency>,
}

#[derive(Debug)]
pub struct ActionBuilder<'a> {
    snapshot: &'a BottleForestSnapshot,
    bottles: BTreeSet<BottleRef<'a>>,
    dependencies: BTreeSet<(Option<BottleRef<'a>>, BottleRef<'a>)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct BottleRef<'a> {
    name: &'a str,
    version: &'a str,
}

impl BottleForestSnapshot {
    pub fn new() -> anyhow::Result<Self> {
        let bottles = DownloadedBottle::get_all()?
            .into_iter()
            .map(|b| (b.id(), b))
            .collect();
        let dependencies = Dependency::get_all()?;

        Ok(Self {
            bottles,
            dependencies,
        })
    }
}

impl<'a> ActionBuilder<'a> {
    pub fn new(snapshot: &'a BottleForestSnapshot) -> Self {
        let bottles = snapshot.bottles.values().map(BottleRef::from).collect();
        let dependencies = snapshot
            .dependencies
            .iter()
            .map(|dep| {
                (
                    dep.dependent_id()
                        .map(|id| snapshot.bottles.get(&id).unwrap().into()),
                    snapshot.bottles.get(&dep.dependency_id()).unwrap().into(),
                )
            })
            .collect();

        Self {
            snapshot,
            bottles,
            dependencies,
        }
    }

    pub fn add(mut self, bottles: &[String]) -> anyhow::Result<Self> {
        for name in bottles {
            let formula = Formula::get(name)?;
            self.bottles.insert(formula.into());
            self.dependencies.insert((None, formula.into()));
        }

        Ok(self)
    }

    pub fn remove_all(mut self) -> Self {
        self.bottles.clear();

        self
    }

    pub fn remove(mut self, bottles: &'a [String]) -> anyhow::Result<Self> {
        for name in bottles {
            let bottles_with_name = self
                .bottles
                .range(BottleRef { name, version: "" }..)
                .take_while(|b| b.name == name)
                .copied()
                .collect::<Vec<BottleRef<'a>>>();
            anyhow::ensure!(
                !bottles_with_name.is_empty(),
                "Could not remove {name} as it is not installed"
            );

            for bottle in bottles_with_name {
                self.bottles.remove(&bottle);
            }
        }

        Ok(self)
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        self.fix_dependencies()?;

        let (to_add, to_remove) = diff_bottles(
            &self
                .snapshot
                .bottles
                .values()
                .map(BottleRef::from)
                .collect(),
            &self.bottles,
        );

        anyhow::ensure!(
            !to_add.is_empty() || !to_remove.is_empty(),
            "No bottles to add or remove",
        );

        // Add new bottles
        let downloaded_bottles = to_add
            .par_iter()
            .map(|bottle_ref| {
                let formula = Formula::get_exact(bottle_ref.name)?;
                anyhow::ensure!(
                    formula.versions.stable == bottle_ref.version,
                    "Attempted to install an unavailable version of {}",
                    bottle_ref.name,
                );
                anyhow::ensure!(
                    formula.versions.bottle,
                    "Formula {:?} does not have a corresponding bottle",
                    formula.name,
                );

                let bottle = formula.download_bottle()?;

                Ok(bottle)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        downloaded_bottles
            .par_iter()
            .map(|bottle| {
                bottle.link()?;

                Ok(())
            })
            .collect::<anyhow::Result<Vec<()>>>()?;

        // Save new dependencies to the DB
        let bottles_by_ref = self
            .snapshot
            .bottles
            .values()
            .chain(&downloaded_bottles)
            .map(|b| (BottleRef::from(b), b))
            .collect::<BTreeMap<_, _>>();

        Dependency::replace_all(
            self.dependencies
                .iter()
                .map(|(a, b)| (a.map(|a| bottles_by_ref[&a]), bottles_by_ref[&b])),
        )?;

        // Remove old bottles
        to_remove
            .par_iter()
            .map(|bottle_ref| {
                let bottle = bottles_by_ref[&bottle_ref];
                bottle.unlink()?;
                bottle.remove()?;

                Ok(())
            })
            .collect::<anyhow::Result<Vec<()>>>()?;

        Ok(())
    }

    // HACK: Technically `name` does not need to live for 'a, but I can't figure out how to express that
    fn get_bottle(&self, name: &'a str) -> Option<BottleRef<'a>> {
        self.bottles
            .range(BottleRef { name, version: "" }..)
            .take_while(|b| b.name == name)
            .cloned()
            .next()
    }

    fn get_dependencies(&self, bottle_ref: BottleRef<'a>) -> impl Iterator<Item = BottleRef<'a>> {
        self.dependencies
            .range(
                (
                    Some(bottle_ref),
                    BottleRef {
                        name: "",
                        version: "",
                    },
                )..,
            )
            .take_while(move |(a, _)| a == &Some(bottle_ref))
            .map(|&(_, b)| b)
    }

    fn fix_dependencies(&mut self) -> anyhow::Result<()> {
        self.add_dependencies()?;
        self.remove_orphans();
        Ok(())
    }

    fn add_dependencies(&mut self) -> Result<(), anyhow::Error> {
        let mut stack = Vec::new();
        for bottle in self.bottles.iter() {
            let Ok(formula) = Formula::get_exact(bottle.name) else {
                continue;
            };
            if formula.versions.stable != bottle.version {
                continue;
            }
            stack.push(formula);
        }

        while let Some(formula) = stack.pop() {
            let bottle_ref = BottleRef::from(formula);
            for dependency_name in &formula.dependencies {
                if let Some(dependency_ref) = self.get_bottle(dependency_name) {
                    self.dependencies.insert((Some(bottle_ref), dependency_ref));
                    continue;
                }

                let dependency = Formula::get_exact(dependency_name)?;
                let dependency_ref = BottleRef::from(dependency);
                self.bottles.insert(dependency_ref);
                self.dependencies.insert((Some(bottle_ref), dependency_ref));
                stack.push(dependency);
            }
        }

        Ok(())
    }

    fn remove_orphans(&mut self) {
        let mut ref_counts = self
            .bottles
            .iter()
            .map(|&b| (b, 0))
            .collect::<BTreeMap<_, _>>();

        self.dependencies.retain(|&(a, b)| {
            if let Some(bottle) = a {
                if !self.bottles.contains(&bottle) {
                    return false;
                }
            }
            if !self.bottles.contains(&b) {
                return false;
            }

            let ref_count = ref_counts.get_mut(&b).unwrap();
            *ref_count += 1;

            true
        });

        let mut stack = Vec::new();
        for bottle in self.bottles.iter() {
            if ref_counts[bottle] == 0 {
                stack.push(*bottle);
            }
        }
        while let Some(bottle) = stack.pop() {
            self.bottles.remove(&bottle);

            for dependency in self.get_dependencies(bottle).collect::<Vec<_>>() {
                self.dependencies.remove(&(Some(bottle), dependency));

                let ref_count = ref_counts.get_mut(&dependency).unwrap();
                *ref_count -= 1;
                if *ref_count == 0 {
                    stack.push(dependency);
                }
            }
        }
    }
}

fn diff_bottles<'a>(
    before: &BTreeSet<BottleRef<'a>>,
    after: &BTreeSet<BottleRef<'a>>,
) -> (BTreeSet<BottleRef<'a>>, BTreeSet<BottleRef<'a>>) {
    let added = after.difference(before).cloned().collect();
    let removed = before.difference(after).cloned().collect();
    (added, removed)
}

impl<'a> From<&'a DownloadedBottle> for BottleRef<'a> {
    fn from(bottle: &'a DownloadedBottle) -> Self {
        Self {
            name: bottle.name(),
            version: bottle.version(),
        }
    }
}

impl<'a> From<&'a Formula> for BottleRef<'a> {
    fn from(formula: &'a Formula) -> Self {
        Self {
            name: &formula.name,
            version: &formula.versions.stable,
        }
    }
}
