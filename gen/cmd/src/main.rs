// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use autocxx_engine::parse_file;
use clap::{crate_authors, crate_version, App, Arg, SubCommand};
use indoc::indoc;
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let matches = App::new("autocxx-gen")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Generates C++ files from Rust files that contain include_cpp! macros")
        .long_about(indoc! {"
                Command line utility to expand the Rust 'autocxx'
                include_cpp macro. Normal usage here is to use the gen-cpp
                subcommand to generate the .cpp and .h side of the bindings,
                such that you can build them and link them to Rust code.
                You can also use this utility
                to expand the Rust code if you wish to avoid a dependency
                on a C++ include path within your Rust build process.
        "})
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input .rs file to use")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("outdir")
                .short("o")
                .long("outdir")
                .value_name("PATH")
                .help("output directory path")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("inc")
                .long("inc")
                .value_name("INCLUDE DIRS")
                .help("include path")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("gen-cpp")
                .help("Generate C++ .cpp and .h files. Normal mode of operation.")
                .arg(
                    Arg::with_name("pattern")
                        .long("pattern")
                        .value_name("PATTERN")
                        .help(".h and .cpp output pattern")
                        .default_value("gen")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("cpp-extension")
                        .long("cpp-extension")
                        .value_name("EXTENSION")
                        .default_value("cc")
                        .help("include path")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("generate-exact")
                        .long("generate-exact")
                        .value_name("NUM")
                        .help("always generate this number of .cc and .h files")
                        .takes_value(true),
                ),
        )
        .subcommand(SubCommand::with_name("gen-rs").help("Generate expanded Rust file."))
        .get_matches();
    let mut parsed_file = parse_file(matches.value_of("INPUT").unwrap())
        .expect("Unable to parse Rust file and interpret autocxx macro");
    let incs = matches.value_of("inc").unwrap_or("");
    // TODO - in future, we should provide an option to write a .d file here
    // by passing a callback into the dep_recorder parameter here.
    parsed_file
        .resolve_all(incs, None)
        .expect("Unable to resolve macro");
    let outdir: PathBuf = matches.value_of_os("outdir").unwrap().into();
    if let Some(matches) = matches.subcommand_matches("gen-cpp") {
        let pattern = matches.value_of("pattern").unwrap_or("gen");
        let cpp = matches.value_of("cpp-extension").unwrap_or("cc");
        let desired_number = matches
            .value_of("generate-exact")
            .map(|s| s.parse::<usize>().unwrap());
        let mut counter = 0usize;
        for include_cxx in parsed_file.get_autocxxes() {
            let generations = include_cxx
                .generate_h_and_cxx()
                .expect("Unable to generate header and C++ code");
            for pair in generations.0 {
                let cppname = format!("{}{}.{}", pattern, counter, cpp);
                write_to_file(&outdir, cppname, &pair.implementation);
                write_to_file(&outdir, pair.header_name, &pair.header);
                counter += 1;
            }
        }
        if let Some(desired_number) = desired_number {
            while counter < desired_number {
                write_cpp_file(
                    &outdir,
                    pattern,
                    cpp,
                    counter,
                    "// Blank C++ file generated by autocxx".as_bytes(),
                );
                counter += 1;
            }
        }
    } else if matches.subcommand_matches("gen-rs").is_some() {
        let mut ts = TokenStream::new();
        parsed_file.to_tokens(&mut ts);
        write_to_file(&outdir, "gen.rs".to_string(), ts.to_string().as_bytes());
    } else {
        panic!("Must specify a subcommand");
    }
}

fn write_cpp_file(outdir: &PathBuf, pattern: &str, cpp: &str, counter: usize, content: &[u8]) {
    let cppname = format!("{}{}.{}", pattern, counter, cpp);
    write_to_file(outdir, cppname, content);
}

fn write_to_file(dir: &PathBuf, filename: String, content: &[u8]) {
    let path = dir.join(filename);
    let mut f = File::create(&path).expect("Unable to create file");
    f.write_all(content).expect("Unable to write file");
}
