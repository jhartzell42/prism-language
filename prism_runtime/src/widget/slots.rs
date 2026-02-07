use std::{collections::BTreeMap, ops::Index};

// XXX: This is just an IndexMap, isn't it?

/// These are externally visible properties of the node for backends
/// to interact with them for nodes that require interaction with the backend.
///
/// They have both names and indexes. The expectation is that the backend
/// interacting with them will know what names or indexes to use to query
/// various properties as part of that backend components API.
#[derive(Clone, Debug)]
pub struct Slots<T> {
    pub(crate) values: Vec<T>,
    pub(crate) names: BTreeMap<String, usize>,
}

impl<T> Default for Slots<T> {
    fn default() -> Self {
        Self {
            values: Default::default(),
            names: Default::default(),
        }
    }
}

impl<T> Index<usize> for Slots<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_index(index).unwrap()
    }
}

impl<T> Index<&str> for Slots<T> {
    type Output = T;

    fn index(&self, name: &str) -> &Self::Output {
        self.get_name(name).unwrap()
    }
}

impl<T> Slots<T> {
    /// How many items?
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Add a new item.
    pub fn add(&mut self, name: String, value: T) -> usize {
        if self.names.contains_key(&name) {
            panic!("Already have slot with name {name}");
        }
        let ix = self.values.len();
        self.values.push(value);
        self.names.insert(name, ix);
        ix
    }

    /// Get an item by index.
    pub fn get_index(&self, ix: usize) -> Option<&T> {
        self.values.get(ix)
    }

    /// Get an item by name.
    pub fn get_name(&self, name: &str) -> Option<&T> {
        let ix = self.index_for_name(name)?;
        self.get_index(ix)
    }

    /// Get an index for a given name.
    pub fn index_for_name(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    /// Update an item with a name.
    pub fn update_name(&mut self, name: &str, val: T) -> Result<(), SlotError> {
        self.update_index(
            self.index_for_name(name)
                .ok_or(SlotError::NameNotFound(name.to_string()))?,
            val,
        );
        Ok(())
    }

    /// Update an item with an index
    pub fn update_index(&mut self, ix: usize, val: T) -> Result<(), SlotError> {
        let v = self
            .values
            .get_mut(ix)
            .ok_or(SlotError::IndexNotFound(ix))?;
        *v = val;
        Ok(())
    }
}

#[derive(Debug)]
pub enum SlotError {
    NameNotFound(String),
    IndexNotFound(usize),
}
