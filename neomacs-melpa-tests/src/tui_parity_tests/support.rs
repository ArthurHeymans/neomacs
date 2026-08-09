use std::path::Path;

use neomacs_tui_tests::{TuiLaunch, TuiSession};

use crate::{EmacsRuntime, MelpaSandbox, PreparedPackageSet};

const QUIET_GNU_NATIVE_COMP: &str = "--eval=(progn(set'native-comp-jit-compilation())(set'native-comp-async-report-warnings-errors'silent)(push'(native-compiler)warning-suppress-types)(mapc'kill-process(process-list)))";

pub struct PackageTuiPair {
    pub gnu: TuiSession,
    pub neo: TuiSession,
    _gnu_sandbox: MelpaSandbox,
    _neo_sandbox: MelpaSandbox,
}

impl PackageTuiPair {
    pub fn spawn(label: &str, packages: &PreparedPackageSet) -> Result<Self, String> {
        let gnu_sandbox = MelpaSandbox::new(&format!("{label}-tui-gnu"))?;
        let neo_sandbox = MelpaSandbox::new(&format!("{label}-tui-neo"))?;
        let gnu_startup_file = packages.write_startup_file(gnu_sandbox.root())?;
        let neo_startup_file = packages.write_startup_file(neo_sandbox.root())?;

        let gnu_launch = editor_launch(
            EmacsRuntime::gnu_emacs(),
            &gnu_sandbox,
            packages,
            &gnu_startup_file,
            true,
        );
        let neo_launch = editor_launch(
            EmacsRuntime::neomacs(),
            &neo_sandbox,
            packages,
            &neo_startup_file,
            false,
        );

        Ok(Self {
            gnu: TuiSession::spawn_launch(gnu_launch, "GNU"),
            neo: TuiSession::spawn_launch(neo_launch, "NEO"),
            _gnu_sandbox: gnu_sandbox,
            _neo_sandbox: neo_sandbox,
        })
    }
}

fn editor_launch(
    runtime: EmacsRuntime,
    sandbox: &MelpaSandbox,
    packages: &PreparedPackageSet,
    startup_file: &Path,
    gnu: bool,
) -> TuiLaunch {
    let mut launch = TuiLaunch::new(runtime.executable.as_os_str()).args(["-nw", "-Q"]);
    if gnu {
        launch = launch.arg("-no-comp-spawn").arg(QUIET_GNU_NATIVE_COMP);
    }
    launch
        .arg("--load")
        .arg(startup_file.as_os_str())
        .envs(sandbox.process_environment())
        .envs(packages.process_environment())
        .env_remove("EMACSLOADPATH")
        .env("TERM", "screen-256color")
        .current_dir(sandbox.root())
}
