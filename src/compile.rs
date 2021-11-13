// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2021, Tom Solberg, all rights reserved.
// Created: 13 November 2021

/*!

*/

use anyhow::{Context, Result};
use rune::termcolor::StandardStream;
use std::io::Write;
use std::path::{Path, PathBuf};

use rune::compile::FileSourceLoader;
use rune::{Diagnostics, Options, Source, Sources};

pub(crate) enum CrateKind {
    Library,
    Executable,
}

/// Load context and code for a given path
pub(crate) fn precompile(
    target_dir: &Path,
    crate_kind: CrateKind,
    crate_name: &str,
    crate_path: &Path,
) -> Result<PathBuf> {
    let bytecode_path = target_dir
        .join("deps")
        .join(crate_name)
        .with_extension("rnc");

    let context = rune_modules::with_config(true)?;
    let srcpath = crate_path.join("src").join(match crate_kind {
        CrateKind::Library => "lib.rn",
        CrateKind::Executable => "main.rn",
    });
    let source = Source::from_path(&srcpath)
        .with_context(|| format!("reading file: {}", srcpath.display()))?;

    let mut sources = Sources::new();
    sources.insert(source);

    Let mut diagnostics = Diagnostics::new();

    let mut source_loader = FileSourceLoader::new();
    let mut options = Options::default();

    options.link_checks(false);
    
    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_options(&options)
        .with_source_loader(&mut source_loader)
        .build();

    let mut out = StandardStream::stdout(rune::termcolor::ColorChoice::Always);
    diagnostics.emit(&mut out, &sources)?;

    let unit = match result {
        Ok(unit) => unit,
        Err(err) => Err(err)?,
    };

    eprintln!("{:#?}", unit);
    log::trace!("serializing cache: {}", bytecode_path.display());
     // let f = std::fs::File::create(&bytecode_path)
    //     .with_context(|| format!("when creating output path: {:#?}", bytecode_path))?;
    // bincode::serialize_into(&f, &unit)?;

    // let f = std::fs::File::open(&bytecode_path)
    //     .with_context(|| format!("when reading output path: {:#?}", bytecode_path))?;
    // let unit: rune::Unit = bincode::deserialize_from(f)?	;

    let f = std::fs::File::create(&bytecode_path)
        .with_context(|| format!("when creating output path: {:#?}", bytecode_path))?;
    serde_cbor::to_writer(&f, &unit)?;

    let f = std::fs::File::open(&bytecode_path)
        .with_context(|| format!("when reading output path: {:#?}", bytecode_path))?;
    let unit: rune::Unit = serde_cbor::from_reader(f)?	;
    Ok(bytecode_path)
}
