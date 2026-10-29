# Packaging and releases

How the Linux and Windows releases are made and published. Building and
checking Mincord for development is in [building](building.md).

## Linux AppImage

`scripts/build-appimage` makes the Linux release in `target/dist/linux`:
`Mincord-VERSION-x86_64.AppImage`, its `.zsync` file, the source snapshot and
`SHA256SUMS.txt`. All four go on the release page. AppImageUpdate, and AppImage
managers such as Gear Lever, find a newer release through the `.zsync` file,
named by the update information in the runtime:
`gh-releases-zsync|mincord|mincord|latest|Mincord-*-x86_64.AppImage.zsync`.
The first build compiles every dependency in the Nix sandbox; packing takes
about 10 seconds after that.

Nix builds the binary, `nix/package.nix`'s Discord-only release build, and
`nix/appimage.nix` puts it in an AppDir with the desktop file, the icons and
the AppStream metainfo, `packaging/linux/io.github.mincord.Mincord.metainfo.xml`.
It sets the binary's interpreter to `/lib64/ld-linux-x86-64.so.2` and removes
Nix's RUNPATH, so the binary runs on the host's loader, glibc and libstdc++, as
one built on a distribution would. That's what lets the host's GPU driver,
Mesa or NVIDIA's, load into it: a driver needs the glibc it was built for, and
NVIDIA's must match the kernel module, so neither can come in the AppImage.
Nothing else is bundled either. Besides the C and C++ runtimes, the binary
links only GLib, PulseAudio's client, fontconfig and FreeType, which every
desktop has, and it loads EGL, Vulkan, Wayland, X11 and PipeWire at runtime.

The derivation fails if the binary links anything else, or asks for a glibc
newer than 2.39 or a libstdc++ newer than GCC 14's. That covers Ubuntu 24.04,
Fedora 40, Debian 13 and anything newer. nixpkgs' glibc is newer than that, so
a nixpkgs update can raise what the binary asks for. Raise the floors in
`nix/appimage.nix` only knowing which distributions that drops. Older ones,
such as Ubuntu 22.04, would need a build against their own glibc.

`scripts/package-linux.py`, which runs in the development shell, with
squashfs-tools and zsync added from the shell's nixpkgs, adds the
licenses and notices, `THIRD-PARTY.json`, `SOURCE.txt` and `BUILD.json` to
`usr/share/doc/mincord`, as the Windows package has them, through the helpers
in `scripts/release_package.py` that both packaging scripts use. Skia's
prebuilt Linux archive carries only Skia's own license, so the notices of its
third-party code come from the pinned Skia source,
`nix build path:./nix#skia-source`. Then it packs the AppDir with mksquashfs
(zstd level 19, 1 MiB blocks, fixed owners and times, so the same AppDir packs
to the same file) behind AppImage's official type-2 runtime, pinned by hash.
The runtime is static: it mounts the image with FUSE 3's `fusermount3` and
needs no libfuse, and `--appimage-extract-and-run` runs it without FUSE at all.
The flake sees only the files Git tracks, so add a new file under `nix/` or
`packaging/` before building.

Screen sharing hasn't been tried from the AppImage.

## Arch Linux packages

