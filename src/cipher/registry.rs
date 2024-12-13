use hashbrown::HashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

use super::cipher::Cipher;

pub static REGISTRY: Lazy<RwLock<Registry>> = Lazy::new(|| RwLock::new(Registry::new()));
pub struct Registry {
    keys: HashMap<String, Box<dyn Cipher>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            keys: Default::default(),
        }
    }
    pub fn new_with_keys(keys: HashMap<String, Box<dyn Cipher>>) -> Self {
        Self { keys }
    }
    pub fn add_key(&mut self, name: String, k: Box<dyn Cipher>) {
        self.keys.insert(name, k);
    }
    pub fn get_key(&self, name: &str) -> Option<Box<dyn Cipher>> {
        self.keys.get(name).map(|k| k.clone_self())
    }
    pub fn remove_key(&mut self, name: &str) {
        self.keys.remove(name);
    }
}
