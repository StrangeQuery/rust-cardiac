//! Small helpers shared by the passes.

use std::borrow::Borrow;

/// A map that keeps its keys in the order they were first set. Setting a key again keeps its place; removing it
/// and setting it again puts it last. Comparing two maps compares the order too. A key is looked up by anything it
/// borrows as, as in std's maps: a `String` key by a `&str`.
#[derive(Clone, Debug, PartialEq)]
pub struct Ordered<K, V> {
    items: Vec<(K, V)>,
}

impl<K, V> Default for Ordered<K, V> {
    fn default() -> Self {
        Ordered { items: Vec::new() }
    }
}

impl<K: PartialEq, V> Ordered<K, V> {
    pub fn new() -> Self {
        Ordered { items: Vec::new() }
    }
    pub fn get<Q: PartialEq + ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.items.iter().find(|(x, _)| x.borrow() == k).map(|(_, v)| v)
    }
    pub fn get_mut<Q: PartialEq + ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        self.items.iter_mut().find(|(x, _)| x.borrow() == k).map(|(_, v)| v)
    }
    pub fn has<Q: PartialEq + ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.items.iter().any(|(x, _)| x.borrow() == k)
    }
    pub fn set(&mut self, k: K, v: V) {
        match self.get_mut(&k) {
            Some(x) => *x = v,
            None => self.items.push((k, v)),
        }
    }
    pub fn remove<Q: PartialEq + ?Sized>(&mut self, k: &Q)
    where
        K: Borrow<Q>,
    {
        self.items.retain(|(x, _)| x.borrow() != k);
    }
    /// Keep only the entries `keep` says to, in their order.
    pub fn retain(&mut self, mut keep: impl FnMut(&K, &V) -> bool) {
        self.items.retain(|(k, v)| keep(k, v));
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.items.iter().map(|(k, _)| k)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.items.iter().map(|(k, v)| (k, v))
    }
}
