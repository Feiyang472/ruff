mod args;
mod logging;
mod printer;
mod python_version;
mod version;

use std::fmt::Write;
use std::process::{ExitCode, Termination};
use std::sync::Mutex;

use anyhow::Result;
use anyhow::{Context, anyhow};
use clap::{CommandFactory, Parser};
use colored::Colorize;
use crossbeam::channel as crossbeam_channel;
use rayon::ThreadPoolBuilder;
use ruff_db::cancellation::{Canceled, CancellationToken, CancellationTokenSource};
use ruff_db::diagnostic::{
    Diagnostic, DiagnosticId, DisplayDiagnosticConfig, DisplayDiagnostics, Severity,
};
use ruff_db::files::File;
use ruff_db::system::{OsSystem, SystemPath, SystemPathBuf};
use ruff_db::{STACK_SIZE, max_parallelism};
use salsa::Database;
use ty_project::metadata::options::ProjectOptionsOverrides;
use ty_project::metadata::settings::TerminalSettings;
use ty_project::watch::ProjectWatcher;
use ty_project::{CollectReporter, Db, suppress_all_diagnostics, watch};
use ty_project::{ProjectDatabase, ProjectMetadata};
use ty_server::run_server;
use ty_static::EnvVars;

use crate::args::{CheckCommand, Command, DiagramType, TerminalColor, UmlCommand};
use crate::logging::{VerbosityLevel, setup_tracing};
use crate::printer::Printer;
pub use args::Cli;

pub fn run() -> anyhow::Result<ExitStatus> {
    setup_rayon();
    ruff_db::set_program_version(crate::version::version().to_string()).unwrap();

    let args = wild::args_os();
    let args = argfile::expand_args_from(args, argfile::parse_fromfile, argfile::PREFIX)
        .context("Failed to read CLI arguments from file")?;
    let args = Cli::parse_from(args);

    match args.command {
        Command::Server => run_server().map(|()| ExitStatus::Success),
        Command::Check(check_args) => run_check(check_args),
        Command::Uml(uml_args) => run_uml(uml_args),
        Command::Version => version().map(|()| ExitStatus::Success),
        Command::GenerateShellCompletion { shell } => {
            use std::io::stdout;

            shell.generate(&mut Cli::command(), &mut stdout());
            Ok(ExitStatus::Success)
        }
    }
}

pub(crate) fn version() -> Result<()> {
    let mut stdout = Printer::default().stream_for_requested_summary().lock();
    let version_info = crate::version::version();
    writeln!(stdout, "ty {}", &version_info)?;
    Ok(())
}

