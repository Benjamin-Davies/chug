use std::collections::{BTreeMap, BTreeSet};

use ptree::{TreeBuilder, print_tree};

use crate::db::models::{Dependency, DownloadedBottle};

pub fn list_bottles() -> anyhow::Result<()> {
    let bottles = DownloadedBottle::get_all()?;

    for bottle in bottles {
        println!("{} {}", bottle.name(), bottle.version());
    }

    Ok(())
}

pub fn display_tree() -> anyhow::Result<()> {
    let bottles = DownloadedBottle::get_all()?;
    let dependencies = Dependency::get_all()?;

    let bottle_map = bottles
        .into_iter()
        .map(|b| (b.id(), b))
        .collect::<BTreeMap<_, _>>();
    let mut dependency_map = BTreeMap::new();
    for dependency in dependencies {
        dependency_map
            .entry(dependency.dependent_id())
            .or_insert_with(Vec::new)
            .push(dependency.dependency_id());
    }

    let get_dependencies = |id: Option<i32>| {
        let mut v = dependency_map
            .get(&id)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|id| &bottle_map[id])
            .collect::<Vec<_>>();
        v.sort_by_key(|b| b.name());
        v
    };

    let mut builder = TreeBuilder::new("Installed bottles:".to_owned());
    let mut stack = vec![get_dependencies(None).into_iter()];
    let mut processed = BTreeSet::new();
    while let Some(children) = stack.last_mut() {
        if let Some(child) = children.next() {
            if processed.insert(child.id()) {
                builder.begin_child(format!("{} {}", child.name(), child.version()));
                stack.push(get_dependencies(Some(child.id())).into_iter());
            } else {
                builder.add_empty_child(format!("{} {} (*)", child.name(), child.version()));
            }
        } else {
            stack.pop();
            if !stack.is_empty() {
                builder.end_child();
            }
        }
    }

    let tree = builder.build();
    print_tree(&tree)?;

    Ok(())
}
