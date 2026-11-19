use super::{MODE, SCRATCH, codec::OwnerRequest};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

const FRAME: &str = "ALM_RESTART ";

pub(super) fn send(request: &OwnerRequest) {
    emit(&format!("SEND {}", request.encode()));
}

pub(super) fn emit(event: &str) {
    let mut output = io::stdout().lock();
    writeln!(output, "{FRAME}{event}").unwrap();
    output.flush().unwrap();
}

pub(super) struct Client {
    child: Child,
    stdout: BufReader<ChildStdout>,
}

impl Client {
    pub(super) fn start(mode: &str, dir: &Path) -> Self {
        let mut child = Command::new(env::current_exe().expect("locate Rust test binary"))
            .args([
                "--ignored",
                "--exact",
                "confirmation::tests::restart::client_worker",
                "--nocapture",
                "--format",
                "terse",
            ])
            .env_clear()
            .env("PATH", env::var_os("PATH").unwrap_or_default())
            .env(MODE, mode)
            .env(SCRATCH, dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn fresh confirmation client");
        let stdout = BufReader::new(child.stdout.take().expect("client stdout pipe"));
        Self { child, stdout }
    }

    pub(super) fn write_frame(&mut self, frame: &str) {
        let input = self.child.stdin.as_mut().expect("client stdin pipe");
        writeln!(input, "{frame}").unwrap();
    }

    fn event(&mut self) -> Option<String> {
        (&mut self.stdout).lines().find_map(|line| {
            line.expect("read client stdout")
                .strip_prefix(FRAME)
                .map(str::to_owned)
        })
    }

    pub(super) fn sent(&mut self) -> OwnerRequest {
        let event = self.event().expect("worker sent a request");
        OwnerRequest::decode(event.strip_prefix("SEND ").expect("expected a send frame"))
    }

    pub(super) fn finish(mut self) -> Vec<String> {
        drop(self.child.stdin.take());
        let rest = std::iter::from_fn(|| self.event()).collect();
        assert!(self.child.wait().expect("wait for client").success());
        rest
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
