use super::{DynError, Result, resolve_cli_path};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_SOURCE: &str = "https://melpa.org/packages";
const DEFAULT_FIXTURE_DIR: &str = "neomacs-melpa-tests/fixtures/frozen-melpa";

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum FixtureSource {
    Url(String),
    Directory(PathBuf),
}

#[derive(Debug, Clone)]
pub(super) struct MelpaFixtureOptions {
    pub(super) repo_root: PathBuf,
    pub(super) source: FixtureSource,
    pub(super) fixture_dir: PathBuf,
    pub(super) packages: Vec<String>,
    pub(super) snapshot_date: Option<String>,
}

#[derive(Debug)]
struct ArchivePackage {
    name: String,
    version: String,
    entry: String,
}

#[derive(Debug)]
struct PackageMetadata {
    name: String,
    version: String,
    commit: String,
    license: &'static str,
    filename: String,
    sha256: String,
}

impl MelpaFixtureOptions {
    pub(super) fn parse(
        repo_root: PathBuf,
        args: impl IntoIterator<Item = OsString>,
    ) -> Result<Self> {
        let mut source = FixtureSource::Url(DEFAULT_SOURCE.to_string());
        let mut fixture_dir = repo_root.join(DEFAULT_FIXTURE_DIR);
        let mut packages = Vec::new();
        let mut snapshot_date = None;
        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.to_string_lossy().as_ref() {
                "--source" => {
                    let value = required_value(&mut args, "--source")?;
                    let text = value.to_string_lossy();
                    source = if text.starts_with("https://") || text.starts_with("http://") {
                        FixtureSource::Url(text.into_owned())
                    } else {
                        FixtureSource::Directory(resolve_cli_path(&repo_root, value))
                    };
                }
                "--fixture-dir" => {
                    fixture_dir =
                        resolve_cli_path(&repo_root, required_value(&mut args, "--fixture-dir")?);
                }
                "--package" => {
                    packages.push(
                        required_value(&mut args, "--package")?
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                "--snapshot-date" => {
                    snapshot_date = Some(
                        required_value(&mut args, "--snapshot-date")?
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
                "--help" | "-h" => {
                    return Err(refresh_usage().into());
                }
                other => {
                    return Err(format!(
                        "unknown refresh-melpa-fixtures option: {other}\n\n{}",
                        refresh_usage()
                    )
                    .into());
                }
            }
        }

        Ok(Self {
            repo_root,
            source,
            fixture_dir,
            packages,
            snapshot_date,
        })
    }
}

fn required_value(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<OsString> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value").into())
}

pub(super) fn refresh_melpa_fixtures(options: &MelpaFixtureOptions) -> Result<()> {
    let packages = requested_packages(options)?;
    validate_package_names(&packages)?;
    let snapshot_date = options.snapshot_date.clone().unwrap_or_else(utc_date);
    let scratch_root = options.repo_root.join("tmp");
    fs::create_dir_all(&scratch_root)?;
    let scratch = tempfile::Builder::new()
        .prefix("melpa-fixtures-")
        .tempdir_in(&scratch_root)?;
    let stage = scratch.path().join("frozen-melpa");
    fs::create_dir_all(&stage)?;

    let archive_path = scratch.path().join("archive-contents.upstream");
    fetch(options, "archive-contents", &archive_path)?;
    let archive_contents = fs::read_to_string(&archive_path)?;
    let available = parse_archive_packages(&archive_contents)?;
    let mut selected = Vec::with_capacity(packages.len());
    let mut metadata = Vec::with_capacity(packages.len());

    for package in &packages {
        if let Some(path) = find_builtin_package(&options.repo_root.join("lisp"), package)? {
            let relative = path.strip_prefix(&options.repo_root).unwrap_or(&path);
            return Err(format!(
                "refusing to freeze `{package}` because it is built into Neomacs at {}",
                relative.display()
            )
            .into());
        }
        let entry = available.get(package).ok_or_else(|| {
            format!("MELPA archive does not contain requested package `{package}`")
        })?;
        let filename = format!("{}-{}.tar", entry.name, entry.version);
        let destination = stage.join(&filename);
        fetch(options, &filename, &destination)?;
        metadata.push(validate_tarball(entry, &destination)?);
        selected.push(entry);
    }

    fs::write(stage.join("archive-contents"), render_archive(&selected))?;
    fs::write(stage.join("packages.txt"), render_package_list(&packages))?;
    fs::write(stage.join("SHA256SUMS"), render_checksums(&metadata))?;
    fs::write(
        stage.join("README.md"),
        render_readme(&snapshot_date, &metadata),
    )?;
    publish_snapshot(&stage, &options.fixture_dir)?;

    println!(
        "Refreshed {} frozen MELPA package{} in {}",
        metadata.len(),
        if metadata.len() == 1 { "" } else { "s" },
        options.fixture_dir.display()
    );
    Ok(())
}

fn requested_packages(options: &MelpaFixtureOptions) -> Result<Vec<String>> {
    if !options.packages.is_empty() {
        return Ok(options.packages.clone());
    }
    let package_file = options.fixture_dir.join("packages.txt");
    let contents = fs::read_to_string(&package_file).map_err(|error| {
        format!(
            "read package selection {}: {error}; pass --package NAME or create packages.txt",
            package_file.display()
        )
    })?;
    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

fn validate_package_names(packages: &[String]) -> Result<()> {
    if packages.is_empty() {
        return Err("at least one MELPA package must be selected".into());
    }
    let mut seen = BTreeSet::new();
    for package in packages {
        if package.is_empty()
            || !package
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(format!("invalid MELPA package name `{package}`").into());
        }
        if !seen.insert(package) {
            return Err(format!("duplicate MELPA package `{package}`").into());
        }
    }
    Ok(())
}

fn fetch(options: &MelpaFixtureOptions, filename: &str, destination: &Path) -> Result<()> {
    match &options.source {
        FixtureSource::Url(source) => {
            let url = format!("{}/{}", source.trim_end_matches('/'), filename);
            let output = Command::new("curl")
                .args([
                    "--fail",
                    "--location",
                    "--silent",
                    "--show-error",
                    "--output",
                ])
                .arg(destination)
                .arg(&url)
                .output()
                .map_err(|error| format!("launch curl for {url}: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "download {url} failed with {}\n{}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }
        FixtureSource::Directory(source_root) => {
            let source = source_root.join(filename);
            fs::copy(&source, destination).map_err(|error| -> DynError {
                format!(
                    "copy fixture source {} to {}: {error}",
                    source.display(),
                    destination.display()
                )
                .into()
            })?;
        }
    }
    Ok(())
}

fn parse_archive_packages(contents: &str) -> Result<BTreeMap<String, ArchivePackage>> {
    let mut packages = BTreeMap::new();
    for entry in top_level_entries(contents)? {
        let name = entry
            .trim_start_matches('(')
            .split_whitespace()
            .next()
            .ok_or("archive entry is missing a package name")?
            .to_string();
        if !entry.contains(" tar]") {
            continue;
        }
        let vector = entry
            .find('[')
            .ok_or_else(|| format!("archive entry `{name}` has no descriptor vector"))?;
        let version_start = entry[vector..]
            .find('(')
            .map(|index| vector + index + 1)
            .ok_or_else(|| format!("archive entry `{name}` has no version"))?;
        let version_end = entry[version_start..]
            .find(')')
            .map(|index| version_start + index)
            .ok_or_else(|| format!("archive entry `{name}` has an unterminated version"))?;
        let version = entry[version_start..version_end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(".");
        packages.insert(
            name.clone(),
            ArchivePackage {
                name,
                version,
                entry,
            },
        );
    }
    Ok(packages)
}

fn top_level_entries(contents: &str) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in contents.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if character == '"' {
            in_string = true;
            continue;
        }
        match character {
            '(' => {
                if depth == 1 {
                    start = Some(index);
                }
                depth += 1;
            }
            ')' => {
                if depth == 0 {
                    return Err("archive-contents has an unmatched closing parenthesis".into());
                }
                depth -= 1;
                if depth == 1
                    && let Some(entry_start) = start.take()
                {
                    entries.push(contents[entry_start..=index].to_string());
                }
            }
            _ => {}
        }
    }
    if in_string || depth != 0 {
        return Err("archive-contents is not a balanced Lisp form".into());
    }
    Ok(entries)
}

fn validate_tarball(entry: &ArchivePackage, path: &Path) -> Result<PackageMetadata> {
    let file = fs::File::open(path)?;
    let mut archive = tar::Archive::new(file);
    let package_metadata_name = format!("/{}-pkg.el", entry.name);
    let mut package_metadata = None;
    let mut has_gpl_v3_or_later = false;

    for archive_entry in archive.entries()? {
        let mut archive_entry = archive_entry?;
        if !archive_entry.header().entry_type().is_file() {
            continue;
        }
        let entry_path = archive_entry.path()?.into_owned();
        let mut bytes = Vec::new();
        archive_entry.read_to_end(&mut bytes)?;
        let contents = String::from_utf8_lossy(&bytes);
        let normalized = format!("/{}", entry_path.to_string_lossy());
        has_gpl_v3_or_later |= contents.contains("GNU General Public License")
            && contents.contains("version 3 of the License")
            && contents.contains("(at your option) any later version");
        if normalized.ends_with(&package_metadata_name) {
            package_metadata = Some(contents.into_owned());
        }
    }

    let package_metadata = package_metadata
        .ok_or_else(|| format!("{} does not contain {}-pkg.el", path.display(), entry.name))?;
    let declaration = format!("(define-package \"{}\" \"{}\"", entry.name, entry.version);
    if !package_metadata.contains(&declaration) {
        return Err(format!(
            "{} package metadata does not declare {} {}",
            path.display(),
            entry.name,
            entry.version
        )
        .into());
    }
    let commit = quoted_keyword_value(&package_metadata, ":commit")
        .ok_or_else(|| format!("{} package metadata has no :commit pin", path.display()))?;
    if !has_gpl_v3_or_later {
        return Err(format!(
            "{} does not declare GPL-3.0-or-later licensing",
            path.display()
        )
        .into());
    }
    let bytes = fs::read(path)?;
    let sha256 = lowercase_hex(&Sha256::digest(bytes));

    Ok(PackageMetadata {
        name: entry.name.clone(),
        version: entry.version.clone(),
        commit,
        license: "GPL-3.0-or-later",
        filename: path
            .file_name()
            .expect("staged tarball has a filename")
            .to_string_lossy()
            .into_owned(),
        sha256,
    })
}

fn lowercase_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}

fn quoted_keyword_value(contents: &str, keyword: &str) -> Option<String> {
    let remainder = contents.split_once(keyword)?.1.trim_start();
    let quoted = remainder.strip_prefix('"')?;
    Some(quoted.split_once('"')?.0.to_string())
}

fn find_builtin_package(lisp_root: &Path, package: &str) -> Result<Option<PathBuf>> {
    if !lisp_root.is_dir() {
        return Ok(None);
    }
    let target = format!("{package}.el");
    let mut directories = vec![lisp_root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if file_type.is_file() && entry.file_name() == target.as_str() {
                return Ok(Some(entry.path()));
            }
        }
    }
    Ok(None)
}

fn render_archive(packages: &[&ArchivePackage]) -> String {
    let mut output = String::from("(1\n");
    for package in packages {
        output.push(' ');
        output.push_str(&package.entry);
        output.push('\n');
    }
    output.push_str(")\n");
    output
}

fn render_package_list(packages: &[String]) -> String {
    let mut output = packages.join("\n");
    output.push('\n');
    output
}

fn render_checksums(packages: &[PackageMetadata]) -> String {
    let mut output = String::new();
    for package in packages {
        writeln!(output, "{}  {}", package.sha256, package.filename).unwrap();
    }
    output
}

fn render_readme(snapshot_date: &str, packages: &[PackageMetadata]) -> String {
    let mut output = format!(
        "# Frozen MELPA compatibility archive\n\n\
         This directory is an immutable, offline package source for compatibility\n\
         tests. The tarballs were downloaded unmodified from `{DEFAULT_SOURCE}/`\n\
         on {snapshot_date} and are pinned by `SHA256SUMS`.\n\n\
         Refresh this snapshot with `cargo xtask refresh-melpa-fixtures`.\n\
         The command rejects packages already built into Neomacs and validates\n\
         package name, version, commit, license, and checksum metadata.\n\n\
         | Package | MELPA version | Upstream commit | License |\n\
         |---|---:|---|---|\n"
    );
    for package in packages {
        writeln!(
            output,
            "| {} | {} | `{}` | {} |",
            package.name, package.version, package.commit, package.license
        )
        .unwrap();
    }
    output.push_str(
        "\nThe packages retain their upstream copyright and licensing headers. They\n\
         are test fixtures, not runtime dependencies.\n",
    );
    output
}

fn publish_snapshot(stage: &Path, fixture_dir: &Path) -> Result<()> {
    let fixture_parent = fixture_dir
        .parent()
        .ok_or_else(|| format!("fixture directory has no parent: {}", fixture_dir.display()))?;
    fs::create_dir_all(fixture_parent)?;
    if fixture_dir.exists() && !fixture_dir.is_dir() {
        return Err(format!(
            "fixture destination is not a directory: {}",
            fixture_dir.display()
        )
        .into());
    }
    let backup = stage
        .parent()
        .expect("staged snapshot has a scratch parent")
        .join("previous-frozen-melpa");

    if fixture_dir.exists() {
        fs::rename(fixture_dir, &backup).map_err(|error| {
            format!(
                "move current fixture snapshot {} into workspace scratch {}: {error}",
                fixture_dir.display(),
                backup.display()
            )
        })?;
    }
    if let Err(publish_error) = fs::rename(stage, fixture_dir) {
        if backup.exists()
            && let Err(restore_error) = fs::rename(&backup, fixture_dir)
        {
            return Err(format!(
                "publish fixture snapshot failed: {publish_error}; restoring {} also failed: {restore_error}",
                fixture_dir.display()
            )
            .into());
        }
        return Err(format!(
            "publish fixture snapshot {} failed; previous snapshot restored: {publish_error}",
            fixture_dir.display()
        )
        .into());
    }
    if backup.exists() {
        fs::remove_dir_all(backup)?;
    }
    Ok(())
}

fn utc_date() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    civil_date_from_unix_days((seconds / 86_400) as i64)
}

fn civil_date_from_unix_days(days: i64) -> String {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn refresh_usage() -> &'static str {
    "Usage: cargo xtask refresh-melpa-fixtures \
     [--source URL_OR_DIR] [--fixture-dir DIR] [--package NAME]...\n\
     \n\
     Refresh the pinned offline MELPA archive. Without --package, package names\n\
     are read from the fixture directory's packages.txt. All scratch files are\n\
     created below the repository's ./tmp directory."
}

#[cfg(test)]
mod tests {
    use super::civil_date_from_unix_days;

    #[test]
    fn civil_date_conversion_covers_epoch_and_snapshot_date() {
        assert_eq!(civil_date_from_unix_days(0), "1970-01-01");
        assert_eq!(civil_date_from_unix_days(20_659), "2026-07-25");
    }
}
