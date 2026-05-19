// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod analyze;
mod app_cmd;
mod bin_cmd;
mod bunpp_cmd;
mod cli;
mod commands;
mod linux_cmd;
mod patch;
mod rust_cmd;
#[cfg(test)]
mod schema_test;
mod subprocess;
mod wasm_cmd;
mod win32_cmd;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::dispatch::run_from_args() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("n2b a échoué : {err:?}");
            ExitCode::from(2)
        }
    }
}
