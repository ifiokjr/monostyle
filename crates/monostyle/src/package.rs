//! Workspace and package detection.
//!
//! A repository is rarely one unit of code, and a score for the whole thing is not what a reader
//! needs when they are told to fix something. This module finds the packages a repository declares
//! and attributes each analyzed file to the package that owns it, so the report can say "this
//! package is the problem" rather than "this repository scored 72".
//!
//! # Detection strategy
//!
//! Manifests are read directly rather than by shelling out to each ecosystem's tooling, because
//! the goal is attribution rather than dependency resolution. Four workspace formats cover the
//! repositories this tool is used on:
//!
//! | Ecosystem | Manifest | Declares members in |
//! | --- | --- | --- |
//! | Cargo | `Cargo.toml` | `[workspace] members` |
//! | npm | `package.json` | `workspaces` |
//! | pnpm | `pnpm-workspace.yaml` | `packages` |
//! | Dart | `pubspec.yaml` | `workspace` |
//!
//! A directory containing a manifest but no member list is still a package — that is the common
//! case for a single-package repository — so detection degrades to "one package at the root"
//! rather than reporting nothing.

use std::path::Path;
use std::path::PathBuf;

use monostyle_core::Category;
use monostyle_core::Score;
use monostyle_core::ScoringConfig;
use serde::Serialize;

use crate::aggregate::FileScore;

/// A detected package within a repository.
#[derive(Debug, Clone, Serialize)]
pub struct Package {
	/// The package's name, as declared in its manifest.
	pub name: String,
	/// The directory containing the manifest, relative to the repository root.
	pub directory: PathBuf,
	/// Which ecosystem declared it.
	pub ecosystem: Ecosystem,
}

/// The ecosystem a package belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
	/// A Cargo crate.
	Cargo,
	/// An npm package.
	Npm,
	/// A Dart or Flutter package.
	Dart,
}

impl Ecosystem {
	/// A human-readable label.
	#[must_use]
	pub const fn label(self) -> &'static str {
		match self {
			Self::Cargo => "cargo",
			Self::Npm => "npm",
			Self::Dart => "dart",
		}
	}
}

/// A package with its analyzed files and scores attached.
#[derive(Debug, Clone, Serialize)]
pub struct PackageReport {
	/// The detected package.
	pub package: Package,
	/// Lines of code in the package.
	pub code_lines: usize,
	/// How many files were analyzed.
	pub files: usize,
	/// Readability score, weighted by lines within the package.
	pub readability: Score,
	/// Complexity score, weighted by lines within the package.
	pub complexity: Score,
	/// Total penalty the package contributes.
	pub total_penalty: f64,
}

impl PackageReport {
	/// The combined score, out of 100.
	#[must_use]
	pub fn overall(&self) -> f64 {
		f64::midpoint(self.readability.value, self.complexity.value)
	}
}

/// Detects the packages declared under `root`.
///
/// Returns an empty vector when nothing resembles a package, which the caller reports as a
/// repository without declared packages rather than as an error.
#[must_use]
pub fn detect_packages(root: &Path) -> Vec<Package> {
	let mut packages = members_of_every_ecosystem(root);

	// A repository with a single manifest and no member list is one package at its root.
	if packages.is_empty() {
		packages.extend(root_package(root));
	}

	packages.sort_by(|left, right| left.directory.cmp(&right.directory));
	packages.dedup_by(|left, right| left.directory == right.directory);
	packages
}

/// Finds the packages declared as workspace members across every ecosystem.
///
/// Each ecosystem is a table entry rather than a repeated loop, so a fix to the scanning reaches all of
/// them and adding an ecosystem is one line.
fn members_of_every_ecosystem(root: &Path) -> Vec<Package> {
	let mut packages = Vec::new();

	for detector in DETECTORS {
		for member in (detector.members)(root) {
			for directory in expand_member(root, &member) {
				let Some(name) = (detector.name_at)(&directory) else {
					continue;
				};

				packages.push(Package {
					name,
					directory,
					ecosystem: detector.ecosystem,
				});
			}
		}
	}

	packages
}

/// Finds the single package at the repository root, if one is declared.
fn root_package(root: &Path) -> Option<Package> {
	DETECTORS.iter().find_map(|detector| {
		let name = (detector.name_at)(root)?;

		Some(Package {
			name,
			directory: root.to_path_buf(),
			ecosystem: detector.ecosystem,
		})
	})
}

/// How to find the packages one ecosystem declares.
struct Detector {
	/// The ecosystem this describes.
	ecosystem: Ecosystem,
	/// Reads the workspace member list, which may be glob patterns.
	members: fn(&Path) -> Vec<String>,
	/// Reads a package's name from its manifest in `directory`, if there is one.
	name_at: fn(&Path) -> Option<String>,
}

