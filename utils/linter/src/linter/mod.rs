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

use indexmap::IndexMap;
use leo_ast::Stub;
use leo_compiler::{Compiler, CompilerOptions};
use leo_errors::{CliError, UtilError};
use leo_errors::emitter::Handler;
use leo_retriever::Retriever;
use leo_span::Symbol;


use leo_package::{build::BuildDirectory, outputs::OutputsDirectory, source::SourceDirectory};
use std::fs;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

use snarkvm::{
    package::Package, 
    prelude::{Network, ProgramID, Error},
};
pub struct Linter<N: Network> {
    package_path: PathBuf,
    home_path: PathBuf,
    program_id: ProgramID<N>,
    endpoint: String,
}
impl<N: Network> Linter<N> {
    pub fn new(program_id: ProgramID<N>, endpoint: String, package_path: PathBuf, home_path: PathBuf) -> Result<Self, UtilError> {
        Ok(Self {
            package_path: package_path.clone(),
            endpoint: endpoint.clone(),
            program_id: program_id.clone(),
            home_path: home_path.clone(),
        })
    }
    
    pub fn lint(&self) -> Result<(), Error> {
        let build_directory = self.package_path.join("build");
        if build_directory.exists() {
            std::fs::remove_dir_all(&build_directory).map_err(CliError::build_error)?;
        }
        Package::create(&build_directory, &self.program_id).map_err(CliError::build_error)?;
        let handler = Handler::default();
        let main_sym = Symbol::intern(&self.program_id.name().to_string());
        let mut retriever =  Retriever::<N>::new(
            main_sym, 
            &self.package_path, 
            &self.home_path, 
            self.endpoint.clone()
        )    
        .map_err(|err| UtilError::failed_to_retrieve_dependencies(err, Default::default()))?;
        let mut local_dependencies = retriever.retrieve().map_err(|err| UtilError::failed_to_retrieve_dependencies(err, Default::default()))?;
        local_dependencies.push(main_sym);
        let recursive_build = true;
        for dependency in local_dependencies.into_iter() {
            let (local_path, stubs) = retriever.prepare_local(dependency).map_err(CliError::build_error)?;
            let local_outputs_directory = OutputsDirectory::create(&local_path).map_err(CliError::build_error)?;
            let local_build_directory = BuildDirectory::create(&local_path).map_err(CliError::build_error)?;
            let local_source_files = SourceDirectory::files(&local_path).map_err(CliError::build_error)?;
            SourceDirectory::check_files(&local_source_files).map_err(CliError::build_error)?;
            for file_path in local_source_files.clone() {
                compile_leo_file(
                    file_path,
                    &ProgramID::<N>::try_from(format!("{}.aleo", dependency))
                        .map_err(|_| UtilError::snarkvm_error_building_program_id(Default::default()))?,
                    &local_outputs_directory,
                    &local_build_directory,
                    &handler,
                    stubs.clone(),
                )?;
            }
            fs::remove_dir_all(local_build_directory.to_str().unwrap()).expect("Failed to remove build directory");
            fs::remove_dir_all(local_outputs_directory.to_str().unwrap()).expect("Failed to remove outputs directory");
            for file_path in local_source_files.clone() {
                let code = fs::read_to_string(file_path.to_str().unwrap()).expect("Failed to read file");
                let normalized_code = normalize_code(&code);
                fs::write(file_path.to_str().unwrap(), normalized_code).expect("Failed to write file");
            }
        }
        Ok(())
    }

}

#[allow(clippy::too_many_arguments)]
fn compile_leo_file<N: Network>(
    file_path: PathBuf,
    program_id: &ProgramID<N>,
    outputs: &Path,
    build: &Path,
    handler: &Handler,
    stubs: IndexMap<Symbol, Stub>,
)  -> Result<(), Error> {
    let program_name = program_id.name().to_string();
    let mut compiler = Compiler::<N>::new(
        program_name.clone(),
        program_id.network().to_string(),
        handler,
        file_path.clone(),
        outputs.to_path_buf(),
        Some(CompilerOptions::default()),
        stubs,
    );
    compiler.compile()?;
    Ok(())
}
fn normalize_code(code: &str) -> String {
    let mut result = String::new();
    let mut indent_level = 0;
    let mut inside_brace = false;
    let mut inside_comment = false;

    let mut chars = code.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                
                result.push(c);
                result.push('\n');
                indent_level += 1;
                add_indentation(&mut result, indent_level);
                inside_brace = true;
            }
            '}' => {
                if inside_brace {
                    indent_level -= 1;
                    result.push('\n');
                    add_indentation(&mut result, indent_level);
                    result.push(c);
                    result.push('\n');
                    add_indentation(&mut result, indent_level);
                    inside_brace = indent_level > 0;
                }
            }
            ';' => {
                result.push(c);
                result.push('\n');
                add_indentation(&mut result, indent_level);
                inside_comment = false;
            }
            ':' => {
                result.push(c);
                result.push(' '); // Add space after colon for readability
            }
            '(' => {
                result.push(c);
                result.push(' ');
            }
            ')' => {
                result.push(' ');
                result.push(c);
            }
            '/' => {
                if chars.peek() == Some(&'/') {
                    inside_comment = true;
                    result.push(c);
                    result.push(chars.next().unwrap()); // Skip the next '/'
                    
                } else {
                    result.push(c);
                }
            }
            '\n' => {
                if inside_comment {
                    inside_comment = false;
                    result.push('\n');
                    add_indentation(&mut result, indent_level);
                }
                // Ignore explicit newlines in the input
                continue;
            }
            ' ' => {
                // Skip multiple spaces
                if !result.ends_with(' ') {
                    result.push(c);
                }
            }
            _ => {
                if inside_comment {
                    if c == '\n' {
                        inside_comment = false;
                    }
                }
                result.push(c);
            }
        }
    }

    // Remove any trailing newlines or spaces
    result.trim_end().to_string()
}
fn add_indentation(result: &mut String, indent_level: usize) {
    for _ in 0..indent_level {
        result.push_str("    "); // 4 spaces for indentation
    }
}
