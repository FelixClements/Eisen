//! Persisted Hybrid Logical Clock (HLC) generator (P1.02).
//!
//! Provides a monotonic, restart-safe clock for local mutations and received
//! events, plus skew diagnostics. Storage is pluggable so the platform layer
//! can later supply an encrypted store (P1.09).

use crate::{DeviceId, Hlc};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Errors returned by the clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClockError {
    Storage(String),
    CounterExhausted,
    SkewDetected {
        received_wall: u64,
        physical: u64,
        threshold: u64,
    },
}

impl std::fmt::Display for ClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClockError::Storage(s) => write!(f, "clock storage error: {s}"),
            ClockError::CounterExhausted => write!(f, "HLC counter exhausted"),
            ClockError::SkewDetected {
                received_wall,
                physical,
                threshold,
            } => write!(
                f,
                "clock skew detected: received wall {received_wall} is more than {threshold} ms ahead of physical time {physical}"
            ),
        }
    }
}

impl std::error::Error for ClockError {}

/// Persistence interface for the clock state.
pub trait ClockStorage: Send + Sync {
    fn load(&self) -> Result<Option<(u64, u32)>, ClockError>;
    fn save(&self, wall: u64, counter: u32) -> Result<(), ClockError>;
}

/// In-memory storage for tests and transient clocks.
#[derive(Clone, Debug, Default)]
pub struct InMemoryStorage {
    state: Arc<Mutex<Option<(u64, u32)>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
        }
    }
}

impl ClockStorage for InMemoryStorage {
    fn load(&self) -> Result<Option<(u64, u32)>, ClockError> {
        Ok(*self.state.lock().unwrap())
    }

    fn save(&self, wall: u64, counter: u32) -> Result<(), ClockError> {
        *self.state.lock().unwrap() = Some((wall, counter));
        Ok(())
    }
}

