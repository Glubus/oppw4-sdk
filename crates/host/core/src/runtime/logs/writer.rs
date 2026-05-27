use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use crate::runtime::time;

pub(super) struct SessionLogWriter {
    root: PathBuf,
    file: Option<File>,
}

impl SessionLogWriter {
    pub(super) fn new(root: PathBuf) -> Self {
        Self { root, file: None }
    }

    pub(super) fn write(&mut self, message: &str, session_stamp: &str) -> std::io::Result<()> {
        let timestamp = time::line_timestamp();
        let file = self.file(session_stamp)?;
        writeln!(file, "[{timestamp}] {message}")?;
        file.flush()
    }

    fn file(&mut self, session_stamp: &str) -> std::io::Result<&mut File> {
        if self.file.is_none() {
            fs::create_dir_all(&self.root)?;
            let path = self.root.join(format!("{session_stamp}.log"));
            self.file = Some(OpenOptions::new().create(true).append(true).open(path)?);
        }
        Ok(self.file.as_mut().expect("log file was initialized"))
    }
}