fn run_uml(args: UmlCommand) -> anyhow::Result<ExitStatus> {
    use std::io::Write as IoWrite;
    use ty_uml::{ExtractionConfig, OutputFormat, RenderConfig, extract_diagram, render_to_string};

    let verbosity = args.verbosity.level();
    let _guard = setup_tracing(verbosity, TerminalColor::Auto)?;

    tracing::debug!("Starting UML generation");

    // Get the current working directory
    let cwd = {
        let cwd = std::env::current_dir().context("Failed to get the current working directory")?;
        SystemPathBuf::from_path_buf(cwd).map_err(|path| {
            anyhow!(
                "The current working directory `{}` contains non-Unicode characters.",
                path.display()
            )
        })?
    };

    // Determine the project path
    let project_path = args
        .project
        .as_ref()
        .map(|project| {
            if project.as_std_path().is_dir() {
                Ok(SystemPath::absolute(project, &cwd))
            } else {
                Err(anyhow!(
                    "Provided project path `{project}` is not a directory"
                ))
            }
        })
        .transpose()?
        .unwrap_or_else(|| cwd.clone());

    // Get paths to analyze
    let analyze_paths: Vec<_> = if args.paths.is_empty() {
        vec![project_path.clone()]
    } else {
        args.paths
            .iter()
            .map(|path| SystemPath::absolute(path, &cwd))
            .collect()
    };

    // Create the system and database
    let system = OsSystem::new(&cwd);
    let project_metadata = ProjectMetadata::discover(&project_path, &system)?;
    let db = ProjectDatabase::new(project_metadata, system)?;

    // Configure extraction
    let extraction_config = ExtractionConfig {
        include_private: args.include_private,
        include_dunder: args.include_dunder,
        include_inherited: false,
        max_depth: Some(args.max_depth),
        module_filter: None,
        extract_classes: matches!(args.diagram_type, DiagramType::Class | DiagramType::All),
        extract_modules: matches!(args.diagram_type, DiagramType::Module | DiagramType::All),
        extract_calls: matches!(args.diagram_type, DiagramType::Call | DiagramType::All),
    };

    // Configure rendering
    let render_config = RenderConfig {
        title: args.title,
        show_visibility: args.show_visibility,
        show_types: args.show_types,
        show_parameters: true,
        group_by_module: true,
        direction: ty_uml::render::DiagramDirection::TopToBottom,
        color_scheme: ty_uml::render::ColorScheme::Default,
        show_external_refs: false,
        max_type_width: Some(40),
    };

    // Extract diagram from all files in the project
    let mut diagram = ty_uml::UmlDiagram::new();

    for path in &analyze_paths {
        // Collect Python files
        let files = collect_python_files(&db, path);

        for file in files {
            let file_diagram = extract_diagram(&db, file, &extraction_config);
            diagram.merge(file_diagram);
        }
    }

    // Render the diagram
    let output_format: OutputFormat = args.format.into();
    let output = render_to_string(&diagram, output_format, &render_config);

    // Write output
    if let Some(output_path) = args.output {
        let output_path = SystemPath::absolute(&output_path, &cwd);
        std::fs::write(output_path.as_std_path(), &output)
            .context("Failed to write output file")?;
        tracing::info!("Wrote UML diagram to {}", output_path);
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(output.as_bytes())?;
    }

    // Report statistics
    let stats = diagram.stats();
    tracing::info!(
        "Generated diagram with {} classes, {} relationships, {} modules, {} functions, {} calls",
        stats.class_count,
        stats.relationship_count,
        stats.module_count,
        stats.function_count,
        stats.call_count
    );

    Ok(ExitStatus::Success)
}

/// Collect all Python files from a directory or single file.
fn collect_python_files(db: &ProjectDatabase, path: &SystemPath) -> Vec<File> {
    use ruff_db::files::system_path_to_file;

    let mut files = Vec::new();

    if path.as_std_path().is_file() {
        if let Ok(file) = system_path_to_file(db, path) {
            files.push(file);
        }
    } else if path.as_std_path().is_dir() {
        // Walk the directory and collect .py files
        fn walk_dir(db: &ProjectDatabase, dir: &std::path::Path, files: &mut Vec<File>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(Result::ok) {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if ext == "py" || ext == "pyi" {
                                if let Ok(system_path) =
                                    SystemPathBuf::from_path_buf(entry_path.clone())
                                {
                                    if let Ok(file) =
                                        ruff_db::files::system_path_to_file(db, &system_path)
                                    {
                                        files.push(file);
                                    }
                                }
                            }
                        }
                    } else if entry_path.is_dir() {
                        // Skip hidden directories and common non-source directories
                        if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                            if !name.starts_with('.')
                                && name != "__pycache__"
                                && name != "node_modules"
                                && name != ".git"
                            {
                                walk_dir(db, &entry_path, files);
                            }
                        }
                    }
                }
            }
        }

        walk_dir(db, path.as_std_path(), &mut files);
    }

    files
}

