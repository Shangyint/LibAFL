//! The ``NewHashFeedback`` uses the backtrace hash and a hashset to only keep novel cases

use std::{fmt::Debug, hash::Hash, marker::PhantomData};

use hashbrown::HashSet;
use num_traits::PrimInt;
use serde::{Deserialize, Serialize};

use crate::{
    bolts::tuples::Named,
    events::EventFirer,
    executors::ExitKind,
    feedbacks::{Feedback, FeedbackState},
    inputs::Input,
    observers::{ObserverWithHashField, ObserversTuple},
    state::{HasClientPerfMonitor, HasFeedbackState},
    Error,
};

/// A state that implements this trait has a hash set
pub trait HashSetState<T> {
    /// creates a new instance with a specific hashset
    fn with_hash_set(name: &'static str, hash_set: HashSet<T>) -> Self;
    /// updates the `hash_set` with the given value
    fn update_hash_set(&mut self, value: T) -> Result<bool, Error>;
}

/// The state of [`NewHashFeedback`]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "T: serde::de::DeserializeOwned")]
pub struct NewHashFeedbackState<T>
where
    T: PrimInt + Default + Copy + 'static + Serialize + serde::de::DeserializeOwned + Hash + Debug,
{
    /// Contains information about untouched entries
    pub hash_set: HashSet<T>,
    /// Name identifier of this instance
    pub name: String,
}

impl<T> FeedbackState for NewHashFeedbackState<T>
where
    T: PrimInt + Default + Copy + 'static + Serialize + serde::de::DeserializeOwned + Hash + Debug,
{
    fn reset(&mut self) -> Result<(), Error> {
        self.hash_set.clear();
        Ok(())
    }
}

impl<T> Named for NewHashFeedbackState<T>
where
    T: PrimInt + Default + Copy + 'static + Serialize + serde::de::DeserializeOwned + Hash + Debug,
{
    #[inline]
    fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl<T> NewHashFeedbackState<T>
where
    T: PrimInt + Default + Copy + 'static + Serialize + serde::de::DeserializeOwned + Hash + Debug,
{
    /// Create a new [`NewHashFeedbackState`]
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            hash_set: HashSet::<T>::new(),
            name: name.to_string(),
        }
    }

    /// Create a new [`NewHashFeedbackState`] for an observer that implements [`ObserverWithHashField`]
    pub fn with_observer(backtrace_observer: &(impl ObserverWithHashField + Named)) -> Self {
        Self {
            hash_set: HashSet::<T>::new(),
            name: backtrace_observer.name().to_string(),
        }
    }
}
impl<T> HashSetState<T> for NewHashFeedbackState<T>
where
    T: PrimInt + Default + Copy + 'static + Serialize + serde::de::DeserializeOwned + Hash + Debug,
{
    /// Create new [`NewHashFeedbackState`] using a name and a hash set.
    #[must_use]
    fn with_hash_set(name: &'static str, hash_set: HashSet<T>) -> Self {
        Self {
            hash_set,
            name: name.to_string(),
        }
    }

    fn update_hash_set(&mut self, value: T) -> Result<bool, Error> {
        let r = self.hash_set.insert(value);
        println!("Got r={}, the hashset is {:?}", r, &self.hash_set);
        Ok(r)
    }
}

/// A [`NewHashFeedback`] maintains a hashset of already seen stacktraces and considers interesting unseen ones
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewHashFeedback<O, T> {
    feedback_name: String,
    observer_name: String,
    phantom: PhantomData<(O, T)>,
}

impl<I, O, S, T> Feedback<I, S> for NewHashFeedback<O, T>
where
    I: Input,
    S: HasClientPerfMonitor + HasFeedbackState,
    O: ObserverWithHashField + Named + Debug,
    T: PrimInt + Default + Copy + 'static + Serialize + serde::de::DeserializeOwned + Hash + Debug,
{
    type FeedbackState = NewHashFeedbackState<T>;

    fn is_interesting<EM, OT>(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &I,
        observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, Error>
    where
        EM: EventFirer<I>,
        OT: ObserversTuple<I, S>,
    {
        let observer = observers
            .match_name::<O>(&self.observer_name)
            .expect("A NewHashFeedback needs a BacktraceObserver");

        let backtrace_state = _state
            .feedback_state_mut()
            .match_name_mut::<NewHashFeedbackState<u64>>(&self.observer_name)
            .unwrap();

        match observer.hash() {
            Some(hash) => {
                let res = backtrace_state
                    .update_hash_set(*hash)
                    .expect("Failed to update the hash state");
                Ok(res)
            }
            None => {
                // We get here if the hash was not updated, i.e the first run or if no crash happens
                Ok(false)
            }
        }
    }

    fn init_state(&mut self) -> Result<Self::FeedbackState, Error> {
        Ok(Self::FeedbackState::new(&self.observer_name))
    }
}

impl<O, T> Named for NewHashFeedback<O, T> {
    #[inline]
    fn name(&self) -> &str {
        &self.feedback_name
    }
}

impl<O, T> NewHashFeedback<O, T>
where
    O: ObserverWithHashField + Named + Debug,
    T: PrimInt + Default + Copy + 'static + Serialize + serde::de::DeserializeOwned + Hash + Debug,
{
    /// Returns a new [`NewHashFeedback`]. Carefull, it's recommended to use `new_with_observer`
    /// Setting an observer name that doesn't exist would eventually trigger a panic.
    #[must_use]
    pub fn new(feedback_name: &str, observer_name: &str) -> Self {
        Self {
            feedback_name: feedback_name.to_string(),
            observer_name: observer_name.to_string(),
            phantom: PhantomData,
        }
    }

    /// Returns a new [`NewHashFeedback`].
    #[must_use]
    pub fn new_with_observer(feedback_name: &str, observer: &O) -> Self {
        Self {
            feedback_name: feedback_name.to_string(),
            observer_name: observer.name().to_string(),
            phantom: PhantomData,
        }
    }
}
