use std::collections::HashMap;
use std::env;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use env_logger::Env;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use indicatif_log_bridge::LogWrapper;

use crate::models::{FileInfo, FileType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressMode {
    Quiet,
    Default,
    Verbose,
}

#[derive(Debug, Default, Clone)]
pub struct ScanStats {
    pub processes: usize,
    pub scan_names: String,
    pub initial_files: usize,
    pub initial_dirs: usize,
    pub initial_size: u64,
    pub excluded_count: usize,
    pub final_files: usize,
    pub final_dirs: usize,
    pub final_size: u64,
    pub error_count: usize,
    pub total_bytes_scanned: u64,
    pub packages_assembled: usize,
    pub manifests_seen: usize,
    pub phase_timings: Vec<(String, f64)>,
}

pub struct ScanProgress {
    mode: ProgressMode,
    multi: MultiProgress,
    scan_bar: ProgressBar,
    stats: Mutex<ScanStats>,
    phase_starts: Mutex<HashMap<&'static str, Instant>>,
    phase_spinner: Mutex<Option<ProgressBar>>,
    started_at: Instant,
    stderr_is_tty: bool,
}

impl ScanProgress {
    pub fn new(mode: ProgressMode) -> Self {
        let stderr_is_tty = std::io::stderr().is_terminal();
        let multi = match mode {
            ProgressMode::Quiet => MultiProgress::with_draw_target(ProgressDrawTarget::hidden()),
            ProgressMode::Default if stderr_is_tty => {
                MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(15))
            }
            ProgressMode::Default | ProgressMode::Verbose => {
                MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
            }
        };

        let scan_bar = if mode == ProgressMode::Default && stderr_is_tty {
            multi.add(ProgressBar::new(0))
        } else {
            ProgressBar::hidden()
        };

        scan_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({per_sec}) ({eta})",
                )
                .expect("Failed to create progress bar style")
                .progress_chars("#>-"),
        );

        Self {
            mode,
            multi,
            scan_bar,
            stats: Mutex::new(ScanStats::default()),
            phase_starts: Mutex::new(HashMap::new()),
            phase_spinner: Mutex::new(None),
            started_at: Instant::now(),
            stderr_is_tty,
        }
    }

    pub fn set_processes(&self, processes: usize) {
        let mut stats = self.stats.lock().expect("stats lock poisoned");
        stats.processes = processes;
    }

    pub fn set_scan_names(&self, scan_names: String) {
        let mut stats = self.stats.lock().expect("stats lock poisoned");
        stats.scan_names = scan_names;
    }

    pub fn init_logging_bridge(&self) {
        if self.mode == ProgressMode::Quiet {
            return;
        }

        let logger =
            env_logger::Builder::from_env(Env::default().default_filter_or("warn")).build();
        let level = logger.filter();
        if LogWrapper::new(self.multi.clone(), logger)
            .try_init()
            .is_ok()
        {
            log::set_max_level(level);
        }
    }

    pub fn start_discovery(&self) {
        self.start_phase("discovery");
        match self.mode {
            ProgressMode::Quiet => {}
            ProgressMode::Default => {
                self.start_spinner("Collecting files...");
            }
            ProgressMode::Verbose => {
                self.message("Collecting files...");
            }
        }
    }

    pub fn finish_discovery(&self, files: usize, dirs: usize, size: u64, excluded: usize) {
        self.finish_spinner();
        self.finish_phase("discovery");
        let mut stats = self.stats.lock().expect("stats lock poisoned");
        stats.initial_files = files;
        stats.initial_dirs = dirs;
        stats.initial_size = size;
        stats.excluded_count = excluded;
    }

    pub fn start_spdx_load(&self) {
        self.start_phase("spdx_load");
        self.message("Loading SPDX data, this may take a while...");
    }

    pub fn finish_spdx_load(&self) {
        self.finish_phase("spdx_load");
    }

    pub fn start_scan(&self, total_files: usize) {
        self.start_phase("scan");
        self.scan_bar.set_length(total_files as u64);
        self.scan_bar.set_position(0);
    }

    pub fn file_completed(&self, path: &Path, bytes: u64, scan_errors: &[String]) {
        self.scan_bar.inc(1);
        let mut stats = self.stats.lock().expect("stats lock poisoned");
        stats.total_bytes_scanned += bytes;

        let has_error = !scan_errors.is_empty();
        if has_error {
            stats.error_count += 1;
        }
        drop(stats);

        match self.mode {
            ProgressMode::Quiet => {}
            ProgressMode::Default => {
                if has_error {
                    self.error(&format!("Path: {}", path.to_string_lossy()));
                }
            }
            ProgressMode::Verbose => {
                self.message(&path.to_string_lossy());
                for err in scan_errors {
                    for line in err.lines() {
                        self.error(&format!("  {line}"));
                    }
                }
            }
        }
    }

    pub fn record_runtime_error(&self, path: &Path, err: &str) {
        let mut stats = self.stats.lock().expect("stats lock poisoned");
        stats.error_count += 1;
        drop(stats);

        match self.mode {
            ProgressMode::Quiet => {}
            ProgressMode::Default => self.error(&format!("Path: {}", path.to_string_lossy())),
            ProgressMode::Verbose => {
                self.error(&format!("Path: {}", path.to_string_lossy()));
                for line in err.lines() {
                    self.error(&format!("  {line}"));
                }
            }
        }
    }

    pub fn finish_scan(&self) {
        self.finish_phase("scan");
        if self.mode == ProgressMode::Default && self.stderr_is_tty {
            self.scan_bar.finish_with_message("Scan complete!");
        } else {
            self.scan_bar.finish_and_clear();
        }
    }

    pub fn start_assembly(&self) {
        self.start_phase("assembly");
        match self.mode {
            ProgressMode::Quiet => {}
            ProgressMode::Default => self.start_spinner("Assembling packages..."),
            ProgressMode::Verbose => self.message("Assembling packages..."),
        }
    }

    pub fn finish_assembly(&self, packages_assembled: usize, manifests_seen: usize) {
        self.finish_spinner();
        self.finish_phase("assembly");
        let mut stats = self.stats.lock().expect("stats lock poisoned");
        stats.packages_assembled = packages_assembled;
        stats.manifests_seen = manifests_seen;
    }

    pub fn start_output(&self) {
        self.start_phase("output");
        match self.mode {
            ProgressMode::Quiet => {}
            ProgressMode::Default => self.start_spinner("Writing output..."),
            ProgressMode::Verbose => self.message("Writing output..."),
        }
    }

    pub fn output_written(&self, text: &str) {
        self.message(text);
    }

    pub fn finish_output(&self) {
        self.finish_spinner();
        self.finish_phase("output");
    }

    pub fn record_final_counts(&self, files: &[FileInfo]) {
        let mut stats = self.stats.lock().expect("stats lock poisoned");
        stats.final_files = files
            .iter()
            .filter(|f| f.file_type == FileType::File)
            .count();
        stats.final_dirs = files
            .iter()
            .filter(|f| f.file_type == FileType::Directory)
            .count();
        stats.final_size = files
            .iter()
            .filter(|f| f.file_type == FileType::File)
            .map(|f| f.size)
            .sum();
    }

    pub fn display_summary(&self, scan_start: &str, scan_end: &str) {
        if self.mode == ProgressMode::Quiet {
            return;
        }

        let mut stats = self.stats.lock().expect("stats lock poisoned");
        let total = self.started_at.elapsed().as_secs_f64();
        stats
            .phase_timings
            .push(("total".to_string(), total.max(0.0)));

        if stats.error_count > 0 {
            self.error("Some files failed to scan properly:");
        }

        let speed_files = if total > 0.0 {
            stats.final_files as f64 / total
        } else {
            0.0
        };
        let speed_bytes = if total > 0.0 {
            stats.total_bytes_scanned as f64 / total
        } else {
            0.0
        };

        self.message("Scanning done.");
        let processes = if stats.processes > 0 {
            stats.processes
        } else {
            num_cpus_for_display()
        };
        let scan_names = if stats.scan_names.is_empty() {
            "scan".to_string()
        } else {
            stats.scan_names.clone()
        };
        self.message(&format!(
            "Summary:        {scan_names} with {processes} process(es)"
        ));
        self.message(&format!("Errors count:   {}", stats.error_count));
        self.message(&format!(
            "Scan Speed:     {speed_files:.2} files/sec. {}/sec.",
            format_size(speed_bytes as u64)
        ));
        self.message(&format!(
            "Initial counts: {} resource(s): {} file(s) and {} directorie(s) for {}",
            stats.initial_files + stats.initial_dirs,
            stats.initial_files,
            stats.initial_dirs,
            format_size(stats.initial_size)
        ));
        self.message(&format!(
            "Final counts:   {} resource(s): {} file(s) and {} directorie(s) for {}",
            stats.final_files + stats.final_dirs,
            stats.final_files,
            stats.final_dirs,
            format_size(stats.final_size)
        ));
        self.message(&format!("Excluded count: {}", stats.excluded_count));
        self.message(&format!(
            "Packages:       {} assembled from {} manifests",
            stats.packages_assembled, stats.manifests_seen
        ));
        self.message("Timings:");
        self.message(&format!("  scan_start: {scan_start}"));
        self.message(&format!("  scan_end:   {scan_end}"));
        for (name, value) in &stats.phase_timings {
            self.message(&format!("  {name}: {value:.2}s"));
        }
    }

    fn message(&self, msg: &str) {
        if self.mode == ProgressMode::Quiet {
            return;
        }

        if self.mode == ProgressMode::Default && self.stderr_is_tty {
            let _ = self.multi.println(msg);
        } else {
            eprintln!("{msg}");
        }
    }

    fn error(&self, msg: &str) {
        if self.mode == ProgressMode::Quiet {
            return;
        }

        if supports_color(self.stderr_is_tty) {
            self.message(&format!("\u{1b}[31m{msg}\u{1b}[0m"));
        } else {
            self.message(msg);
        }
    }

    fn start_phase(&self, phase: &'static str) {
        self.phase_starts
            .lock()
            .expect("phase lock poisoned")
            .insert(phase, Instant::now());
    }

    fn finish_phase(&self, phase: &'static str) {
        let start = self
            .phase_starts
            .lock()
            .expect("phase lock poisoned")
            .remove(phase);
        if let Some(start) = start {
            let mut stats = self.stats.lock().expect("stats lock poisoned");
            stats
                .phase_timings
                .push((phase.to_string(), start.elapsed().as_secs_f64()));
        }
    }

    fn start_spinner(&self, message: &str) {
        if self.mode != ProgressMode::Default || !self.stderr_is_tty {
            self.message(message);
            return;
        }

        let spinner = self.multi.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Failed to create spinner style"),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        spinner.set_message(message.to_string());
        *self
            .phase_spinner
            .lock()
            .expect("phase spinner lock poisoned") = Some(spinner);
    }

    fn finish_spinner(&self) {
        if let Some(spinner) = self
            .phase_spinner
            .lock()
            .expect("phase spinner lock poisoned")
            .take()
        {
            spinner.finish_and_clear();
        }
    }
}

fn supports_color(stderr_is_tty: bool) -> bool {
    if !stderr_is_tty {
        return false;
    }
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    !matches!(env::var("TERM"), Ok(term) if term == "dumb")
}

pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 Bytes".to_string();
    }
    if bytes == 1 {
        return "1 Byte".to_string();
    }

    let mut size = bytes as f64;
    let units = ["Bytes", "KB", "MB", "GB", "TB"];
    let mut idx = 0;
    while size >= 1024.0 && idx < units.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }

    if idx == 0 {
        format!("{} {}", bytes, units[idx])
    } else {
        format!("{size:.2} {}", units[idx])
    }
}

fn num_cpus_for_display() -> usize {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    if cpus > 1 { cpus - 1 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::format_size;

    #[test]
    fn format_size_matches_expected_shape() {
        assert_eq!(format_size(0), "0 Bytes");
        assert_eq!(format_size(1), "1 Byte");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(2_567_000), "2.45 MB");
    }
}