fn run_check(args: CheckCommand) -> anyhow::Result<ExitStatus> {
    // Enabled ANSI colors on Windows 10.
    #[cfg(windows)]
    assert!(colored::control::set_virtual_terminal(true).is_ok());

    set_colored_override(args.color);

    let verbosity = args.verbosity.level();
    let _guard = setup_tracing(verbosity, args.color.unwrap_or_default())?;

    let printer = Printer::new(verbosity, args.no_progress);

    tracing::debug!("Version: {}", version::version());

    // The base path to which all CLI arguments are relative to.
    let cwd = {
        let cwd = std::env::current_dir().context("Failed to get the current working directory")?;
        SystemPathBuf::from_path_buf(cwd)
            .map_err(|path| {
                anyhow!(
                    "The current working directory `{}` contains non-Unicode characters. ty only supports Unicode paths.",
                    path.display()
                )
            })?
    };

    let project_path = args
        .project
        .as_ref()
        .map(|project| {
            if project.as_std_path().is_dir() {
                Ok(SystemPath::absolute(project, &cwd))
            } else {
                Err(anyhow!(
                    "Provided project path `{project}` is not a directory"
                ))
            }
        })
        .transpose()?
        .unwrap_or_else(|| cwd.clone());

    let check_paths: Vec<_> = args
        .paths
        .iter()
        .map(|path| SystemPath::absolute(path, &cwd))
        .collect();

    let mode = if args.add_ignore {
        MainLoopMode::AddIgnore
    } else {
        MainLoopMode::Check
    };

    let system = OsSystem::new(&cwd);
    let watch = args.watch;
    let exit_zero = args.exit_zero;
    let config_file = args
        .config_file
        .as_ref()
        .map(|path| SystemPath::absolute(path, &cwd));
    let force_exclude = args.force_exclude();

    let mut project_metadata = match &config_file {
        Some(config_file) => {
            ProjectMetadata::from_config_file(config_file.clone(), &project_path, &system)?
        }
        None => ProjectMetadata::discover(&project_path, &system)?,
    };

    project_metadata.apply_configuration_files(&system)?;

    let project_options_overrides = ProjectOptionsOverrides::new(config_file, args.into_options());
    project_metadata.apply_overrides(&project_options_overrides);

    let mut db = ProjectDatabase::new(project_metadata, system)?;
    let project = db.project();

    project.set_verbose(&mut db, verbosity >= VerbosityLevel::Verbose);
    project.set_force_exclude(&mut db, force_exclude);

    if !check_paths.is_empty() {
        project.set_included_paths(&mut db, check_paths);
    }

    let (main_loop, main_loop_cancellation_token) =
        MainLoop::new(mode, project_options_overrides, printer);

    // Listen to Ctrl+C and abort the watch mode.
    let main_loop_cancellation_token = Mutex::new(Some(main_loop_cancellation_token));
    ctrlc::set_handler(move || {
        let mut lock = main_loop_cancellation_token.lock().unwrap();

        if let Some(token) = lock.take() {
            token.stop();
        }
    })?;

    let exit_status = if watch {
        main_loop.watch(&mut db)?
    } else {
        main_loop.run(&mut db)?
    };

    let mut stdout = printer.stream_for_requested_summary().lock();
    match std::env::var(EnvVars::TY_MEMORY_REPORT).as_deref() {
        Ok("short") => write!(stdout, "{}", db.salsa_memory_dump().display_short())?,
        Ok("mypy_primer") => write!(stdout, "{}", db.salsa_memory_dump().display_mypy_primer())?,
        Ok("full") => {
            write!(stdout, "{}", db.salsa_memory_dump().display_full())?;
        }
        Ok(other) => {
            tracing::warn!(
                "Unknown value for `TY_MEMORY_REPORT`: `{other}`. Valid values are `short`, `mypy_primer`, and `full`."
            );
        }
        Err(_) => {}
    }

    std::mem::forget(db);

    if exit_zero {
        Ok(ExitStatus::Success)
    } else {
        Ok(exit_status)
    }
}

#[derive(Copy, Clone)]
pub enum ExitStatus {
    /// Checking was successful and there were no errors.
    Success = 0,

    /// Checking was successful but there were errors.
    Failure = 1,

    /// Checking failed due to an invocation error (e.g. the current directory no longer exists, incorrect CLI arguments, ...)
    Error = 2,

    /// Internal ty error (panic, or any other error that isn't due to the user using the
    /// program incorrectly or transient environment errors).
    InternalError = 101,
}