/// Every ecosystem monostyle detects packages in.
///
/// A table rather than a chain of loops so adding an ecosystem is one entry, and so the same code handles
/// workspace members and the single-manifest fallback.
const DETECTORS: &[Detector] = &[
	Detector {
		ecosystem: Ecosystem::Cargo,
		members: |root| read_workspace_manifest(root.join("Cargo.toml")).unwrap_or_default(),
		name_at: |directory| cargo_package_name(directory),
	},
	Detector {
		ecosystem: Ecosystem::Npm,
		members: npm_workspace_members,
		name_at: |directory| npm_package_name(directory),
	},
	Detector {
		ecosystem: Ecosystem::Dart,
		members: dart_workspace_members,
		name_at: |directory| dart_package_name(directory),
	},
];

/// Attributes each file to the package whose directory contains it.
///
/// The longest matching directory wins, so a nested package claims its files rather than the
/// repository root claiming them.
#[must_use]
pub fn attribute_files(packages: &[Package], files: &[PathBuf]) -> Vec<(Package, Vec<PathBuf>)> {
	let mut attributed: Vec<(Package, Vec<PathBuf>)> = packages
		.iter()
		.map(|package| (package.clone(), Vec::new()))
		.collect();

	for file in files {
		let owner = packages
			.iter()
			.enumerate()
			.filter(|(_index, package)| file.starts_with(&package.directory))
			.max_by_key(|(_index, package)| package.directory.components().count());

		if let Some((index, _package)) = owner
			&& let Some((_package, owned)) = attributed.get_mut(index)
		{
			owned.push(file.clone());
		}
	}

	attributed.retain(|(_package, owned)| !owned.is_empty());
	attributed
}

/// Builds a package report from the files attributed to it.
#[must_use]
pub fn score_package(
	package: &Package,
	scores: &[FileScore],
	config: ScoringConfig,
) -> PackageReport {
	let code_lines: usize = scores.iter().map(|file| file.code_lines).sum();
	let readability = crate::aggregate::project_score(scores, Category::Readability, config);
	let complexity = crate::aggregate::project_score(scores, Category::Complexity, config);

	PackageReport {
		package: package.clone(),
		code_lines,
		files: scores.len(),
		readability,
		complexity,
		total_penalty: scores.iter().map(|file| file.total_penalty).sum(),
	}
}

/// Expands a workspace member pattern into the directories it names.
///
/// A member is either a literal directory or a glob over one level, which is how nearly every
/// JavaScript and Dart monorepo declares its members: `packages/*`, `crates/*`, `apps/web/*`. The
/// expansion covers a trailing `*` segment and a `**` depth, which is as much of glob syntax as this
/// needs — the goal is attribution, not a general file matcher.
fn expand_member(root: &Path, member: &str) -> Vec<PathBuf> {
	let member = member.trim_end_matches('/');

	// A literal member names one directory.
	if !member.contains('*') {
		return vec![root.join(member)];
	}

	// Split at the first segment containing a wildcard; everything before it is a literal prefix.
	let segments: Vec<&str> = member.split('/').collect();
	let wildcard = segments.iter().position(|segment| segment.contains('*'));

	let Some(index) = wildcard else {
		return vec![root.join(member)];
	};

	// Everything after the wildcard must be a literal suffix, which this expansion does not support. A
	// member like `crates/*/src` would otherwise be silently joined into a nonsense path.
	if segments
		.get(index + 1..)
		.is_some_and(|rest| !rest.is_empty())
	{
		return Vec::new();
	}

	let prefix = root.join(segments.get(..index).unwrap_or_default().join("/"));
	let depth_is_recursive = segments.get(index) == Some(&"**");

	let Ok(entries) = std::fs::read_dir(&prefix) else {
		return Vec::new();
	};

	let mut directories: Vec<PathBuf> = entries
		.filter_map(Result::ok)
		.filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
		.map(|entry| entry.path())
		.collect();

	// `**` also matches nested directories, so the walk descends; a single `*` does not.
	if depth_is_recursive {
		let mut nested = Vec::new();

		for directory in &directories {
			if let Ok(entries) = std::fs::read_dir(directory) {
				nested.extend(
					entries
						.filter_map(Result::ok)
						.filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
						.map(|entry| entry.path()),
				);
			}
		}

		directories.extend(nested);
	}

	directories.sort();
	directories
}

