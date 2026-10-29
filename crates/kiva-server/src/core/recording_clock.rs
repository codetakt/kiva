//! Recording time belongs to the repository's serialized admission, not HTTP parsing.

use time::{Duration, OffsetDateTime};

use super::response::postgres_microsecond_timestamp;

/// A conservative watermark for an empty repository. No admitted recording can
/// precede it. Using the same precision on both backends keeps query boundaries
/// and the JSON `stored` value identical to the persistent column.
pub(crate) const EMPTY_WATERMARK: OffsetDateTime =
    OffsetDateTime::UNIX_EPOCH.saturating_sub(Duration::days(719_162));

#[derive(Debug, Clone, Default)]
pub(crate) struct RecordingClock {
    #[cfg(test)]
    fixed: std::sync::Arc<std::sync::RwLock<Option<OffsetDateTime>>>,
    #[cfg(test)]
    barrier: std::sync::Arc<std::sync::Mutex<Option<RecordingBarrier>>>,
}

impl RecordingClock {
    pub(crate) fn now(&self) -> OffsetDateTime {
        if let Some(fixed) = self.override_time() {
            return fixed;
        }
        OffsetDateTime::now_utc()
    }

    #[cfg_attr(not(test), allow(clippy::unused_self, clippy::missing_const_for_fn))]
    pub(crate) fn override_time(&self) -> Option<OffsetDateTime> {
        #[cfg(test)]
        return *self.fixed.read().expect("recording clock lock");
        #[cfg(not(test))]
        None
    }

    pub(crate) fn next(&self, previous: OffsetDateTime) -> OffsetDateTime {
        postgres_microsecond_timestamp(self.now()).max(previous + Duration::microseconds(1))
    }

    /// Instrument the actual admission boundary; production has no delay.
    #[cfg_attr(not(test), allow(clippy::unused_async))]
    pub(crate) async fn recorded(&self, instant: OffsetDateTime) {
        #[cfg(not(test))]
        let _ = instant;
        #[cfg(test)]
        {
            let barrier = self.barrier.lock().expect("recording barrier lock").take();
            if let Some(barrier) = barrier {
                let _ = barrier.assigned.send(instant);
                let _ = barrier.release.await;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn set(&self, instant: OffsetDateTime) {
        *self.fixed.write().expect("recording clock lock") = Some(instant);
    }
}

#[cfg(test)]
#[derive(Debug)]
struct RecordingBarrier {
    assigned: tokio::sync::oneshot::Sender<OffsetDateTime>,
    release: tokio::sync::oneshot::Receiver<()>,
}