impl ExitStatus {
    pub const fn is_internal_error(self) -> bool {
        matches!(self, ExitStatus::InternalError)
    }
}

impl Termination for ExitStatus {
    fn report(self) -> ExitCode {
        ExitCode::from(self as u8)
    }
}

struct MainLoop {
    mode: MainLoopMode,

    /// Sender that can be used to send messages to the main loop.
    sender: crossbeam_channel::Sender<MainLoopMessage>,

    /// Receiver for the messages sent **to** the main loop.
    receiver: crossbeam_channel::Receiver<MainLoopMessage>,

    /// The file system watcher, if running in watch mode.
    watcher: Option<ProjectWatcher>,

    /// Interface for displaying information to the user.
    printer: Printer,

    project_options_overrides: ProjectOptionsOverrides,

    /// Cancellation token that gets set by Ctrl+C.
    /// Used for long-running operations on the main thread. Operations on background threads
    /// use Salsa's cancellation mechanism.
    cancellation_token: CancellationToken,
}

impl MainLoop {
    fn new(
        mode: MainLoopMode,
        project_options_overrides: ProjectOptionsOverrides,
        printer: Printer,
    ) -> (Self, MainLoopCancellationToken) {
        let (sender, receiver) = crossbeam_channel::bounded(10);

        let cancellation_token_source = CancellationTokenSource::new();
        let cancellation_token = cancellation_token_source.token();

        (
            Self {
                mode,
                sender: sender.clone(),
                receiver,
                watcher: None,
                project_options_overrides,
                printer,
                cancellation_token,
            },
            MainLoopCancellationToken {
                sender,
                source: cancellation_token_source,
            },
        )
    }

    fn watch(mut self, db: &mut ProjectDatabase) -> Result<ExitStatus> {
        tracing::debug!("Starting watch mode");
        let sender = self.sender.clone();
        let watcher = watch::directory_watcher(move |event| {
            sender.send(MainLoopMessage::ApplyChanges(event)).unwrap();
        })?;

        self.watcher = Some(ProjectWatcher::new(watcher, db));
        self.run(db)?;

        Ok(ExitStatus::Success)
    }

    fn run(self, db: &mut ProjectDatabase) -> Result<ExitStatus> {
        self.sender.send(MainLoopMessage::CheckWorkspace).unwrap();

        let result = self.main_loop(db);

        tracing::debug!("Exiting main loop");

        result
    }

