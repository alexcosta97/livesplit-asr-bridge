//! The state of a file picker shown on another thread, so the window keeps
//! answering the compositor while it is open.

use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
};

/// What a file is picked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickFor {
    /// A `.wasm` file to load (Open…).
    Open,
    /// Where to save the log (Save log…).
    SaveLog,
    /// The file for the file selection setting with this key (Browse).
    Setting(String),
}

/// The file picker, while one is open. There is one at a time, whatever
/// it is for.
#[derive(Debug, Default)]
pub struct FilePick {
    /// What the open picker is for, and where the picked file arrives, or
    /// `None` if the picker was cancelled.
    pending: Option<(PickFor, Receiver<Option<PathBuf>>)>,
}

/// What [`FilePick::poll`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickPoll {
    /// No picker is open.
    Idle,
    /// The picker is still open.
    Waiting,
    /// A file was picked, for what the picker was opened for.
    Picked(PickFor, PathBuf),
    /// The picker was closed without a file, or its thread ended without
    /// an answer.
    Cancelled,
}

impl FilePick {
    /// Whether a picker is open.
    pub fn is_open(&self) -> bool {
        self.pending.is_some()
    }

    /// Starts a pick for `for_`, and returns where the picker's thread
    /// sends the answer. `None` if a picker is already open.
    pub fn begin(&mut self, for_: PickFor) -> Option<Sender<Option<PathBuf>>> {
        if self.is_open() {
            return None;
        }
        let (sender, receiver) = mpsc::channel();
        self.pending = Some((for_, receiver));
        Some(sender)
    }

    /// Forgets the open picker, for a thread that couldn't be started.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// Checks for the picker's answer, without waiting. Once there is one,
    /// no picker is open.
    pub fn poll(&mut self) -> PickPoll {
        let Some((_, receiver)) = &self.pending else {
            return PickPoll::Idle;
        };
        let answer = match receiver.try_recv() {
            Err(TryRecvError::Empty) => return PickPoll::Waiting,
            Ok(answer) => answer,
            Err(TryRecvError::Disconnected) => None,
        };
        let Some((for_, _)) = self.pending.take() else {
            return PickPoll::Idle;
        };
        match answer {
            Some(path) => PickPoll::Picked(for_, path),
            None => PickPoll::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_at_first() {
        let mut pick = FilePick::default();
        assert!(!pick.is_open());
        assert_eq!(pick.poll(), PickPoll::Idle);
    }

    #[test]
    fn only_one_picker_at_a_time() {
        let mut pick = FilePick::default();
        let _sender = pick.begin(PickFor::Open).unwrap();
        assert!(pick.is_open());
        assert!(pick.begin(PickFor::Open).is_none());
    }

    #[test]
    fn a_second_pick_for_something_else_is_ignored() {
        let mut pick = FilePick::default();
        let sender = pick.begin(PickFor::SaveLog).unwrap();
        assert!(pick.begin(PickFor::Setting("route".to_owned())).is_none());
        assert!(pick.begin(PickFor::Open).is_none());
        // The open picker keeps what it was for.
        sender.send(Some(PathBuf::from("a.log"))).unwrap();
        assert_eq!(
            pick.poll(),
            PickPoll::Picked(PickFor::SaveLog, PathBuf::from("a.log"))
        );
    }

    #[test]
    fn waits_until_the_picker_answers() {
        let mut pick = FilePick::default();
        let _sender = pick.begin(PickFor::Open).unwrap();
        assert_eq!(pick.poll(), PickPoll::Waiting);
        assert!(pick.is_open());
    }

    #[test]
    fn picked_file_then_idle() {
        let mut pick = FilePick::default();
        let sender = pick.begin(PickFor::Open).unwrap();
        sender.send(Some(PathBuf::from("a.wasm"))).unwrap();
        assert_eq!(
            pick.poll(),
            PickPoll::Picked(PickFor::Open, PathBuf::from("a.wasm"))
        );
        assert!(!pick.is_open());
        assert_eq!(pick.poll(), PickPoll::Idle);
    }

    #[test]
    fn no_file_is_cancelled() {
        let mut pick = FilePick::default();
        let sender = pick.begin(PickFor::SaveLog).unwrap();
        sender.send(None).unwrap();
        assert_eq!(pick.poll(), PickPoll::Cancelled);
        assert!(!pick.is_open());
    }

    #[test]
    fn thread_ending_without_an_answer_is_cancelled() {
        let mut pick = FilePick::default();
        drop(pick.begin(PickFor::Setting("route".to_owned())).unwrap());
        assert_eq!(pick.poll(), PickPoll::Cancelled);
        assert!(!pick.is_open());
        assert!(pick.begin(PickFor::Open).is_some());
    }
}
