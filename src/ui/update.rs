//! The update offer in the window: when it is asked for, what the card
//! shows, and what taking the offer does. The check and the download are
//! `crate::update`; this is the part that borrows the adapter.
//!
//! Mincord asks GitHub a little after the window opens, so the first frames
//! aren't competing with it, and once every six hours after that. Nothing is
//! downloaded until the person asks for it, so a release they say Later to
//! costs one small request. A build that can't update itself never asks,
//! and neither does one with `MINCORD_NO_UPDATE` set, which keeps a
//! development build off GitHub.

use std::time::Duration;

use super::*;
use crate::update::{self, Release};

/// How long after the window opens the first check waits, and how often it
/// goes again.
const FIRST: Duration = Duration::from_secs(20);
const AGAIN: Duration = Duration::from_secs(6 * 60 * 60);
/// How long "Installing Mincord" shows before Mincord leaves the installer
/// to it.
const QUIT_AFTER: Duration = Duration::from_millis(1200);
/// How far the download has to move before the card says so again.
const STEP: f32 = 0.005;

#[derive(Default)]
pub(super) struct Updates {
    /// The release on offer, with how far taking it has got; none while
    /// there's nothing newer, or while the person has waved it away.
    offer: Option<Offer>,
    /// The repeated check.
    timer: slint::Timer,
    /// The download, which Cancel aborts. Aborting it drops the task that
    /// is downloading, which stops the request and the file it was
    /// writing; a Slint handle only detaches when it's dropped.
    download: Option<slint::JoinHandle<()>>,
    /// The release the person said Later to, which isn't offered again
    /// until Mincord next starts.
    waved_away: Option<String>,
}

struct Offer {
    release: Release,
    stage: UpdateStage,
    /// Under the title: how far the download has got, or why it failed.
    detail: SharedString,
    progress: f32,
}

impl Updates {
    /// Starts asking, once the window is up. The timer sits on the
    /// adapter, which outlives a window closed to the tray, so a Mincord
    /// left in the tray goes on checking.
    pub(super) fn watch(weak: std::rc::Weak<RefCell<Adapter>>) {
        if !update::SUPPORTED || std::env::var_os("MINCORD_NO_UPDATE").is_some() {
            return;
        }
        let Some(adapter) = weak.upgrade() else {
            return;
        };
        if adapter.borrow().updates.timer.running() {
            return;
        }
        // The first check, once the window has settled.
        let first = weak.clone();
        slint::Timer::single_shot(FIRST, move || check(first));
        // And every six hours, for a Mincord left running. A repeated
        // timer fires after its first interval, so this doesn't double up
        // with the one above.
        adapter
            .borrow()
            .updates
            .timer
            .start(slint::TimerMode::Repeated, AGAIN, move || {
                check(weak.clone());
            });
    }

    /// The card's state on the window, pushed only where it differs: this
    /// runs with every refresh, and setting a property draws again.
    pub(super) fn show(&self, window: &MincordWindow) {
        let Some(offer) = &self.offer else {
            if window.get_update_shown() {
                window.set_update_shown(false);
            }
            return;
        };
        if window.get_update_version() != offer.release.version.as_str() {
            window.set_update_version(offer.release.version.as_str().into());
            window.set_update_notes(offer.release.notes.as_str().into());
            window.set_update_running(update::running().into());
        }
        if window.get_update_stage() != offer.stage {
            window.set_update_stage(offer.stage);
        }
        if window.get_update_detail() != offer.detail {
            window.set_update_detail(offer.detail.clone());
        }
        if window.get_update_progress() != offer.progress {
            window.set_update_progress(offer.progress);
        }
        if !window.get_update_shown() {
            window.set_update_shown(true);
        }
    }
}

/// Asks GitHub what the latest release is, and offers it if it is newer.
fn check(weak: std::rc::Weak<RefCell<Adapter>>) {
    let Some(adapter) = weak.upgrade() else {
        return;
    };
    // Not over one already on offer, or one being downloaded.
    if adapter.borrow().updates.offer.is_some() {
        return;
    }
    let asking = adapter.borrow().runtime.spawn(update::latest());
    let task = slint::spawn_local(async move {
        let found = match asking.await {
            Ok(Ok(found)) => found,
            Ok(Err(error)) => {
                // A check that fails is the ordinary case offline. The next
                // one goes in six hours.
                tracing::debug!(%error, "could not ask for the latest release");
                return;
            }
            Err(_) => return,
        };
        let (Some(release), Some(adapter)) = (found, weak.upgrade()) else {
            return;
        };
        let window = adapter.borrow().window.upgrade();
        let mut adapter = adapter.borrow_mut();
        let updates = &mut adapter.updates;
        if updates.offer.is_some() || updates.waved_away.as_deref() == Some(&release.version) {
            return;
        }
        tracing::info!(version = %release.version, "a newer Mincord is out");
        updates.offer = Some(Offer {
            release,
            stage: UpdateStage::Offered,
            detail: SharedString::default(),
            progress: 0.0,
        });
        if let Some(window) = window {
            adapter.refresh(&window);
        }
    });
    if let Err(error) = task {
        tracing::warn!(%error, "could not check for an update");
    }
}

