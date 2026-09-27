//! The state of a file picker shown on another thread, so the window keeps
//! answering the compositor while it is open.

use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
};

/// The file picker, while one is open.
#[derive(Debug, Default)]
pub struct FilePick {
    /// Receives the picked file, or `None` if the picker was cancelled.
    pending: Option<Receiver<Option<PathBuf>>>,
}

/// What [`FilePick::poll`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickPoll {
    /// No picker is open.
    Idle,
    /// The picker is still open.
    Waiting,
    Picked(PathBuf),
    /// The picker was closed without a file, or its thread ended without
    /// an answer.
    Cancelled,
}

impl FilePick {
    /// Whether a picker is open.
    pub fn is_open(&self) -> bool {
        self.pending.is_some()
    }

    /// Starts a pick, and returns where the picker's thread sends the
    /// answer. `None` if a picker is already open.
    pub fn begin(&mut self) -> Option<Sender<Option<PathBuf>>> {
        if self.is_open() {
            return None;
        }
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        Some(sender)
    }

    /// Forgets the open picker, for a thread that couldn't be started.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// Checks for the picker's answer, without waiting. Once there is one,
    /// no picker is open.
    pub fn poll(&mut self) -> PickPoll {
        let Some(receiver) = &self.pending else {
            return PickPoll::Idle;
        };
        let result = match receiver.try_recv() {
            Err(TryRecvError::Empty) => return PickPoll::Waiting,
            Ok(Some(path)) => PickPoll::Picked(path),
            Ok(None) | Err(TryRecvError::Disconnected) => PickPoll::Cancelled,
        };
        self.pending = None;
        result
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
        let _sender = pick.begin().unwrap();
        assert!(pick.is_open());
        assert!(pick.begin().is_none());
    }

    #[test]
    fn waits_until_the_picker_answers() {
        let mut pick = FilePick::default();
        let _sender = pick.begin().unwrap();
        assert_eq!(pick.poll(), PickPoll::Waiting);
        assert!(pick.is_open());
    }

    #[test]
    fn picked_file_then_idle() {
        let mut pick = FilePick::default();
        let sender = pick.begin().unwrap();
        sender.send(Some(PathBuf::from("a.wasm"))).unwrap();
        assert_eq!(pick.poll(), PickPoll::Picked(PathBuf::from("a.wasm")));
        assert!(!pick.is_open());
        assert_eq!(pick.poll(), PickPoll::Idle);
    }

    #[test]
    fn no_file_is_cancelled() {
        let mut pick = FilePick::default();
        let sender = pick.begin().unwrap();
        sender.send(None).unwrap();
        assert_eq!(pick.poll(), PickPoll::Cancelled);
        assert!(!pick.is_open());
    }

    #[test]
    fn thread_ending_without_an_answer_is_cancelled() {
        let mut pick = FilePick::default();
        drop(pick.begin().unwrap());
        assert_eq!(pick.poll(), PickPoll::Cancelled);
        assert!(!pick.is_open());
        assert!(pick.begin().is_some());
    }
}