    fn main_loop(mut self, db: &mut ProjectDatabase) -> Result<ExitStatus> {
        // Schedule the first check.
        tracing::debug!("Starting main loop");

        let mut revision = 0u64;

        while let Ok(message) = self.receiver.recv() {
            match message {
                MainLoopMessage::CheckWorkspace => {
                    let db = db.clone();
                    let sender = self.sender.clone();

                    // Spawn a new task that checks the project. This needs to be done in a separate thread
                    // to prevent blocking the main loop here.
                    rayon::spawn(move || {
                        let mut reporter = IndicatifReporter::from(self.printer);
                        let bar = reporter.bar.clone();

                        match salsa::Cancelled::catch(|| {
                            db.check_with_reporter(&mut reporter);
                            reporter.bar.finish_and_clear();
                            reporter.collector.into_sorted(&db)
                        }) {
                            Ok(result) => {
                                // Send the result back to the main loop for printing.
                                sender
                                    .send(MainLoopMessage::CheckCompleted { result, revision })
                                    .unwrap();
                            }
                            Err(cancelled) => {
                                bar.finish_and_clear();
                                tracing::debug!("Check has been cancelled: {cancelled:?}");
                            }
                        }
                    });
                }

                MainLoopMessage::CheckCompleted {
                    result,
                    revision: check_revision,
                } => {
                    if check_revision != revision {
                        tracing::debug!(
                            "Discarding check result for outdated revision: current: {revision}, result revision: {check_revision}"
                        );
                        continue;
                    }

                    if db.project().files(db).is_empty() {
                        tracing::warn!("No python files found under the given path(s)");
                    }

                    let result = match self.mode {
                        MainLoopMode::Check => {
                            // TODO: We should have an official flag to silence workspace diagnostics.
                            if std::env::var("TY_MEMORY_REPORT").as_deref() == Ok("mypy_primer") {
                                return Ok(ExitStatus::Success);
                            }

                            self.write_diagnostics(db, &result)?;

                            if self.cancellation_token.is_cancelled() {
                                Err(Canceled)
                            } else {
                                Ok(result)
                            }
                        }
                        MainLoopMode::AddIgnore => {
                            if let Ok(result) =
                                suppress_all_diagnostics(db, result, &self.cancellation_token)
                            {
                                self.write_diagnostics(db, &result.diagnostics)?;

                                let terminal_settings = db.project().settings(db).terminal();
                                let is_human_readable =
                                    terminal_settings.output_format.is_human_readable();

                                if is_human_readable {
                                    writeln!(
                                        self.printer.stream_for_failure_summary(),
                                        "Added {} ignore comment{}",
                                        result.count,
                                        if result.count > 1 { "s" } else { "" }
                                    )?;
                                }

                                Ok(result.diagnostics)
                            } else {
                                Err(Canceled)
                            }
                        }
                    };

                    let exit_status = match result.as_deref() {
                        Ok([]) => ExitStatus::Success,
                        Ok(diagnostics) => {
                            let terminal_settings = db.project().settings(db).terminal();
                            exit_status_from_diagnostics(diagnostics, terminal_settings)
                        }
                        Err(Canceled) => ExitStatus::Success,
                    };

                    if exit_status.is_internal_error() {
                        tracing::warn!(
                            "A fatal error occurred while checking some files. Not all project files were analyzed. See the diagnostics list above for details."
                        );
                    }

                    if self.watcher.is_some() {
                        continue;
                    }

                    return Ok(exit_status);
                }

                MainLoopMessage::ApplyChanges(changes) => {
                    Printer::clear_screen()?;

                    revision += 1;
                    // Automatically cancels any pending queries and waits for them to complete.
                    db.apply_changes(changes, Some(&self.project_options_overrides));
                    if let Some(watcher) = self.watcher.as_mut() {
                        watcher.update(db);
                    }

                    self.sender.send(MainLoopMessage::CheckWorkspace).unwrap();
                }
                MainLoopMessage::Exit => {
                    // Cancel any pending queries and wait for them to complete.
                    db.trigger_cancellation();
                    return Ok(ExitStatus::Success);
                }
            }

            tracing::debug!("Waiting for next main loop message.");
        }

        Ok(ExitStatus::Success)
    }