/// Update, or Try again after a failure: downloads the installer, showing
/// how far it has got, and hands it over once it is whole.
pub(super) fn take(adapter: &mut Adapter) {
    let Some(offer) = &mut adapter.updates.offer else {
        return;
    };
    let release = offer.release.clone();
    offer.stage = UpdateStage::Downloading;
    offer.progress = 0.0;
    offer.detail = downloaded(0, release.size);
    // Mincord keeps the installer it downloads beside its own data, and
    // the next download writes over it.
    let folder = adapter.root.join("updates");
    let weak = adapter.weak.clone();
    // The download sends how far it has got, and the window shows the last
    // of those: the file is written as it arrives, so nothing waits on the
    // UI keeping up.
    let (sender, mut progress) = tokio::sync::mpsc::unbounded_channel();
    let downloading = adapter.runtime.spawn(async move {
        update::download(&release, &folder, |so_far, expected| {
            let _ = sender.send((so_far, expected));
        })
        .await
    });
    let showing = weak.clone();
    let task = slint::spawn_local(async move {
        // The installer comes down in hundreds of chunks. Each one moves
        // the card's own properties, never the whole window, and only
        // once the bar has somewhere to go.
        let mut shown = 0.0;
        while let Some((so_far, expected)) = progress.recv().await {
            let Some(adapter) = showing.upgrade() else {
                return;
            };
            let window = adapter.borrow().window.upgrade();
            let mut adapter = adapter.borrow_mut();
            let Some(offer) = &mut adapter.updates.offer else {
                return;
            };
            if offer.stage != UpdateStage::Downloading {
                return;
            }
            let along = if expected == 0 {
                0.0
            } else {
                so_far as f32 / expected as f32
            };
            if along - shown < STEP && so_far < expected {
                continue;
            }
            shown = along;
            offer.detail = downloaded(so_far, expected);
            offer.progress = along;
            if let Some(window) = window {
                adapter.updates.show(&window);
            }
        }
        // The channel closes when the download ends, either way.
        let setup = match downloading.await {
            Ok(Ok(setup)) => Some(setup),
            Ok(Err(error)) => {
                tracing::warn!(%error, "could not download the update");
                None
            }
            // Cancelled.
            Err(_) => return,
        };
        let Some(adapter) = showing.upgrade() else {
            return;
        };
        let window = adapter.borrow().window.upgrade();
        let mut adapter = adapter.borrow_mut();
        let Some(offer) = &mut adapter.updates.offer else {
            return;
        };
        let Some(setup) = setup else {
            offer.stage = UpdateStage::Failed;
            offer.detail = "The download didn't finish. Check your connection.".into();
            offer.progress = 0.0;
            if let Some(window) = window {
                adapter.refresh(&window);
            }
            return;
        };
        offer.stage = UpdateStage::Installing;
        offer.detail = SharedString::default();
        if let Some(window) = &window {
            adapter.refresh(window);
        }
        // The installer puts the new Mincord in this one's place and
        // starts it again, which `/restartapp=1` asks it to do.
        if let Err(error) = update::install(&setup) {
            tracing::error!(%error, "could not start the update's installer");
            if let Some(offer) = &mut adapter.updates.offer {
                offer.stage = UpdateStage::Failed;
                offer.detail = "Mincord couldn't start the installer.".into();
            }
            if let Some(window) = window {
                adapter.refresh(&window);
            }
            return;
        }
        // Mincord goes rather than waiting to be closed: it keeps running
        // in the tray with its window shut, so a close request wouldn't end
        // it, and the installer can't replace a running executable. The
        // card shows what is happening for a moment first.
        tracing::info!("quitting for the update's installer");
        slint::Timer::single_shot(QUIT_AFTER, || {
            let _ = slint::quit_event_loop();
        });
    });
    match task {
        Ok(task) => adapter.updates.download = Some(task),
        Err(error) => {
            tracing::error!(%error, "could not start the update's download");
            if let Some(offer) = &mut adapter.updates.offer {
                offer.stage = UpdateStage::Failed;
                offer.detail = "Mincord couldn't start the download.".into();
            }
        }
    }
}

/// Later, or Cancel while it downloads: the card goes, and that release
/// isn't offered again until Mincord next starts.
pub(super) fn wave_away(adapter: &mut Adapter) {
    // Which drops the download it owns, stopping the request.
    if let Some(download) = adapter.updates.download.take() {
        download.abort();
    }
    if let Some(offer) = adapter.updates.offer.take() {
        adapter.updates.waved_away = Some(offer.release.version);
    }
}

/// What's new: the releases page in the browser, which has the notes in
/// full and the files the card doesn't download.
pub(super) fn open_notes() {
    super::browser::open(update::RELEASES);
}

/// "12.4 of 57.0 MB", or just what has come down where GitHub didn't say
/// how much there is.
fn downloaded(so_far: u64, expected: u64) -> SharedString {
    if expected == 0 {
        return format!("{} so far", megabytes(so_far)).into();
    }
    format!("{} of {}", megabytes(so_far), megabytes(expected)).into()
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}
