use crate::store_has_item;
use crate::store_push;
use crate::store_remove;
use crate::store_update;
use anyhow::Result;
use bp7::{Bundle, CanonicalData, EndpointID, BUNDLE_AGE_BLOCK};
use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Constraint is a retention constraint as defined in the subsections of the
/// fifth chapter of draft-ietf-dtn-bpbis-12.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constraint {
    /// DispatchPending is assigned to a bundle if its dispatching is pending.
    DispatchPending,
    /// ForwardPending is assigned to a bundle if its forwarding is pending.
    ForwardPending,
    /// ReassemblyPending is assigned to a fragmented bundle if its reassembly is
    /// pending.
    ReassemblyPending,
    /// Contraindicated is assigned to a bundle if it could not be delivered and
    /// was moved to the contraindicated stage. This Constraint was not defined
    /// in draft-ietf-dtn-bpbis-12, but seemed reasonable for this implementation.
    Contraindicated,

    /// LocalEndpoint is assigned to a bundle after delivery to a local endpoint.
    /// This constraint demands storage until the endpoint removes this constraint.
    LocalEndpoint,

    /// This bundle has been deleted, only the meta data is kept to prevent
    /// resubmission in the future.
    Deleted,
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// BundlePack is a set of a bundle, it's creation or reception time stamp and
/// a set of constraints used in the process of delivering this bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct BundlePack {
    pub bundle: Bundle,
    pub receiver: EndpointID,
    pub timestamp: u64,
    pub id: String,
    pub size: usize,
    constraints: HashSet<Constraint>,
}

impl fmt::Display for BundlePack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {:?}", self.bundle.id(), self.constraints)
    }
}

/// Create from a given bundle.
impl From<Bundle> for BundlePack {
    fn from(mut bundle: Bundle) -> Self {
        let bid = bundle.id();
        let size = bundle.to_cbor().len();
        BundlePack {
            receiver: bundle.primary.destination.clone(),
            bundle,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
            id: bid,
            size,
            constraints: HashSet::new(),
        }
    }
}

impl BundlePack {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn sync(&self) -> Result<()> {
        if !store_has_item(self.id()) {
            store_push(self)?;
        } else if !self.has_constraints() {
            store_remove(self.id());
        } else {
            // TODO: add update logic
            store_update(self)?;
        }
        Ok(())
    }
    pub fn has_receiver(&self) -> bool {
        self.receiver != EndpointID::none()
    }
    pub fn has_constraint(&self, constraint: Constraint) -> bool {
        self.constraints.contains(&constraint)
    }
    pub fn has_constraints(&self) -> bool {
        !self.constraints.is_empty()
    }
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.insert(constraint);
    }
    pub fn remove_constraint(&mut self, constraint: Constraint) {
        self.constraints.remove(&constraint);
    }
    pub fn clear_constraints(&mut self) {
        let local_set = self.has_constraint(Constraint::LocalEndpoint);

        self.constraints.clear();

        if local_set {
            self.add_constraint(Constraint::LocalEndpoint);
        }
    }
    /// UpdateBundleAge updates the bundle's Bundle Age block based on its reception
    /// timestamp, if such a block exists.
    pub fn update_bundle_age(&mut self) -> Option<u64> {
        if let Some(block) = self.bundle.extension_block_by_type_mut(BUNDLE_AGE_BLOCK) {
            let mut new_age = 0 as u64; // TODO: lost fight with borrowchecker

            if let CanonicalData::BundleAge(age) = block.data() {
                let offset = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as u64
                    - self.timestamp;
                new_age = age + offset;
            }
            if new_age != 0 {
                block.set_data(CanonicalData::BundleAge(new_age));
                return Some(new_age);
            }
        }
        None
    }
}
