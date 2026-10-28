//! Updating Mincord in place on Windows. Mincord asks GitHub for the latest
//! release, and when that is newer than this build it offers it. Taking the
//! offer downloads the same installer the releases page carries and runs it
//! with Inno Setup's silent switches, so the person sees Mincord's own
//! window rather than a setup wizard. The installer closes Mincord
//! (`CloseApplications=force`), replaces it and starts it again, which
//! `/restartapp=1` asks it to do; see packaging/windows/mincord.iss.
//!
//! Only Windows updates itself. A Linux release is an AppImage with a
//! `.zsync` file, which AppImageUpdate and Gear Lever follow, or an Arch
//! package pacman owns; a self-updater there would fight the thing that
//! installed it. See docs/packaging.md.

use std::path::{Path, PathBuf};

/// Whether this build can update itself.
pub const SUPPORTED: bool = cfg!(windows);

/// Where the releases are. The asset is the per-user installer
/// `scripts/package-windows.py` builds.
const LATEST: &str = "https://api.github.com/repos/mincord/mincord/releases/latest";
pub const RELEASES: &str = "https://github.com/mincord/mincord/releases";
/// The tail of the Windows installer's name, which tells it from the
/// portable ZIP and the Linux files on the same release.
const SETUP_SUFFIX: &str = "-windows-x64-setup.exe";

/// This build's version, which a release is newer than or it isn't offered.
pub fn running() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// A release newer than this build, as the offer names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Its version without the tag's `v`, such as `0.3.0`.
    pub version: String,
    /// Where its Windows installer is.
    pub setup: String,
    /// How large that installer is, for the progress the download shows;
    /// 0 when GitHub didn't say.
    pub size: u64,
    /// Its notes, as the release page shows them.
    pub notes: String,
}

/// The newest release, when it is newer than this build; `None` when this
/// build is the newest, or the release carries no Windows installer.
pub async fn latest() -> anyhow::Result<Option<Release>> {
    let feed: serde_json::Value = client()?
        .get(LATEST)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(offer(&feed, running()))
}

/// One request's client. It is made where it's used, so a run that never
/// updates never builds one, and GitHub turns away a request with no
/// user agent.
fn client() -> anyhow::Result<reqwest::Client> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    Ok(reqwest::Client::builder()
        .user_agent(concat!("Mincord/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()?)
}

/// The release `feed` describes, when it is newer than `running` and has an
/// installer to download. Apart from the request, so the choice is testable.
fn offer(feed: &serde_json::Value, running: &str) -> Option<Release> {
    if feed["draft"].as_bool() == Some(true) || feed["prerelease"].as_bool() == Some(true) {
        return None;
    }
    let version = feed["tag_name"].as_str()?.trim_start_matches('v');
    if !newer(version, running) {
        return None;
    }
    let asset = feed["assets"]
        .as_array()?
        .iter()
        .find(|asset| asset["name"].as_str().is_some_and(ends_with_setup))?;
    Some(Release {
        version: version.to_owned(),
        setup: asset["browser_download_url"].as_str()?.to_owned(),
        size: asset["size"].as_u64().unwrap_or_default(),
        notes: feed["body"].as_str().unwrap_or_default().trim().to_owned(),
    })
}

fn ends_with_setup(name: &str) -> bool {
    name.ends_with(SETUP_SUFFIX)
}

/// Whether `offered` is a later version than `running`. Versions are
/// `major.minor.patch` with an optional suffix, which a nightly carries
/// (`0.2.0nightly.20261020.5`); as SemVer orders a pre-release, a version
/// with a suffix comes before the same one without, and two suffixes
/// compare as text. An unreadable version is never newer.
fn newer(offered: &str, running: &str) -> bool {
    let Some(offered) = parts(offered) else {
        return false;
    };
    let Some(running) = parts(running) else {
        // Nothing to compare against: don't offer.
        return false;
    };
    offered > running
}

/// `version` as what orders it: its three numbers, whether it has no
/// suffix, and the suffix. The middle field is what puts `0.2.0` after
/// `0.2.0nightly.20261020.5`, since `true` orders after `false`.
fn parts(version: &str) -> Option<([u64; 3], bool, &str)> {
    let end = version
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(version.len());
    let (numbers, suffix) = version.split_at(end);
    let mut found = [0; 3];
    let mut fields = numbers.split('.');
    for field in &mut found {
        *field = fields.next()?.parse().ok()?;
    }
    if fields.next().is_some() {
        return None;
    }
    Some((found, suffix.is_empty(), suffix))
}

/// Downloads `release`'s installer into `folder`, calling `progress` with
/// the bytes so far and the bytes expected as it goes. The file is written
/// as it arrives rather than held in memory: a Mincord installer is around
/// 57 MiB.
pub async fn download(
    release: &Release,
    folder: &Path,
    mut progress: impl FnMut(u64, u64),
) -> anyhow::Result<PathBuf> {
    use futures_util::StreamExt;
    use std::io::Write;

    std::fs::create_dir_all(folder)?;
    let setup = folder.join(format!("Mincord-{}-setup.exe", release.version));
    // A part file, moved into place once it is whole, so an interrupted
    // download is never run.
    let part = setup.with_extension("part");
    let response = client()?
        .get(&release.setup)
        .send()
        .await?
        .error_for_status()?;
    let expected = response.content_length().unwrap_or(release.size);
    let mut file = std::fs::File::create(&part)?;
    let mut so_far = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        so_far += chunk.len() as u64;
        progress(so_far, expected);
    }
    file.sync_all()?;
    drop(file);
    std::fs::rename(&part, &setup)?;
    Ok(setup)
}

/// Runs the installer `setup` silently and leaves it running: it replaces
/// this Mincord and starts it again. The caller quits once this returns,
/// since Windows won't replace a running executable, and Mincord goes on
/// running in the tray with its window shut.
#[cfg(windows)]
pub fn install(setup: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;

    // DETACHED_PROCESS: the installer outlives the Mincord that started it.
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    std::process::Command::new(setup)
        .args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/NOCANCEL",
            // mincord.iss starts Mincord again on this.
            "/restartapp=1",
        ])
        .creation_flags(DETACHED_PROCESS)
        .spawn()?;
    Ok(())
}

