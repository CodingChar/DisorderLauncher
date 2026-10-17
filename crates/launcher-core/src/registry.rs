use identity_api::{IdentityProvider, IdentityProviderKind};
use std::collections::HashMap;
use std::sync::Arc;

pub struct IdentityRegistry {
    providers: HashMap<IdentityProviderKind, Arc<dyn IdentityProvider>>,
}

impl IdentityRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn IdentityProvider>) {
        self.providers.insert(provider.kind(), provider);
    }

    pub fn get(&self, kind: IdentityProviderKind) -> Option<Arc<dyn IdentityProvider>> {
        self.providers.get(&kind).cloned()
    }
}

impl Default for IdentityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
