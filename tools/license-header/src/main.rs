// Copyright 2025 Rigatoni Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Apache 2.0 License Header Management Tool
#[derive(Parser)]
#[command(name = "license-header")]
#[command(author = "Rigatoni Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Apply, check, or remove Apache 2.0 license headers from Rust source files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply license headers to Rust files
    Apply {
        /// Directory or file to process
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Header template file
        #[arg(short = 't', long, default_value = "HEADER")]
        template: PathBuf,

        /// Dry run - show what would be changed without modifying files
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Force overwrite existing headers
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Check if files have correct license headers
    Check {
        /// Directory or file to check
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Header template file
        #[arg(short = 't', long, default_value = "HEADER")]
        template: PathBuf,
    },

    /// Remove license headers from Rust files
    Remove {
        /// Directory or file to process
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Dry run - show what would be changed without modifying files
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Apply {
            path,
            template,
            dry_run,
            force,
        } => apply_headers(&path, &template, dry_run, force),
        Commands::Check { path, template } => check_headers(&path, &template),
        Commands::Remove { path, dry_run } => remove_headers(&path, dry_run),
    }
}

fn apply_headers(
    path: &Path,
    template_path: &Path,
    dry_run: bool,
    force: bool,
) -> io::Result<()> {
    let header = fs::read_to_string(template_path)?;
    let header = header.trim();

    let files = collect_rust_files(path)?;
    let mut modified = 0;
    let mut skipped = 0;

    for file_path in files {
        let content = fs::read_to_string(&file_path)?;

        if has_license_header(&content) && !force {
            println!("⏭️  Skipped (has header): {}", file_path.display());
            skipped += 1;
            continue;
        }

        let new_content = if has_license_header(&content) && force {
            // Remove old header and add new one
            let content_without_header = remove_license_header(&content);
            format!("{}\n\n{}", header, content_without_header.trim_start())
        } else {
            // Add header to file without one
            format!("{}\n\n{}", header, content.trim_start())
        };

        if dry_run {
            println!("🔍 Would modify: {}", file_path.display());
        } else {
            fs::write(&file_path, new_content)?;
            println!("✅ Modified: {}", file_path.display());
        }
        modified += 1;
    }

    println!("\n📊 Summary:");
    println!("   Modified: {}", modified);
    println!("   Skipped:  {}", skipped);
    if dry_run {
        println!("   (Dry run - no files were actually modified)");
    }

    Ok(())
}

fn check_headers(path: &Path, template_path: &Path) -> io::Result<()> {
    let expected_header = fs::read_to_string(template_path)?;
    let expected_header = expected_header.trim();

    let files = collect_rust_files(path)?;
    let mut missing = Vec::new();
    let mut present = Vec::new();

    for file_path in files {
        let content = fs::read_to_string(&file_path)?;

        if content.trim_start().starts_with("// Copyright") {
            println!("✅ Has header: {}", file_path.display());
            present.push(file_path);
        } else {
            println!("❌ Missing header: {}", file_path.display());
            missing.push(file_path);
        }
    }

    println!("\n📊 Summary:");
    println!("   With headers:    {}", present.len());
    println!("   Without headers: {}", missing.len());

    if !missing.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

fn remove_headers(path: &Path, dry_run: bool) -> io::Result<()> {
    let files = collect_rust_files(path)?;
    let mut modified = 0;
    let mut skipped = 0;

    for file_path in files {
        let content = fs::read_to_string(&file_path)?;

        if !has_license_header(&content) {
            println!("⏭️  Skipped (no header): {}", file_path.display());
            skipped += 1;
            continue;
        }

        let new_content = remove_license_header(&content);

        if dry_run {
            println!("🔍 Would remove header from: {}", file_path.display());
        } else {
            fs::write(&file_path, new_content)?;
            println!("✅ Removed header from: {}", file_path.display());
        }
        modified += 1;
    }

    println!("\n📊 Summary:");
    println!("   Modified: {}", modified);
    println!("   Skipped:  {}", skipped);
    if dry_run {
        println!("   (Dry run - no files were actually modified)");
    }

    Ok(())
}

fn collect_rust_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            if entry_path.is_file()
                && entry_path.extension().and_then(|s| s.to_str()) == Some("rs")
            {
                // Skip target directory
                if !entry_path
                    .components()
                    .any(|c| c.as_os_str() == "target")
                {
                    files.push(entry_path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

fn has_license_header(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with("// Copyright")
        && (trimmed.contains("Licensed under the Apache License")
            || trimmed.contains("SPDX-License-Identifier"))
}

fn remove_license_header(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut start_idx = 0;
    let mut in_header = false;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("// Copyright") {
            in_header = true;
        }

        if in_header {
            if trimmed.starts_with("//") || trimmed.is_empty() {
                start_idx = idx + 1;
            } else {
                break;
            }
        }
    }

    lines[start_idx..].join("\n").trim_start().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_license_header() {
        let with_header = r#"// Copyright 2025 Rigatoni Contributors
//
// Licensed under the Apache License, Version 2.0
// SPDX-License-Identifier: Apache-2.0

fn main() {}"#;

        let without_header = r#"fn main() {}"#;

        assert!(has_license_header(with_header));
        assert!(!has_license_header(without_header));
    }

    #[test]
    fn test_remove_license_header() {
        let with_header = r#"// Copyright 2025 Rigatoni Contributors
//
// Licensed under the Apache License, Version 2.0
// SPDX-License-Identifier: Apache-2.0

fn main() {
    println!("Hello");
}"#;

        let expected = r#"fn main() {
    println!("Hello");
}"#;

        assert_eq!(remove_license_header(with_header), expected);
    }
}
