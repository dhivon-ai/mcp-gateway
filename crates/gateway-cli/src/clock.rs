//! Darwin continuous time includes sleep. Boot identity makes persisted anchors restart-safe.
use crate::{EndpointError, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const NANOS_PER_SECOND: u64 = 1_000_000_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClockSample {
    pub boot_id: Uuid,
    pub continuous_ns: u64,
    pub wall_ns: u64,
}
impl ClockSample {
    pub fn wall_seconds(self) -> Result<i64> {
        i64::try_from(self.wall_ns / NANOS_PER_SECOND).map_err(|_| EndpointError::ClockUncertain)
    }
    pub fn elapsed_since(self, earlier: Self) -> Result<u64> {
        if self.boot_id != earlier.boot_id || self.wall_ns < earlier.wall_ns {
            return Err(EndpointError::ClockUncertain);
        }
        self.continuous_ns
            .checked_sub(earlier.continuous_ns)
            .ok_or(EndpointError::ClockUncertain)
    }
}
pub trait Clock: Send + Sync {
    fn sample(&self) -> Result<ClockSample>;
}
pub struct NativeClock {
    #[cfg(target_os = "macos")]
    boot_id: Uuid,
}
impl NativeClock {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            use sysctl::Sysctl;
            let boot = sysctl::Ctl::new("kern.bootsessionuuid")
                .and_then(|ctl| ctl.value_string())
                .map_err(|_| EndpointError::ClockUncertain)?;
            let boot_id = Uuid::parse_str(&boot).map_err(|_| EndpointError::ClockUncertain)?;
            Ok(Self { boot_id })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(EndpointError::Unsupported)
        }
    }
}
impl Clock for NativeClock {
    fn sample(&self) -> Result<ClockSample> {
        #[cfg(target_os = "macos")]
        {
            // Apple Libc maps RAW to mach_continuous_time; UPTIME_RAW would exclude sleep.
            let time = nix::time::clock_gettime(nix::time::ClockId::from_raw(
                nix::libc::CLOCK_MONOTONIC_RAW,
            ))
            .map_err(|_| EndpointError::ClockUncertain)?;
            let seconds =
                u64::try_from(time.tv_sec()).map_err(|_| EndpointError::ClockUncertain)?;
            let nanos = u64::try_from(time.tv_nsec()).map_err(|_| EndpointError::ClockUncertain)?;
            if nanos >= NANOS_PER_SECOND {
                return Err(EndpointError::ClockUncertain);
            }
            let continuous_ns = seconds
                .checked_mul(NANOS_PER_SECOND)
                .and_then(|s| s.checked_add(nanos))
                .ok_or(EndpointError::ClockUncertain)?;
            let wall_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| u64::try_from(d.as_nanos()).ok())
                .ok_or(EndpointError::ClockUncertain)?;
            Ok(ClockSample {
                boot_id: self.boot_id,
                continuous_ns,
                wall_ns,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(EndpointError::Unsupported)
        }
    }
}
pub fn epoch_ns(seconds: i64) -> Result<u64> {
    u64::try_from(seconds)
        .ok()
        .and_then(|s| s.checked_mul(NANOS_PER_SECOND))
        .ok_or(EndpointError::InvalidCache)
}
