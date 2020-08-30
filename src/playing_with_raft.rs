use raft::{
    eraftpb::{ConfState, Entry, HardState, Snapshot},
    storage::{RaftState, Storage},
};
use serde::{Deserialize, Serialize};
use {
    byteorder::BigEndian,
    zerocopy::{byteorder::U64, AsBytes, FromBytes, LayoutVerified, Unaligned},
};

static RAFT_STATE_KEY: &[u8; 9] = b"raftstate";

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PersistedRaftState {
    hard_state: PersistedHardState,
    conf_state: PersistedConfState,
}

impl From<PersistedRaftState> for RaftState {
    fn from(state: PersistedRaftState) -> Self {
        RaftState::new(state.hard_state.into(), state.conf_state.into())
    }
}

impl From<RaftState> for PersistedRaftState {
    fn from(state: RaftState) -> Self {
        PersistedRaftState {
            hard_state: state.hard_state.into(),
            conf_state: state.conf_state.into(),
        }
    }
}

impl Default for PersistedRaftState {
    fn default() -> PersistedRaftState {
        PersistedRaftState {
            hard_state: Default::default(),
            conf_state: Default::default(),
        }
    }
}

// We use BigEndian in order to keep the lexicographic order.
#[derive(FromBytes, AsBytes, Unaligned, Clone)]
#[repr(C)]
struct EntryIndex(U64<BigEndian>);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PersistedConfState {
    pub voters: Vec<u64>,
    pub learners: Vec<u64>,
    pub voters_outgoing: Vec<u64>,
    pub learners_next: Vec<u64>,
    pub auto_leave: bool,
}

impl From<PersistedConfState> for ConfState {
    fn from(state: PersistedConfState) -> Self {
        let mut conf_change = ConfState::new();
        conf_change.set_voters(state.voters);
        conf_change.set_learners(state.learners);
        conf_change.set_voters_outgoing(state.voters_outgoing);
        conf_change.set_learners_next(state.learners_next);
        conf_change.set_auto_leave(state.auto_leave);
        conf_change
    }
}

impl From<ConfState> for PersistedConfState {
    fn from(state: ConfState) -> Self {
        PersistedConfState {
            voters: state.get_voters().to_vec(),
            learners: state.get_learners().to_vec(),
            voters_outgoing: state.get_voters_outgoing().to_vec(),
            learners_next: state.get_learners_next().to_vec(),
            auto_leave: state.get_auto_leave(),
        }
    }
}

impl Default for PersistedConfState {
    fn default() -> PersistedConfState {
        ConfState::default().into()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PersistedHardState {
    term: u64,
    vote: u64,
    commit: u64,
}

impl From<PersistedHardState> for HardState {
    fn from(state: PersistedHardState) -> Self {
        let mut hard_state = HardState::new();
        hard_state.set_term(state.term);
        hard_state.set_vote(state.vote);
        hard_state.set_commit(state.commit);
        hard_state
    }
}

impl From<HardState> for PersistedHardState {
    fn from(state: HardState) -> Self {
        PersistedHardState {
            term: state.get_term(),
            vote: state.get_vote(),
            commit: state.get_commit(),
        }
    }
}

impl Default for PersistedHardState {
    fn default() -> PersistedHardState {
        HardState::default().into()
    }
}

#[derive(Debug)]
struct SledStorage {
    is_storage_recovered: bool,
    state_storage: sled::Tree,
    entries_storage: sled::Tree,
}

impl SledStorage {
    fn new() -> SledStorage {
        let db = sled::open("acteur_db")
            .expect("Cannot connect to the Raft storage database (raft/sled).");

        SledStorage {
            is_storage_recovered: db.was_recovered(),
            state_storage: db
                .open_tree("state")
                .expect("Cannot open the storage tree for cluster state (raft/sled)."),
            entries_storage: db
                .open_tree("entries")
                .expect("Cannot open the entries tree for the cluster state (raft/sled)."),
        }
    }
}

impl SledStorage {
    fn create_new_state(&self) -> Result<RaftState, raft::Error> {
        let new_raft_state = RaftState::new(HardState::default(), ConfState::default());

        let state_to_store: PersistedRaftState = new_raft_state.clone().into();

        let encoded_state = bincode::serialize(&state_to_store)
            .expect("Cannot encode default storage error (raft/bincode).");

        self.state_storage
            .insert(RAFT_STATE_KEY, encoded_state)
            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;

        Ok(new_raft_state)
    }

    fn get_previous_state(&self) -> Result<Option<RaftState>, raft::Error> {
        if self.is_storage_recovered {
            return Ok(None);
        }

        let stored_state = self
            .state_storage
            .get(RAFT_STATE_KEY)
            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;

        match stored_state {
            Some(state) => {
                match bincode::deserialize::<Option<PersistedRaftState>>(&state) {
                    Ok(Some(state)) => Ok(Some(state.into())),
                    // In case of error decoding, we delete the state from the DB and say that there is no state
                    Err(_) => {
                        self.state_storage
                            .remove(RAFT_STATE_KEY)
                            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;
                        //TODO: WARNING: We maybe should not block here.
                        self.state_storage
                            .flush()
                            .map_err(|_| raft::Error::Store(raft::StorageError::Unavailable))?;
                        Ok(None)
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }
}

impl Storage for SledStorage {
    fn initial_state(&self) -> Result<RaftState, raft::Error> {
        let state = self.get_previous_state()?;

        let state = match state {
            Some(state) => state,
            None => self.create_new_state()?,
        };

        Ok(state)
    }

    fn entries(
        &self,
        _low: u64,
        _high: u64,
        _max_size: impl Into<Option<u64>>,
    ) -> Result<Vec<Entry>, raft::Error> {
        todo!()
    }

    fn term(&self, _idx: u64) -> Result<u64, raft::Error> {
        todo!()
    }

    fn first_index(&self) -> Result<u64, raft::Error> {
        match self.entries_storage.iter().next() {
            Some(Ok(mut entry)) => {
                let layout: LayoutVerified<&mut [u8], EntryIndex> =
                    LayoutVerified::new_unaligned(&mut *entry.0)
                    //TODO: What do we do when we find corrupted data?
                    .expect("Corrupted data! D:");

                // We used Zerocopy but... I don't know how to remove the copy! Sooo... there you go.
                let entry: EntryIndex = layout.into_ref().clone();

                Ok(entry.0.get())
            }
            _ => Err(raft::Error::Store(raft::StorageError::Unavailable)),
        }
    }

    fn last_index(&self) -> Result<u64, raft::Error> {
        match self.entries_storage.iter().rev().next() {
            Some(Ok(mut entry)) => {
                let layout: LayoutVerified<&mut [u8], EntryIndex> =
                    LayoutVerified::new_unaligned(&mut *entry.0)
                    //TODO: What do we do when we find corrupted data?
                    .expect("Corrupted data! D:");

                // We used Zerocopy but... I don't know how to remove the copy! Sooo... there you go.
                let entry: EntryIndex = layout.into_ref().clone();

                Ok(entry.0.get())
            }
            _ => Err(raft::Error::Store(raft::StorageError::Unavailable)),
        }
    }

    fn snapshot(&self, _request_index: u64) -> Result<Snapshot, raft::Error> {
        todo!()
    }
}
