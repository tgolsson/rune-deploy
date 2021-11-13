// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2021, Tom Solberg, all rights reserved.
// Created: 13 November 2021

// The contents of this file are a reconstruction of the Cargo.lock format

/*!

 */

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Dependency {
    name: String,
    version: String,
    source: Option<String>,
    checksum: Option<String>,
    dependencies: Option<Vec<String>>,
}


#[derive(Serialize, Deserialize)]
struct LockData {
    version: Option<u32>,
    package: Option<Vec<Dependency>>,
}
