mod compile;
mod faux_crate;
mod lockfile;

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher, SipHasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use faux_crate::generate_fake_crate;
// TODO[Tsolberg] -- once stabilized swithch to thiserror.
use serde::{de, Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use structopt::StructOpt;

// The manifest format is largely cribbed from Cargo, to help
// discovery and familiarity!

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
enum RuneKind {
    /// A rune crate containing only Rust modules
    Rust,

    /// A rune crate containing only Rune code
    Rune,

    /// The crate contains a mix of Rune or Rust code
    Mixed,
}

impl Default for RuneKind {
    fn default() -> Self {
        RuneKind::Rune
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct RuneProject {
    name: String,
    version: String,
    #[serde(default)]
    kind: RuneKind,
    authors: Option<Vec<String>>,
    description: Option<String>,
    homepage: Option<String>,
    documentation: Option<String>,
    keywords: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    license: Option<String>,
    license_file: Option<String>,
    repository: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct RuneManifest {
    project: RuneProject,
    dependencies: Option<HashMap<String, RuneDependency>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum RuneDependency<P = String> {
    /// In the simple format, only a version is specified, eg.
    /// `package = "<version>"`
    Simple(String),
    /// The simple format is equivalent to a detailed dependency
    /// specifying only a version, eg.
    /// `package = { version = "<version>" }`
    Detailed(DetailedRuneDependency<P>),
}

impl<'de, P: Deserialize<'de>> de::Deserialize<'de> for RuneDependency<P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct RuneDependencyVisitor<P>(PhantomData<P>);

        impl<'de, P: Deserialize<'de>> de::Visitor<'de> for RuneDependencyVisitor<P> {
            type Value = RuneDependency<P>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(
                    "a version string like \"0.9.8\" or a \
                     detailed dependency like { version = \"0.9.8\" }",
                )
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(RuneDependency::Simple(s.to_owned()))
            }

            fn visit_map<V>(self, map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mvd = de::value::MapAccessDeserializer::new(map);
                DetailedRuneDependency::deserialize(mvd).map(RuneDependency::Detailed)
            }
        }

        deserializer.deserialize_any(RuneDependencyVisitor(PhantomData))
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct DetailedRuneDependency<P = String> {
    version: Option<String>,
    registry: Option<String>,
    /// The URL of the `registry` field.
    /// This is an internal implementation detail. When Cargo creates a
    /// package, it replaces `registry` with `registry-index` so that the
    /// manifest contains the correct URL. All users won't have the same
    /// registry names configured, so Cargo can't rely on just the name for
    /// crates published by other users.
    registry_index: Option<String>,
    // `path` is relative to the file it appears in. If that's a `Cargo.rune`, it'll be relative to
    // that RUNE file, and if it's a `.cargo/config` file, it'll be relative to that file.
    path: Option<P>,
    git: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
    features: Option<Vec<String>>,
    optional: Option<bool>,
    default_features: Option<bool>,
    #[serde(rename = "default_features")]
    default_features2: Option<bool>,
    package: Option<String>,
    public: Option<bool>,
}

// Explicit implementation so we avoid pulling in P: Default
impl<P> Default for DetailedRuneDependency<P> {
    fn default() -> Self {
        Self {
            version: Default::default(),
            registry: Default::default(),
            registry_index: Default::default(),
            path: Default::default(),
            git: Default::default(),
            branch: Default::default(),
            tag: Default::default(),
            rev: Default::default(),
            features: Default::default(),
            optional: Default::default(),
            default_features: Default::default(),
            default_features2: Default::default(),
            package: Default::default(),
            public: Default::default(),
        }
    }
}
#[derive(StructOpt, Debug, Clone)]
struct DeployArgs {
    path: Option<String>,
}

fn load_manifest(path: &Path) -> Result<(RuneManifest, u64)> {
    let contents = std::fs::read_to_string(path).context("when loading manifest")?;
    let manifest: RuneManifest = toml::from_str(&contents).context("when parsing manifest")?;

    let mut hasher = DefaultHasher::default();
    contents.hash(&mut hasher);

    Ok((manifest, hasher.finish()))
}

fn get_cache_root() -> PathBuf {
    dirs::cache_dir().unwrap().join(".rune")
}

pub(crate) fn create_dir(path: PathBuf) -> Result<PathBuf> {
    if path.exists() {
        anyhow::ensure!(path.is_dir(), "output dir is a file");
        Ok(path)
    } else {
        std::fs::create_dir(&path)?;
        Ok(path)
    }
}

fn create_output_dir(path: &Path) -> Result<PathBuf> {
    let path = path.join("target");
    create_dir(path)
}

fn download_dependencies(
    cache_root: &Path,
    dependencies: &HashMap<String, RuneDependency>,
) -> Result<()> {
    for (name, spec) in dependencies {
        match spec {
            RuneDependency::Simple(name) => {
                // TODO[Lookup]
            }
            RuneDependency::Detailed(_) => todo!(),
        }
    }

    Ok(())
}

fn main() {
    let args: DeployArgs = DeployArgs::from_args();

    let path = match args.path {
        Some(path) => std::path::PathBuf::from(path),
        None => std::env::current_dir().unwrap(),
    };

    let manifest_path = path.join("Rune.toml");
    let (manifest, hash) = load_manifest(&manifest_path).expect("a manifest");

    let cache_root = get_cache_root();
    if let Some(deps) = manifest.dependencies.as_ref() {
        download_dependencies(&cache_root.join("src"), deps)
            .expect("when downloading dependencies");
    }

    let output_dir = create_output_dir(&path).expect("a valid output dir");

    generate_fake_crate(&cache_root, &output_dir, &manifest).expect("faux crate generation failed");
}