    fn write_diagnostics(
        &self,
        db: &ProjectDatabase,
        diagnostics: &[Diagnostic],
    ) -> anyhow::Result<()> {
        let terminal_settings = db.project().settings(db).terminal();
        let is_human_readable = terminal_settings.output_format.is_human_readable();

        match diagnostics {
            [] => {
                if is_human_readable {
                    writeln!(
                        self.printer.stream_for_success_summary(),
                        "{}",
                        "All checks passed!".green().bold()
                    )?;
                }
            }
            diagnostics => {
                let diagnostics_count = diagnostics.len();

                let mut stdout = self.printer.stream_for_details().lock();

                // Only render diagnostics if they're going to be displayed, since doing
                // so is expensive.
                if stdout.is_enabled() {
                    let display_config = DisplayDiagnosticConfig::default()
                        .format(terminal_settings.output_format.into())
                        .color(colored::control::SHOULD_COLORIZE.should_colorize())
                        .with_cancellation_token(Some(self.cancellation_token.clone()))
                        .show_fix_diff(true);

                    write!(
                        stdout,
                        "{}",
                        DisplayDiagnostics::new(db, &display_config, diagnostics)
                    )?;
                }

                if !self.cancellation_token.is_cancelled() && is_human_readable {
                    writeln!(
                        self.printer.stream_for_failure_summary(),
                        "Found {} diagnostic{}",
                        diagnostics_count,
                        if diagnostics_count > 1 { "s" } else { "" }
                    )?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
enum MainLoopMode {
    Check,
    AddIgnore,
}

fn exit_status_from_diagnostics(
    diagnostics: &[Diagnostic],
    terminal_settings: &TerminalSettings,
) -> ExitStatus {
    if diagnostics.is_empty() {
        return ExitStatus::Success;
    }

    let mut max_severity = Severity::Info;
    let mut io_error = false;

    for diagnostic in diagnostics {
        max_severity = max_severity.max(diagnostic.severity());
        io_error = io_error || matches!(diagnostic.id(), DiagnosticId::Io);
    }

    if !max_severity.is_fatal() && io_error {
        return ExitStatus::Error;
    }

    match max_severity {
        Severity::Info => ExitStatus::Success,
        Severity::Warning => {
            if terminal_settings.error_on_warning {
                ExitStatus::Failure
            } else {
                ExitStatus::Success
            }
        }
        Severity::Error => ExitStatus::Failure,
        Severity::Fatal => ExitStatus::InternalError,
    }
}

/// A progress reporter for `ty check`.
struct IndicatifReporter {
    collector: CollectReporter,

    /// A reporter that is ready, containing a progress bar to report to.
    ///
    /// Initialization of the bar is deferred to [`ty_project::ProgressReporter::set_files`] so we
    /// do not initialize the bar too early as it may take a while to collect the number of files to
    /// process and we don't want to display an empty "0/0" bar.
    bar: indicatif::ProgressBar,

    printer: Printer,
}

impl From<Printer> for IndicatifReporter {
    fn from(printer: Printer) -> Self {
        Self {
            bar: indicatif::ProgressBar::hidden(),
            collector: CollectReporter::default(),
            printer,
        }
    }
}

impl ty_project::ProgressReporter for IndicatifReporter {
    fn set_files(&mut self, files: usize) {
        self.collector.set_files(files);

        self.bar.set_length(files as u64);
        self.bar.set_message("Checking");
        self.bar.set_style(
            indicatif::ProgressStyle::with_template(
                "{msg:8.dim} {bar:60.green/dim} {pos}/{len} files",
            )
            .unwrap()
            .progress_chars("--"),
        );
        self.bar.set_draw_target(self.printer.progress_target());
    }

    fn report_checked_file(&self, db: &ProjectDatabase, file: File, diagnostics: &[Diagnostic]) {
        self.collector.report_checked_file(db, file, diagnostics);
        self.bar.inc(1);
    }

    fn report_diagnostics(&mut self, db: &ProjectDatabase, diagnostics: Vec<Diagnostic>) {
        self.collector.report_diagnostics(db, diagnostics);
    }
}

#[derive(Debug)]
struct MainLoopCancellationToken {
    sender: crossbeam_channel::Sender<MainLoopMessage>,
    source: CancellationTokenSource,
}

impl MainLoopCancellationToken {
    fn stop(self) {
        self.source.cancel();
        self.sender.send(MainLoopMessage::Exit).unwrap();
    }
}

/// Message sent from the orchestrator to the main loop.
#[derive(Debug)]
enum MainLoopMessage {
    CheckWorkspace,
    CheckCompleted {
        /// The diagnostics that were found during the check.
        result: Vec<Diagnostic>,
        revision: u64,
    },
    ApplyChanges(Vec<watch::ChangeEvent>),
    Exit,
}

fn set_colored_override(color: Option<TerminalColor>) {
    let Some(color) = color else {
        return;
    };

    match color {
        TerminalColor::Auto => {
            colored::control::unset_override();
        }
        TerminalColor::Always => {
            colored::control::set_override(true);
        }
        TerminalColor::Never => {
            colored::control::set_override(false);
        }
    }
}

/// Initializes the global rayon thread pool to never use more than `TY_MAX_PARALLELISM` threads.
fn setup_rayon() {
    ThreadPoolBuilder::default()
        .num_threads(max_parallelism().get())
        .stack_size(STACK_SIZE)
        .build_global()
        .unwrap();
}