#[cfg(not(windows))]
pub fn install(_setup: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "only the Windows build installs its own update",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_versions_are_newer_and_a_nightly_comes_before_its_release() {
        assert!(newer("0.3.0", "0.2.0"));
        assert!(newer("0.2.1", "0.2.0"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(!newer("0.2.0", "0.2.0"), "the same version");
        assert!(!newer("0.1.9", "0.2.0"), "an older one");
        // A nightly sorts after the release it builds on and before the
        // one it is named for, as pacman and SemVer sort it.
        assert!(newer("0.2.0nightly.20261020.5", "0.1.0"));
        assert!(!newer("0.2.0nightly.20261020.5", "0.2.0"));
        assert!(newer("0.2.0", "0.2.0nightly.20261020.5"));
        assert!(newer("0.2.0nightly.20261021.1", "0.2.0nightly.20261020.5"));
        // Nothing readable is never newer: no offer beats a wrong one.
        assert!(!newer("latest", "0.2.0"));
        assert!(!newer("0.3", "0.2.0"));
        assert!(!newer("0.3.0.1", "0.2.0"));
    }

    #[test]
    fn a_release_is_offered_only_with_a_windows_installer_that_is_newer() {
        let feed = |tag: &str, assets: serde_json::Value| {
            serde_json::json!({
                "tag_name": tag,
                "body": "  Fixed the thing.  ",
                "draft": false,
                "prerelease": false,
                "assets": assets,
            })
        };
        let setup = serde_json::json!([
            {
                "name": "Mincord-0.3.0-x86_64.AppImage",
                "browser_download_url": "https://example.invalid/appimage",
                "size": 1,
            },
            {
                "name": "Mincord-0.3.0-windows-x64-setup.exe",
                "browser_download_url": "https://example.invalid/setup.exe",
                "size": 59_000_000,
            },
        ]);

        let offered = offer(&feed("v0.3.0", setup.clone()), "0.2.0").expect("newer");
        assert_eq!(offered.version, "0.3.0");
        assert_eq!(offered.setup, "https://example.invalid/setup.exe");
        assert_eq!(offered.size, 59_000_000);
        assert_eq!(offered.notes, "Fixed the thing.", "trimmed");

        assert_eq!(
            offer(&feed("v0.2.0", setup.clone()), "0.2.0"),
            None,
            "this build is the newest"
        );
        assert_eq!(
            offer(
                &feed(
                    "v0.3.0",
                    serde_json::json!([{
                        "name": "Mincord-0.3.0-x86_64.AppImage",
                        "browser_download_url": "https://example.invalid/appimage",
                        "size": 1,
                    }])
                ),
                "0.2.0",
            ),
            None,
            "no Windows installer to run"
        );
        let mut draft = feed("v0.4.0", setup);
        draft["prerelease"] = serde_json::Value::Bool(true);
        assert_eq!(offer(&draft, "0.2.0"), None, "not released yet");
    }
}
