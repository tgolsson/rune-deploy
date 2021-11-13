// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2021, Tom Solberg, all rights reserved.
// Created: 13 November 2021

/*!

*/

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

use crate::{load_manifest, RuneDependency, RuneManifest};

fn create_crate_path(path: &Path) -> Result<PathBuf> {
    crate::create_dir(path.join("crate"))
}

fn get_dependency_path(cache_path: &Path, name: &str, dep: &RuneDependency) -> PathBuf {
    // FIXME[Tsolberg]
    cache_path.to_owned()
}

fn generate_cargo_toml(
    cache_path: &Path,
    target_path: &Path,
    manifest: &crate::RuneManifest,
) -> Result<()> {
    let mut lines = vec!["[project]".to_owned()];

    lines.push(format!("name = {:?}", manifest.project.name));
    lines.push("edition = \"2021\"".to_owned());

    lines.push(format!(
        "authors = {:?}",
        manifest.project.authors.as_ref().unwrap_or(&vec![])
    ));
    lines.push(format!("version = {:?}", manifest.project.version));

    lines.push("".to_owned());
    lines.push("[dependencies]".to_owned());
    lines.push("serde_cbor = \"*\"".to_owned());
    lines.push("rune = { git = \"https://github.com/rune-rs/rune\" }".to_owned());
    lines.push("rune-modules = { git = \"https://github.com/rune-rs/rune\" }".to_owned());

    if let Some(deps) = manifest.dependencies.as_ref() {
        for (name, dep) in deps {
            let dependency_path = get_dependency_path(cache_path, name, dep);
            let (manifest, hash) = load_manifest(&dependency_path)?;
        }
    }

    std::fs::write(target_path.join("Cargo.toml"), lines.join("\n"))?;

    Ok(())
}

pub fn generate_main_rs(
    src_dir: &Path,
    manifest: &RuneManifest,
    linked_files: Vec<PathBuf>,
) -> Result<()> {
    let mut lines = vec!["/* THIS FILE IS GENERATED BY RUNE-DEPLOY */".to_owned()];
    lines.push("use rune::{Unit, Context, Vm};".to_owned());
    lines.push("use std::sync::Arc;".to_owned());
    lines.push("use rune_modules::with_config;".to_owned());

    lines.push("".to_owned());
    lines.push("pub fn main() {".to_owned());
    lines.push("    let mut context = with_config(true).unwrap();".to_owned());
    for load_file in linked_files {
        let file = PathBuf::from(load_file.file_name().unwrap());

        lines.push(format!(
            r#"    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../deps/", "{}"));"#,
            file.to_str().unwrap()
        ));

        lines.push("    let unit: Unit = serde_cbor::from_slice(bytes).unwrap();".to_owned());
	lines.push("    let unit = Arc::new(unit);".to_owned());
	lines.push("    let runtime = Arc::new(context.runtime());".to_owned());
	lines.push("    let mut vm = Vm::new(runtime.clone(), unit.clone());".to_owned());
	lines.push(r#"    vm.call(["main"], ()).unwrap();"#.to_owned());
	
    }

    lines.push("}".to_owned());

    std::fs::create_dir_all(src_dir)?;

    std::fs::write(src_dir.join("main.rs"), lines.join("\n"))?;

    Ok(())
}

pub fn precompile_rune_code(
    cache_path: &Path,
    root_dir: &Path,
    crate_dir: &Path,
    target_dir: &Path,
    manifest: &crate::RuneManifest,
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(target_dir.join("deps"))?;
    let mut main = crate::compile::precompile(
        target_dir,
        crate::compile::CrateKind::Executable,
        &manifest.project.name,
        root_dir,
    )?;

    Ok(vec![main])
}

pub fn compile(crate_dir: &Path) -> Result<()> {
    std::process::Command::new("cargo")
        .arg("run")
        .arg("--release")
        .current_dir(crate_dir)
        .spawn()?
        .wait()?;
    Ok(())
}

pub fn generate_fake_crate(
    cache_path: &Path,
    target_path: &Path,
    manifest: &crate::RuneManifest,
) -> Result<()> {
    let crate_dir = create_crate_path(&target_path)?;

    generate_cargo_toml(cache_path, &crate_dir, manifest)?;
    let linked_files = precompile_rune_code(
        cache_path,
        target_path.parent().unwrap(),
        &crate_dir,
        target_path,
        manifest,
    )
    .context("during precompilation")?;
    generate_main_rs(&crate_dir.join("src"), manifest, linked_files)?;

    compile(&crate_dir)?;

    Ok(())
}
