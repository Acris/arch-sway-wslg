use sha2::{Digest, Sha256};

pub type TextHash = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingWrite {
    pub request_id: u64,
    pub hash: TextHash,
}

/// Echo suppression for both directions.
///
/// Callers hash a text once with [`MirrorState::hash`] and pass the hash around;
/// a 16 MiB selection would otherwise be digested several times per hop.
#[derive(Debug, Default)]
pub struct MirrorState {
    next_request_id: u64,
    wayland_hash: Option<TextHash>,
    windows_hash: Option<TextHash>,
    windows_sequence: Option<u32>,
    pending_windows: Option<PendingWrite>,
    // The hash is the identity of a publish: two publishes of the same text are
    // interchangeable, so the first roundtrip may commit either.
    pending_wayland: Option<TextHash>,
}

impl MirrorState {
    #[must_use]
    pub fn hash(text: &[u8]) -> TextHash {
        Sha256::digest(text).into()
    }

    #[must_use]
    pub fn is_wayland_echo(&self, hash: &TextHash) -> bool {
        self.wayland_hash.as_ref() == Some(hash) || self.pending_wayland.as_ref() == Some(hash)
    }

    #[must_use]
    pub fn is_windows_echo(&self, hash: &TextHash) -> bool {
        self.windows_hash.as_ref() == Some(hash)
            || self
                .pending_windows
                .as_ref()
                .is_some_and(|pending| pending.hash == *hash)
    }

    pub fn observe_wayland(&mut self, hash: TextHash) {
        self.pending_wayland = None;
        self.wayland_hash = Some(hash);
    }

    pub fn invalidate_wayland(&mut self) {
        self.wayland_hash = None;
    }

    pub fn reject_wayland_selection(&mut self) {
        self.pending_wayland = None;
        self.wayland_hash = None;
    }

    pub fn observe_windows(&mut self, hash: TextHash, sequence: u32) {
        self.windows_hash = Some(hash);
        self.windows_sequence = Some(sequence);
    }

    pub fn invalidate_windows(&mut self, sequence: u32) {
        self.windows_hash = None;
        self.windows_sequence = Some(sequence);
    }

    pub fn reset_windows_transport(&mut self) {
        self.pending_windows = None;
    }

    #[must_use]
    pub fn has_pending_windows_write(&self) -> bool {
        self.pending_windows.is_some()
    }

    pub fn begin_windows_write(&mut self, hash: TextHash) -> PendingWrite {
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        let pending = PendingWrite {
            request_id: self.next_request_id,
            hash,
        };
        self.pending_windows = Some(pending.clone());
        pending
    }

    pub fn commit_windows_write(&mut self, request_id: u64, sequence: u32) -> bool {
        let Some(pending) = self.pending_windows.take() else {
            return false;
        };
        if pending.request_id != request_id {
            self.pending_windows = Some(pending);
            return false;
        }
        self.windows_hash = Some(pending.hash);
        self.windows_sequence = Some(sequence);
        true
    }

    pub fn fail_windows_write(&mut self, request_id: u64) -> bool {
        if self.pending_windows.as_ref().map(|write| write.request_id) != Some(request_id) {
            return false;
        }
        self.pending_windows = None;
        true
    }

    pub fn begin_wayland_publish(&mut self, hash: TextHash, sequence: u32) {
        self.observe_windows(hash, sequence);
        self.pending_wayland = Some(hash);
    }

    pub fn commit_wayland_publish(&mut self, hash: &TextHash) -> bool {
        if self.pending_wayland.as_ref() != Some(hash) {
            return false;
        }
        self.pending_wayland = None;
        self.wayland_hash = Some(*hash);
        true
    }

    #[must_use]
    pub fn windows_changed_since(&self, sequence: u32) -> bool {
        self.windows_sequence != Some(sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(text: &[u8]) -> TextHash {
        MirrorState::hash(text)
    }

    #[test]
    fn failed_write_never_becomes_committed() {
        let mut state = MirrorState::default();
        let pending = state.begin_windows_write(h(b"new"));
        assert!(state.fail_windows_write(pending.request_id));
        assert!(!state.is_windows_echo(&h(b"new")));
    }

    #[test]
    fn only_matching_ack_commits() {
        let mut state = MirrorState::default();
        let pending = state.begin_windows_write(h(b"new"));
        assert!(!state.commit_windows_write(pending.request_id + 1, 10));
        assert!(state.commit_windows_write(pending.request_id, 10));
        assert!(state.is_windows_echo(&h(b"new")));
        assert!(!state.windows_changed_since(10));
    }

    #[test]
    fn wayland_publish_commits_only_after_matching_roundtrip() {
        let mut state = MirrorState::default();
        state.begin_wayland_publish(h(b"from Windows"), 11);
        assert!(state.is_wayland_echo(&h(b"from Windows")));
        assert!(!state.commit_wayland_publish(&h(b"something else")));
        assert!(state.commit_wayland_publish(&h(b"from Windows")));
        assert!(!state.commit_wayland_publish(&h(b"from Windows")));
        assert!(!state.windows_changed_since(11));
    }

    #[test]
    fn non_text_windows_selection_invalidates_echo_state() {
        let mut state = MirrorState::default();
        let pending = state.begin_windows_write(h(b"same text"));
        assert!(state.commit_windows_write(pending.request_id, 7));
        assert!(state.is_windows_echo(&h(b"same text")));

        state.invalidate_windows(8);

        assert!(!state.is_windows_echo(&h(b"same text")));
        assert!(!state.windows_changed_since(8));
    }

    #[test]
    fn directions_have_independent_current_state() {
        let mut state = MirrorState::default();
        state.observe_wayland(h(b"Linux"));
        state.observe_windows(h(b"Windows"), 4);

        assert!(state.is_wayland_echo(&h(b"Linux")));
        assert!(!state.is_wayland_echo(&h(b"Windows")));
        assert!(state.is_windows_echo(&h(b"Windows")));
        assert!(!state.is_windows_echo(&h(b"Linux")));
    }

    #[test]
    fn wayland_invalidation_allows_same_text_again() {
        let mut state = MirrorState::default();
        state.observe_wayland(h(b"same text"));
        assert!(state.is_wayland_echo(&h(b"same text")));

        state.invalidate_wayland();

        assert!(!state.is_wayland_echo(&h(b"same text")));
    }

    #[test]
    fn rejected_wayland_selection_does_not_suppress_windows_text() {
        let mut state = MirrorState::default();
        state.observe_wayland(h(b"same text"));
        state.observe_windows(h(b"same text"), 7);

        state.reject_wayland_selection();

        assert!(!state.is_wayland_echo(&h(b"same text")));
    }

    #[test]
    fn external_wayland_selection_cancels_stale_publish_ack() {
        let mut state = MirrorState::default();
        state.begin_wayland_publish(h(b"from Windows"), 12);

        state.observe_wayland(h(b"new in Sway"));

        assert!(!state.commit_wayland_publish(&h(b"from Windows")));
        assert!(state.is_wayland_echo(&h(b"new in Sway")));
        assert!(!state.is_wayland_echo(&h(b"from Windows")));
    }
}