/// Reads a Cargo workspace's member list.
fn read_workspace_manifest(path: PathBuf) -> Option<Vec<String>> {
	let contents = std::fs::read_to_string(path).ok()?;
	let parsed: toml::Value = toml::from_str(&contents).ok()?;

	parsed
		.get("workspace")?
		.get("members")?
		.as_array()
		.map(|members| {
			members
				.iter()
				.filter_map(|member| member.as_str().map(str::to_string))
				.collect()
		})
}

/// Reads a package name from a Cargo manifest.
fn cargo_package_name(directory: &Path) -> Option<String> {
	let contents = std::fs::read_to_string(directory.join("Cargo.toml")).ok()?;
	let parsed: toml::Value = toml::from_str(&contents).ok()?;

	parsed
		.get("package")?
		.get("name")?
		.as_str()
		.map(str::to_string)
}

/// Reads npm workspace members from `package.json` or `pnpm-workspace.yaml`.
fn npm_workspace_members(root: &Path) -> Vec<String> {
	// pnpm declares workspaces in YAML, which is the form most JavaScript monorepos use.
	if let Ok(contents) = std::fs::read_to_string(root.join("pnpm-workspace.yaml")) {
		return parse_workspace_yaml(&contents);
	}

	if let Ok(contents) = std::fs::read_to_string(root.join("package.json"))
		&& let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&contents)
	{
		return match parsed.get("workspaces") {
			Some(serde_json::Value::Array(members)) => {
				members
					.iter()
					.filter_map(|member| member.as_str().map(str::to_string))
					.collect()
			}
			Some(serde_json::Value::Object(object)) => {
				object
					.get("packages")
					.and_then(|packages| packages.as_array())
					.map(|members| {
						members
							.iter()
							.filter_map(|member| member.as_str().map(str::to_string))
							.collect()
					})
					.unwrap_or_default()
			}
			_ => Vec::new(),
		};
	}

	Vec::new()
}

/// Extracts package globs from a `pnpm-workspace.yaml` without a YAML dependency.
///
/// The file's relevant shape is a single `packages:` list of quoted globs, so a full parser would
/// be a large dependency for one field. Returning nothing on an unrecognized shape is the safe
/// failure: the repository simply reports no packages rather than the wrong ones.
fn parse_workspace_yaml(contents: &str) -> Vec<String> {
	glob_list_under(contents, "packages:")
}

/// Reads the list of quoted values under `key` in a small YAML document.
///
/// Used for both `pnpm-workspace.yaml`'s `packages:` and `pubspec.yaml`'s `workspace:`. They differ only by
/// key name, so one scanner serves both: the shape is a single key followed by a dash-prefixed list, and a
/// full YAML parser would be a large dependency for one field.
///
/// A new top-level key ends the list, which is what keeps a sibling field from being read as a member.
fn glob_list_under(contents: &str, key: &str) -> Vec<String> {
	let mut members = Vec::new();
	let mut in_list = false;

	for line in contents.lines() {
		let trimmed = line.trim();

		if trimmed.starts_with('#') {
			continue;
		}

		if trimmed.starts_with(key) {
			in_list = true;
			continue;
		}

		if !in_list || trimmed.is_empty() {
			continue;
		}

		// A line that is not indented starts a new top-level key, which ends the list.
		let indented = line.starts_with(' ') || line.starts_with('\t');

		if !indented {
			break;
		}

		let Some(entry) = trimmed.strip_prefix("- ") else {
			continue;
		};

		let value = entry.trim().trim_matches(['"', '\'']);

		if !value.is_empty() {
			members.push(value.to_string());
		}
	}

	members
}

/// Reads a package name from an npm manifest.
fn npm_package_name(directory: &Path) -> Option<String> {
	let contents = std::fs::read_to_string(directory.join("package.json")).ok()?;
	let parsed: serde_json::Value = serde_json::from_str(&contents).ok()?;

	parsed.get("name")?.as_str().map(str::to_string)
}

/// Reads Dart workspace members from `pubspec.yaml`.
fn dart_workspace_members(root: &Path) -> Vec<String> {
	let Ok(contents) = std::fs::read_to_string(root.join("pubspec.yaml")) else {
		return Vec::new();
	};

	glob_list_under(&contents, "workspace:")
}

/// Reads a package name from a Dart manifest.
fn dart_package_name(directory: &Path) -> Option<String> {
	let contents = std::fs::read_to_string(directory.join("pubspec.yaml")).ok()?;

	for line in contents.lines() {
		let trimmed = line.trim();

		if let Some(name) = trimmed.strip_prefix("name:") {
			return Some(name.trim().trim_matches(['"', '\'']).to_string());
		}
	}

	None
}
