use super::{MODE, SCRATCH, codec::OwnerRequest};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

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
    events: Receiver<io::Result<String>>,
    timeout: Duration,
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
        let (sender, events) = mpsc::channel();
        thread::spawn(move || {
            for line in stdout.lines() {
                match line {
                    Ok(line) => {
                        if let Some(event) = line.strip_prefix(FRAME)
                            && sender.send(Ok(event.to_owned())).is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        return;
                    }
                }
            }
        });
        Self {
            child,
            events,
            timeout: Duration::from_secs(10),
        }
    }

    pub(super) fn write_frame(&mut self, frame: &str) {
        let input = self.child.stdin.as_mut().expect("client stdin pipe");
        writeln!(input, "{frame}").unwrap();
    }

    fn event(&mut self) -> Option<String> {
        match self.events.recv_timeout(self.timeout) {
            Ok(event) => Some(event.expect("read client stdout")),
            Err(RecvTimeoutError::Disconnected) => None,
            Err(RecvTimeoutError::Timeout) => panic!("timed out waiting for client event"),
        }
    }

    pub(super) fn sent(&mut self) -> OwnerRequest {
        let event = self.event().expect("worker sent a request");
        OwnerRequest::decode(event.strip_prefix("SEND ").expect("expected a send frame"))
    }

    pub(super) fn finish(mut self) -> Vec<String> {
        drop(self.child.stdin.take());
        let rest = std::iter::from_fn(|| self.event()).collect();
        let deadline = Instant::now() + self.timeout;
        loop {
            if let Some(status) = self.child.try_wait().expect("wait for client") {
                assert!(status.success());
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for client exit"
            );
            thread::sleep(Duration::from_millis(10));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::confirmation::tests::restart::Scratch;
    use std::time::Duration;

    #[test]
    fn missing_send_times_out_when_worker_waits_for_input() {
        let scratch = Scratch::new();
        let mut client = Client::start("stall", &scratch.0);
        client.timeout = Duration::from_millis(50);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.sent()));
        let error = result.expect_err("a stalled worker must time out");
        assert_eq!(
            error.downcast_ref::<&str>(),
            Some(&"timed out waiting for client event")
        );
    }
}
