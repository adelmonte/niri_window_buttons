use futures::AsyncReadExt;
use thiserror::Error;
use waybar_cffi::gtk::{
    gio::{File, prelude::InputStreamExtManual, traits::FileExt},
    glib::{self, Priority},
};

pub struct ProcessInfo {
    pub parent_id: Option<i64>,
}

impl ProcessInfo {
    #[tracing::instrument(level = "TRACE")]
    pub async fn query(pid: i64) -> Result<Self, ProcessError> {
        let stat_file = File::for_path(format!("/proc/{pid}/stat"));

        let mut reader = stat_file
            .read_future(Priority::DEFAULT)
            .await
            .map_err(|e| ProcessError::FileOpen { e, pid })?
            .into_async_buf_read(4096);

        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .await
            .map_err(|e| ProcessError::FileRead { e, pid })?;

        let after_comm = content
            .rfind(')')
            .map(|i| &content[i + 1..])
            .ok_or_else(|| ProcessError::MalformedStat { pid })?;

        // Fields after "(comm)": state ppid pgrp session ...
        let ppid_str = after_comm
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| ProcessError::MalformedStat { pid })?;

        let ppid = ppid_str
            .parse()
            .map_err(|_| ProcessError::InvalidPpid { value: ppid_str.to_owned(), pid })?;

        Ok(Self {
            parent_id: if ppid == 0 { None } else { Some(ppid) },
        })
    }
}

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("malformed /proc/{pid}/stat: missing fields")]
    MalformedStat { pid: i64 },

    #[error("invalid PPID in /proc/{pid}/stat: {value}")]
    InvalidPpid { value: String, pid: i64 },

    #[error("cannot open /proc/{pid}/stat: {e}")]
    FileOpen {
        #[source]
        e: glib::Error,
        pid: i64,
    },

    #[error("cannot read /proc/{pid}/stat: {e}")]
    FileRead {
        #[source]
        e: futures::io::Error,
        pid: i64,
    },
}
