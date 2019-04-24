#![allow(dead_code)]
use std::io::{Error, ErrorKind, Result};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::process::{Child, ChildStdout, Command, Stdio};

/// Data structure used to hold processes
/// and allows for the chaining of commands
pub struct Pipe {
    child: Result<Child>,
}

impl Pipe {
    /// Creates a new `Pipe` by taking in a command
    /// as input. An empty string as input will
    /// cause the eventual end of the piping to have
    /// an error returned. Make sure you place in an
    /// actual command.
    pub fn new(command: &str) -> Pipe {
        let mut split = command.split_whitespace();
        let command = match split.next() {
            Some(x) => x,
            None => return pipe_new_error("No command as input"),
        };
        let args = split.collect::<Vec<&str>>();

        Pipe {
            child: Command::new(command)
                .args(args.as_slice())
                .stdout(Stdio::piped())
                .spawn(),
        }
    }

    /// This is used to chain commands together. Use this for each
    /// command that you want to pipe.
    pub fn then(self, command: &str) -> Pipe {
        let stdout = match self.child {
            Ok(child) => match child.stdout {
                Some(stdout) => stdout,
                None => return pipe_new_error("No stdout for a command"),
            },
            Err(e) => return pipe_error(Err(e)),
        };

        let mut split = command.split_whitespace();
        let command = match split.next() {
            Some(x) => x,
            None => return pipe_new_error("No command as input"),
        };
        let args = split.collect::<Vec<&str>>();
        let stdio = unsafe { Stdio::from_raw_fd(stdout.as_raw_fd()) };

        Pipe {
            child: Command::new(command)
                .args(args.as_slice())
                .stdout(Stdio::piped())
                .stdin(stdio)
                .spawn(),
        }
    }

    /// This can be used take a peek at the stdout for the current pipe.
    pub fn peek(&self) -> Result<&ChildStdout> {
        if let Ok(child) = &self.child {
            if let Some(ref stdout) = child.stdout {
                return Ok(stdout);
            }
        }
        Err(Error::new(ErrorKind::Other, "No stdout for a command"))
    }

    /// Return the `Child` process of the final command that
    /// had data piped into it.
    pub fn finally(self) -> Result<Child> {
        self.child
    }
}

/// Helper method to generate a new error from a string
/// but have it be a `Pipe` so that it can be passed through
/// the chain.
fn pipe_new_error(error: &str) -> Pipe {
    Pipe {
        child: Err(Error::new(ErrorKind::Other, error)),
    }
}

/// Helper method used to pass the error down the chain by creating
/// a new pipe with the error passed in.
fn pipe_error(error: Result<Child>) -> Pipe {
    Pipe { child: error }
}

#[test]
fn test_pipe() {
    let out = Pipe::new("ls /")
        .then("grep usr")
        .then("head -c 1")
        .finally()
        .expect("Commands did not pipe")
        .wait_with_output()
        .expect("failed to wait on child");

    assert_eq!("u", &String::from_utf8(out.stdout).unwrap());
}
