//! macOS process-group ownership. Polling never reaps the leader before group termination.
use crate::{EndpointError, Result};
use gateway_core::CommandSpec;
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use rustix::process::{WaitId, WaitIdOptions, waitid};
use std::{
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::process::{Child, Command};

pub struct OwnedProcess {
    child: Child,
    group: Pid,
    pid: rustix::process::Pid,
    owned: bool,
}
impl OwnedProcess {
    /// No shell interpolation, inherited credentials, or captured payload output.
    pub fn spawn(spec: &CommandSpec) -> Result<Self> {
        spec.validate().map_err(|_| EndpointError::Invalid)?;
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        if let Some(home) = std::env::var_os("HOME") {
            command.env("HOME", home);
        }
        command
            .envs(&spec.env)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .process_group(0);
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        let child = command.spawn().map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => EndpointError::MissingExecutable,
            std::io::ErrorKind::PermissionDenied => EndpointError::ExecutionDenied,
            _ => EndpointError::Invalid,
        })?;
        let raw = child
            .id()
            .and_then(|id| i32::try_from(id).ok())
            .ok_or(EndpointError::Invalid)?;
        Ok(Self {
            child,
            group: Pid::from_raw(raw),
            pid: rustix::process::Pid::from_raw(raw).ok_or(EndpointError::Invalid)?,
            owned: true,
        })
    }
    pub fn id(&self) -> u32 {
        self.group.as_raw() as u32
    }
    /// Some(code) means exited; code=None means signalled. The leader remains a zombie/reserved PID.
    pub fn poll_exit(&self) -> Result<Option<Option<i32>>> {
        waitid(
            WaitId::Pid(self.pid),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        )
        .map(|status| status.map(|status| status.exit_status()))
        .map_err(|_| EndpointError::Invalid)
    }
    /// Darwin may return EPERM for an existing group containing only zombies.
    fn zombie_only_group(&self) -> Result<bool> {
        if self.poll_exit()?.is_none() {
            return Ok(false);
        }
        // libproc distinguishes zero results from an error through errno; clear a previous killpg error.
        nix::errno::Errno::clear();
        let pids =
            libproc::processes::pids_by_type(libproc::processes::ProcFilter::ByProgramGroup {
                pgrpid: self.id(),
            })
            .map_err(|_| EndpointError::Invalid)?;
        if pids.len() > 4096 {
            return Err(EndpointError::Invalid);
        }
        for pid in pids {
            if pid == self.id() {
                continue;
            } // waitid already proves this unreaped leader is exited.
            let pid = i32::try_from(pid).map_err(|_| EndpointError::Invalid)?;
            let info = libproc::proc_pid::pidinfo::<libproc::bsd_info::BSDInfo>(pid, 0)
                .map_err(|_| EndpointError::Invalid)?;
            if info.pbi_status != nix::libc::SZOMB {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// TERM, bounded grace, KILL all members while leader is unreaped, then reap only this owned child.
    pub async fn terminate(mut self, grace: Duration) -> Result<ExitStatus> {
        if grace > Duration::from_secs(5) {
            return Err(EndpointError::Invalid);
        }
        let _ = killpg(self.group, Signal::SIGTERM);
        let deadline = tokio::time::Instant::now() + grace;
        let term_deadline = deadline - grace.min(Duration::from_secs(1)) / 4;
        while tokio::time::Instant::now() < term_deadline {
            if matches!(self.poll_exit(), Ok(Some(_))) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let killed = killpg(self.group, Signal::SIGKILL);
        let zombie_only = killed == Err(nix::errno::Errno::EPERM) && self.zombie_only_group()?;
        if !matches!(killed, Ok(()) | Err(nix::errno::Errno::ESRCH)) && !zombie_only {
            return Err(EndpointError::Invalid);
        }
        // No further group signals after wait() may release the leader PID.
        let status = tokio::time::timeout_at(deadline, self.child.wait())
            .await
            .map_err(|_| EndpointError::Invalid)?
            .map_err(|_| EndpointError::Invalid)?;
        self.owned = false;
        Ok(status)
    }
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        if self.owned && self.child.id() == Some(self.id()) {
            let _ = killpg(self.group, Signal::SIGKILL);
        }
    }
}