/// Simple file storage for restart-safe HLC persistence.
///
/// Stores wall (big-endian u64) and counter (big-endian u32) atomically.
/// The HLC wall and counter are not secret, but this implementation will be
/// replaced/wrapped by the encrypted local store in P1.09.
pub struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ClockStorage for FileStorage {
    fn load(&self) -> Result<Option<(u64, u32)>, ClockError> {
        match fs::read(&self.path) {
            Ok(data) if data.is_empty() => Ok(None),
            Ok(data) if data.len() == 12 => {
                let wall = u64::from_be_bytes(data[0..8].try_into().unwrap());
                let counter = u32::from_be_bytes(data[8..12].try_into().unwrap());
                Ok(Some((wall, counter)))
            }
            Ok(_) => Err(ClockError::Storage(
                "clock file has unexpected length".into(),
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ClockError::Storage(e.to_string())),
        }
    }

    fn save(&self, wall: u64, counter: u32) -> Result<(), ClockError> {
        let mut data = Vec::with_capacity(12);
        data.extend_from_slice(&wall.to_be_bytes());
        data.extend_from_slice(&counter.to_be_bytes());

        let tmp = self.path.with_extension("tmp");
        let mut file = fs::File::create(&tmp).map_err(|e| ClockError::Storage(e.to_string()))?;
        file.write_all(&data)
            .map_err(|e| ClockError::Storage(e.to_string()))?;
        file.sync_all()
            .map_err(|e| ClockError::Storage(e.to_string()))?;
        drop(file);

        fs::rename(&tmp, &self.path).map_err(|e| ClockError::Storage(e.to_string()))?;
        Ok(())
    }
}

/// Monotonic, restart-safe Hybrid Logical Clock.
pub struct Clock<S: ClockStorage> {
    storage: S,
    device_id: DeviceId,
    wall: u64,
    counter: u32,
    skew_threshold: u64,
}

impl<S: ClockStorage> Clock<S> {
    /// Create or restore a clock from storage.
    ///
    /// If no state is persisted the clock starts at wall 0, counter 0; the
    /// first `now` or `receive` call will advance it using the supplied
    /// physical time.
    pub fn new(storage: S, device_id: DeviceId, skew_threshold: u64) -> Result<Self, ClockError> {
        let (wall, counter) = storage.load()?.unwrap_or((0, 0));
        Ok(Self {
            storage,
            device_id,
            wall,
            counter,
            skew_threshold,
        })
    }

    /// Generate an HLC for a local mutation.
    pub fn now(&mut self, physical: u64) -> Result<Hlc, ClockError> {
        self.advance(physical, None)
    }

    /// Merge a received HLC and return the next local HLC.
    ///
    /// If the received wall is more than `skew_threshold` ahead of `physical`,
    /// the function returns `Ok` with `Some(SkewInfo)` so the caller can
    /// pause new mutations and reconcile. The clock still advances to maintain
    /// monotonicity.
    pub fn receive(
        &mut self,
        physical: u64,
        received: Hlc,
    ) -> Result<(Hlc, Option<SkewInfo>), ClockError> {
        let skew = if received.wall.saturating_sub(physical) > self.skew_threshold {
            Some(SkewInfo {
                received_wall: received.wall,
                physical,
                threshold: self.skew_threshold,
            })
        } else {
            None
        };

        let hlc = self.advance(physical, Some(received))?;
        Ok((hlc, skew))
    }

    fn advance(&mut self, physical: u64, received: Option<Hlc>) -> Result<Hlc, ClockError> {
        let received_wall = received.map(|h| h.wall).unwrap_or(0);
        let received_counter = received.map(|h| h.counter).unwrap_or(0);

        let wall = physical.max(self.wall).max(received_wall);

        let counter = if wall == self.wall && wall == received_wall {
            self.counter
                .max(received_counter)
                .checked_add(1)
                .ok_or(ClockError::CounterExhausted)?
        } else if wall == self.wall {
            self.counter
                .checked_add(1)
                .ok_or(ClockError::CounterExhausted)?
        } else if wall == received_wall {
            received_counter
                .checked_add(1)
                .ok_or(ClockError::CounterExhausted)?
        } else {
            0
        };

        self.wall = wall;
        self.counter = counter;
        self.storage.save(wall, counter)?;

        Ok(Hlc {
            wall,
            counter,
            device_id: self.device_id,
        })
    }

    /// Current persisted wall and counter.
    pub fn state(&self) -> (u64, u32) {
        (self.wall, self.counter)
    }
}

/// Skew diagnostic returned by `Clock::receive`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkewInfo {
    pub received_wall: u64,
    pub physical: u64,
    pub threshold: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(n: u8) -> DeviceId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        DeviceId(bytes)
    }

    #[test]
    fn local_clock_advances_monotonically() {
        let storage = InMemoryStorage::new();
        let mut clock = Clock::new(storage, dev(1), 300_000).unwrap();

        let a = clock.now(1).unwrap();
        let b = clock.now(1).unwrap();
        let c = clock.now(2).unwrap();

        assert_eq!(a.wall, 1);
        assert_eq!(a.counter, 0);
        assert_eq!(b.wall, 1);
        assert_eq!(b.counter, 1);
        assert_eq!(c.wall, 2);
        assert_eq!(c.counter, 0);
        assert!(b > a);
        assert!(c > b);
    }

    #[test]
    fn receive_merges_hlc() {
        let storage = InMemoryStorage::new();
        let mut clock = Clock::new(storage, dev(1), 300_000).unwrap();

        let local = clock.now(5).unwrap();
        let received = Hlc {
            wall: 10,
            counter: 3,
            device_id: dev(2),
        };
        let (merged, skew) = clock.receive(5, received).unwrap();

        assert_eq!(merged.wall, 10);
        assert_eq!(merged.counter, 4);
        assert!(skew.is_none());
        assert!(merged > local);
        assert!(merged > received);
    }

    #[test]
    fn receive_with_later_physical_time() {
        let storage = InMemoryStorage::new();
        let mut clock = Clock::new(storage, dev(1), 300_000).unwrap();

        let received = Hlc {
            wall: 5,
            counter: 2,
            device_id: dev(2),
        };
        let (merged, _) = clock.receive(10, received).unwrap();

        assert_eq!(merged.wall, 10);
        assert_eq!(merged.counter, 0);
    }

    #[test]
    fn skew_detected_when_received_far_ahead() {
        let storage = InMemoryStorage::new();
        let mut clock = Clock::new(storage, dev(1), 300_000).unwrap();

        let received = Hlc {
            wall: 400_000,
            counter: 0,
            device_id: dev(2),
        };
        let (hlc, skew) = clock.receive(1, received).unwrap();

        assert_eq!(hlc.wall, 400_000);
        assert!(skew.is_some());
        let info = skew.unwrap();
        assert_eq!(info.received_wall, 400_000);
        assert_eq!(info.physical, 1);
        assert_eq!(info.threshold, 300_000);
    }

    #[test]
    fn restart_safe_from_storage() {
        let storage = InMemoryStorage::new();
        {
            let mut clock = Clock::new(storage.clone(), dev(1), 300_000).unwrap();
            clock.now(5).unwrap();
            clock.now(5).unwrap();
        }

        let mut clock2 = Clock::new(storage, dev(1), 300_000).unwrap();
        let next = clock2.now(5).unwrap();
        assert_eq!(next.wall, 5);
        assert_eq!(next.counter, 2);
    }

    #[test]
    fn clock_catches_up_after_restart() {
        let storage = InMemoryStorage::new();
        {
            let mut clock = Clock::new(storage.clone(), dev(1), 300_000).unwrap();
            clock.now(5).unwrap();
        }

        let mut clock2 = Clock::new(storage, dev(1), 300_000).unwrap();
        let next = clock2.now(10).unwrap();
        assert_eq!(next.wall, 10);
        assert_eq!(next.counter, 0);
    }

    #[test]
    fn file_storage_is_restart_safe() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();

        {
            let storage = FileStorage::new(&path);
            let mut clock = Clock::new(storage, dev(1), 300_000).unwrap();
            clock.now(5).unwrap();
            clock.now(5).unwrap();
        }

        let storage = FileStorage::new(&path);
        let mut clock = Clock::new(storage, dev(1), 300_000).unwrap();
        let next = clock.now(5).unwrap();
        assert_eq!(next.wall, 5);
        assert_eq!(next.counter, 2);
    }
}
