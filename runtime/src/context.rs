use std::collections::{BTreeMap, VecDeque};

use crate::*;

/// A `Context` is used to store and manage plugins
#[derive(Default)]
pub struct Context {
    /// Plugin registry
    pub plugins: BTreeMap<PluginIndex, Plugin>,

    /// Error message
    pub error: Option<std::ffi::CString>,
    next_id: std::sync::atomic::AtomicI32,
    reclaimed_ids: VecDeque<PluginIndex>,
}

const START_REUSING_IDS: usize = 25;

impl Context {
    /// Create a new context
    pub fn new() -> Context {
        Context {
            plugins: BTreeMap::new(),
            error: None,
            next_id: std::sync::atomic::AtomicI32::new(0),
            reclaimed_ids: VecDeque::new(),
        }
    }

    /// Get the next valid plugin ID
    pub fn next_id(&mut self) -> Result<PluginIndex, Error> {
        // Make sure we haven't exhausted all plugin IDs, it reach this it would require the machine
        // running this code to have a lot of memory - no computer I tested on was able to allocate
        // the max number of plugins.
        //
        // Since `Context::remove` collects IDs that have been removed we will
        // try to use one of those before returning an error
        let exhausted = self.next_id.load(std::sync::atomic::Ordering::SeqCst) == PluginIndex::MAX;

        // If there is a significant number of old IDs we can start to re-use them
        if self.reclaimed_ids.len() >= START_REUSING_IDS || exhausted {
            if let Some(x) = self.reclaimed_ids.pop_front() {
                return Ok(x);
            }

            if exhausted {
                return Err(anyhow::format_err!(
                    "All plugin descriptors are in use, unable to allocate a new plugin"
                ));
            }
        }

        Ok(self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    /// Set the context error
    pub fn set_error(&mut self, e: impl std::fmt::Debug) {
        self.error = Some(error_string(e));
    }

    /// Convenience function to set error and return the value passed as the final parameter
    pub fn error<T>(&mut self, e: impl std::fmt::Debug, x: T) -> T {
        self.set_error(e);
        x
    }

    /// Get a plugin from the context
    pub fn plugin(&mut self, id: PluginIndex) -> Option<&mut Plugin> {
        self.plugins.get_mut(&id)
    }

    /// Remove a plugin from the context
    pub fn remove(&mut self, id: PluginIndex) {
        self.plugins.remove(&id);

        // Collect old IDs in case we need to re-use them
        self.reclaimed_ids.push_back(id);
    }
}