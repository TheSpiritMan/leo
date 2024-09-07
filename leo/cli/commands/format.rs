// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.


use super::*;

use indexmap::IndexMap;
use leo_ast::Stub;
use leo_compiler::{Compiler, CompilerOptions};
use leo_errors::{CliError, UtilError};
use leo_retriever::{Manifest, NetworkName, Retriever};
use leo_linter::Linter;
use leo_span::Symbol;
use leo_package::{build::BuildDirectory, outputs::OutputsDirectory, source::SourceDirectory};
use std::fs;
use snarkvm::prelude::CanaryV0;
use std::{
    io::Write,
    path::{Path, PathBuf},
    option::Option
};
use snarkvm::{
    package::Package,
    prelude::{MainnetV0, Network, ProgramID, TestnetV0},
};



#[derive(Parser, Debug)]
pub struct Format {}

impl Command for Format {
    type Input = ();
    type Output = ();

    fn log_span(&self) -> Span {
        tracing::span!(tracing::Level::INFO, "Leo")
    }

    fn prelude(&self, _: Context) -> Result<Self::Input> {
        Ok(())
    }

    fn apply(self, context: Context, _: Self::Input) -> Result<Self::Output> {
        handle_format(&self, context)
    }
}
fn handle_format(command: &Format, context: Context) -> Result<<Format as Command>::Output> {
    let package_path = context.dir()?;
    let home_path = context.home()?;
    let endpoint = String::from("https://api.explorer.aleo.org/v1");
    let manifest = Manifest::read_from_dir(&package_path)?;
    let program_id = ProgramID::<TestnetV0>::from_str(manifest.program())?;
    let linter = Linter::<TestnetV0>::new(program_id, endpoint, package_path, home_path)
    .map_err(|err| UtilError::failed_to_retrieve_dependencies(err, Default::default()))?;
    linter.lint()?;
    Ok(())
}