`packaging/arch/PKGBUILD` builds the Discord-only release from source with
Arch's toolchain, as `nix/package.nix` builds it: `cd packaging/arch && makepkg
-si`. It clones the tag of its `pkgver`, which `scripts/set-version` sets to a
stable version, so run `scripts/set-version 0.2.0` before committing and
tagging `v0.2.0`. The tag's commit holds the PKGBUILD, which can't hold its own
checksum, so that source has none; the libwebrtc and Skia archives keep theirs.
Arch's `llvm21` is a later 21.x than Nix's, which is why
`nix/prepare-skia-native.py` takes any LLVM 21. Nothing builds this PKGBUILD
automatically: build it in a clean `archlinux:base-devel` container before a
release, about 6 minutes from cold.

The releases carry a package of their own binary instead. `scripts/package-arch`,
run after `scripts/build-appimage`, packs the AppImage's binary, desktop file,
icon, metainfo and notices into `mincord-VERSION-1-x86_64.pkg.tar.zst` beside
it, through `packaging/arch/release.PKGBUILD`, and adds it to
`SHA256SUMS.txt`. It's the AppImage's binary, which runs on the host's loader
and libraries, so the two packages can't differ. The script runs makepkg in an
`archlinux:base-devel` container with Docker, then installs the package there
and runs `mincord --smoke-test`, in about 20 seconds. A nightly's version
becomes `0.2.0nightly.20261020.5`, which pacman sorts after 0.1.0 and before
0.2.0, as SemVer sorts the nightly; with a dot before `nightly`, pacman would
sort it after 0.2.0. `sudo pacman -U` installs the file or its link; there's no
repository, so `pacman -Syu` doesn't update it.

## Stable releases

A release builds from its tag on GitHub, `.github/workflows/release.yml`.
Prepare it in the repository first: `scripts/set-version 0.1.0`, which writes
the version into `Cargo.toml`, `Cargo.lock`, the Arch PKGBUILD and the
metainfo's releases, then commit that and push the tag `v0.1.0`. Pushing a
`vX.Y.Z` tag starts the workflow; Actions, Release, Run workflow builds a tag
again. The first job refuses a tag that doesn't name `Cargo.toml`'s version,
so the release is always what the tree says it is, and nothing is stamped in
the workflow.

Then the same two builds the nightly makes: `scripts/build-appimage` and
`scripts/package-arch` on one runner, `scripts/build-windows discord
--no-installer` in the Windows shell on another, with the Windows Skia build,
cargo-xwin's MSVC files and the compiled Rust dependencies cached under the
nightly's keys. A third job, on a Windows runner, compiles the installer that
the nightly has to go without: it unpacks the portable ZIP and runs the same
Inno Setup 7.1.0 over the same files as `scripts/package-windows.py` does
under Wine, with the same defines, so the two produce the same
`Mincord-VERSION-windows-x64-setup.exe`. The last job publishes the AppImage,
its `.zsync` file, the Arch package, the Windows ZIP and installer, the source
snapshot and `SHA256SUMS.txt` as the latest release, with the changes since
the previous stable release in its notes. It is not a prerelease, so it is
what the AppImage's update information and the Windows updater find.

## Nightly releases

A nightly of `main` builds on GitHub too: Actions,
Nightly, Run workflow. `.github/workflows/nightly.yml` names it after the next
minor version, the day (UTC) and the workflow's run number, such as
`v0.2.0-nightly.20261020.5`, and titles it as the stable releases are, with the
commit: `Mincord 0.2.0-nightly.20261020.5 (396abe2)`. The run number puts two
nightlies of one day in order, as a commit hash wouldn't, and gives a second
run on one commit a tag of its own. `scripts/set-version` writes that version
into `Cargo.toml` and `Cargo.lock` before building, so the app reports it and
the packages and `SOURCE.txt` name it. One job runs `scripts/build-appimage`
and `scripts/package-arch`, and another `scripts/build-windows` in the Windows shell, with the Windows Skia
build, cargo-xwin's MSVC files and the compiled Rust dependencies
(`Swatinem/rust-cache`) cached between runs. The Linux build has no cache: it
compiles in the Nix sandbox, which can't reach one. The last job publishes the
AppImage, its `.zsync` file, the Arch package, the Windows ZIP, the source snapshot and
`SHA256SUMS.txt` as a prerelease on that commit. Wine doesn't start on the
runner, so there's no installer. The release's notes list the commits since the
previous nightly, or the latest release, each with its author's GitHub account:
features and fixes by their Conventional Commit type, the rest folded away
(`.github/nightly-notes.jq`). Every nightly stays on the releases page. Both
builds zip the source snapshot with the commit's time on every file, so they
make the same file, and the release carries one. A prerelease is never
`latest`, so the AppImage's update information leaves stable installs on stable
releases, and takes a nightly AppImage to the latest stable release, not the
next nightly.

## Updating in place on Windows

The Windows build updates itself; nothing else does. A Linux release is an
AppImage with a `.zsync` file, which AppImageUpdate and Gear Lever follow,
or a package pacman owns, and a self-updater there would fight whatever
installed it. `update::SUPPORTED` is what says which build this is.

Twenty seconds after the window opens, and every six hours a running
Mincord stays open, it asks GitHub for
`/repos/mincord/mincord/releases/latest`
([`src/update.rs`](../src/update.rs)). A release counts when it is not a
prerelease, its tag names a later version than this build, and it carries
the `-windows-x64-setup.exe` asset. Versions compare as three numbers and
a suffix, which orders a nightly before the release it is named for, as
[nightlies](#nightly-releases) and pacman order it. Nothing is downloaded
until the person asks, so a release they say Later to costs that one
request, and a release waved away isn't offered again until Mincord next
starts.

The card in the window's bottom corner ([`ui/update.slint`](../ui/update.slint))
offers it. Update downloads the same installer the releases page carries,
into `updates/` beside Mincord's data, written to the file as it arrives
rather than held in memory: an installer is around 57 MiB. It goes to a
`.part` file first and is moved into place whole, so an interrupted
download is never run. The bar moves every half percent, and moves the
card's own properties rather than refreshing the window, since the
installer comes down in hundreds of chunks. Cancel aborts the task, which
drops the request with it.

Then Mincord runs the installer with `/VERYSILENT /SUPPRESSMSGBOXES
/NORESTART /NOCANCEL /restartapp=1` and quits a moment later. It quits
rather than letting `CloseApplications=force` close it: Mincord goes on
running in the tray with its window shut, so a close request wouldn't end
it, and Windows won't replace a running executable. `/restartapp=1` is a
switch of Mincord's own, which `[Code]`'s `RestartApp` in
`packaging/windows/mincord.iss` reads and a `[Run]` entry with
`skipifnotsilent` acts on; the wizard's own "Launch Mincord" is
`skipifsilent`, so only one of the two ever fires. The installer is the
per-user one, so none of this asks for administrator rights.

`MINCORD_NO_UPDATE`, set to anything, stops Mincord asking at all, for a
development build that shouldn't reach GitHub.

The check runs against the real GitHub, so there's no fake for it. What
decides an offer is apart from the request, in `update::offer`, and tested
there.

## The app icon

`packaging/icon.svg` is Mincord's icon: the logo in coral with its message in
white, on a rounded graphite plate, so it reads on a light taskbar as well as a
dark one. The UI keeps drawing `ui/icons/logo.svg`, which carries no plate and
takes the colour it is drawn in.

On Linux the window's app ID is `io.github.mincord.Mincord` (`src/icon.rs`).
Wayland desktops show a window's icon only through the desktop entry of that
name, `packaging/linux/io.github.mincord.Mincord.desktop`;
`scripts/install-desktop-entry [EXECUTABLE]` installs it and the icon for
this user, running `target/release/mincord` unless told otherwise. The
AppImage installs nothing, so Mincord run from one writes the entry and the
icon to `~/.local/share` itself before it opens its window
(`src/desktop_entry.rs`), with Exec and TryExec naming the AppImage. It
rewrites them when the AppImage has moved, and leaves them alone when a package
has its own entry in `XDG_DATA_DIRS`, or an AppImage manager such as Gear Lever
made one that runs the same file. On X11 the window also carries the icon
itself. Notifications name the entry, so the notification server shows the
same name and icon.

On Windows, `build.rs` compiles `packaging/windows/mincord.rc` with `llvm-rc`
and links the icon into `mincord.exe`, for Explorer and shortcuts; the window
loads it from there for its title bar and the taskbar, each at the size
Windows draws it. After changing `icon.svg`, run `scripts/make-windows-icon`
to draw `mincord.ico` again.
