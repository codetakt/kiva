//! RFC 5280 policy processing with stable, explicit parent links.
//!
//! Equal policy OIDs at equal depths can belong to different branches. Neither
//! child existence nor ancestry can be reconstructed from expected policy sets.
use super::{OID_ANY_POLICY, PolicyTreeNode, Vec};
use der::asn1::ObjectIdentifier as Oid;
use x509_cert::ext::pkix::{
    PolicyMappings,
    certpolicy::{CertificatePolicies, PolicyQualifierInfo},
};

struct Node {
    parent: Option<usize>,
    live: bool,
    value: PolicyTreeNode,
}

pub(super) struct Tree {
    // Entries are never moved or reused. A parent always precedes its children.
    nodes: Vec<Node>,
}

impl Tree {
    pub(super) fn new() -> Self {
        Self {
            nodes: vec![Node {
                parent: None,
                live: true,
                value: PolicyTreeNode {
                    depth: 0,
                    valid_policy: OID_ANY_POLICY,
                    expected_policy_set: vec![OID_ANY_POLICY],
                    qualifiers: Vec::new(),
                },
            }],
        }
    }

    fn at_depth(&self, depth: usize) -> Vec<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.live && node.value.depth == depth)
            .map(|(id, _)| id)
            .collect()
    }

    fn add(
        &mut self,
        parent: usize,
        policy: Oid,
        expected: Vec<Oid>,
        qualifiers: Vec<PolicyQualifierInfo>,
    ) {
        self.nodes.push(Node {
            parent: Some(parent),
            live: true,
            value: PolicyTreeNode {
                depth: self.nodes[parent].value.depth + 1,
                valid_policy: policy,
                expected_policy_set: expected,
                qualifiers,
            },
        });
    }

    // Prune actual childless ancestors, including the root. Retain depth-n
    // leaves; their existence keeps exactly their own ancestor paths alive.
    fn prune(&mut self, depth: usize) {
        for id in (0..self.nodes.len()).rev() {
            if self.nodes[id].live
                && self.nodes[id].value.depth < depth
                && !self
                    .nodes
                    .iter()
                    .any(|node| node.live && node.parent == Some(id))
            {
                self.nodes[id].live = false;
            }
        }
    }

    fn remove_subtree(&mut self, id: usize) {
        self.nodes[id].live = false;
        // Parent-before-child order propagates deletion through every descendant.
        for child in id + 1..self.nodes.len() {
            if self.nodes[child]
                .parent
                .is_some_and(|parent| !self.nodes[parent].live)
            {
                self.nodes[child].live = false;
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        !self.nodes[0].live
    }

    /// §6.1.3(d): specific policy children, per-parent anyPolicy expansion, prune.
    pub(super) fn process(
        &mut self,
        depth: usize,
        policies: &CertificatePolicies,
        expand_any: bool,
    ) {
        let parents = self.at_depth(depth - 1);
        for entry in &policies.0 {
            let policy = entry.policy_identifier;
            if policy == OID_ANY_POLICY {
                continue;
            }
            let mut matches: Vec<_> = parents
                .iter()
                .copied()
                .filter(|&id| self.nodes[id].value.expected_policy_set.contains(&policy))
                .collect();
            if matches.is_empty() {
                matches.extend(
                    parents
                        .iter()
                        .copied()
                        .filter(|&id| self.nodes[id].value.valid_policy == OID_ANY_POLICY),
                );
            }
            for parent in matches {
                self.add(
                    parent,
                    policy,
                    vec![policy],
                    entry.policy_qualifiers.clone().unwrap_or_default(),
                );
            }
        }
        if expand_any {
            if let Some(any) = policies
                .0
                .iter()
                .find(|entry| entry.policy_identifier == OID_ANY_POLICY)
            {
                for parent in parents {
                    for policy in self.nodes[parent].value.expected_policy_set.clone() {
                        if !self.nodes.iter().any(|node| {
                            node.live
                                && node.parent == Some(parent)
                                && node.value.valid_policy == policy
                        }) {
                            self.add(
                                parent,
                                policy,
                                vec![policy],
                                any.policy_qualifiers.clone().unwrap_or_default(),
                            );
                        }
                    }
                }
            }
        }
        self.prune(depth);
    }

    /// §6.1.4(b): replace each issuer policy's expected set once, as a set.
    pub(super) fn map(&mut self, depth: usize, mappings: &PolicyMappings, permitted: bool) {
        let mut groups: Vec<(Oid, Vec<Oid>)> = Vec::new();
        for mapping in &mappings.0 {
            if let Some((_, subjects)) = groups
                .iter_mut()
                .find(|(issuer, _)| *issuer == mapping.issuer_domain_policy)
            {
                if !subjects.contains(&mapping.subject_domain_policy) {
                    subjects.push(mapping.subject_domain_policy);
                }
            } else {
                groups.push((
                    mapping.issuer_domain_policy,
                    vec![mapping.subject_domain_policy],
                ));
            }
        }
        for (issuer, mut subjects) in groups {
            subjects.sort();
            let matches: Vec<_> = self
                .at_depth(depth)
                .into_iter()
                .filter(|&id| self.nodes[id].value.valid_policy == issuer)
                .collect();
            if !permitted {
                for id in matches {
                    self.remove_subtree(id);
                }
            } else if !matches.is_empty() {
                for id in matches {
                    self.nodes[id].value.expected_policy_set = subjects.clone();
                }
            } else {
                // A synthesized mapping node is a sibling of this depth's
                // anyPolicy node, with that node's actual parent and qualifiers.
                let any_nodes: Vec<_> = self
                    .at_depth(depth)
                    .into_iter()
                    .filter(|&id| self.nodes[id].value.valid_policy == OID_ANY_POLICY)
                    .collect();
                for id in any_nodes {
                    if let Some(parent) = self.nodes[id].parent {
                        self.add(
                            parent,
                            issuer,
                            subjects.clone(),
                            self.nodes[id].value.qualifiers.clone(),
                        );
                    }
                }
            }
        }
        self.prune(depth);
    }

    /// §6.1.5(g): intersect using actual parents, deleting complete branches.
    pub(super) fn intersect(&mut self, initial: &[Oid], depth: usize) {
        // Preserve the provider's empty-set convention, and also the explicit
        // anyPolicy value used by the product's required initial-policy input.
        if initial.is_empty() || initial.contains(&OID_ANY_POLICY) {
            return;
        }
        let policy_nodes: Vec<_> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.live
                    && node.parent.is_some_and(|parent| {
                        self.nodes[parent].value.valid_policy == OID_ANY_POLICY
                    })
            })
            .map(|(id, _)| id)
            .collect();
        for &id in &policy_nodes {
            let policy = self.nodes[id].value.valid_policy;
            if policy != OID_ANY_POLICY && !initial.contains(&policy) {
                self.remove_subtree(id);
            }
        }
        let present: Vec<_> = policy_nodes
            .iter()
            .filter(|&&id| self.nodes[id].live)
            .map(|&id| self.nodes[id].value.valid_policy)
            .collect();
        let any_leaves: Vec<_> = self
            .at_depth(depth)
            .into_iter()
            .filter(|&id| self.nodes[id].value.valid_policy == OID_ANY_POLICY)
            .collect();
        for id in any_leaves {
            if let Some(parent) = self.nodes[id].parent {
                for &policy in initial {
                    if !present.contains(&policy)
                        && !self.nodes.iter().any(|node| {
                            node.live
                                && node.parent == Some(parent)
                                && node.value.valid_policy == policy
                        })
                    {
                        self.add(
                            parent,
                            policy,
                            vec![policy],
                            self.nodes[id].value.qualifiers.clone(),
                        );
                    }
                }
            }
            self.remove_subtree(id);
        }
        self.prune(depth);
    }

    pub(super) fn into_nodes(self) -> Vec<PolicyTreeNode> {
        self.nodes
            .into_iter()
            .filter(|node| node.live)
            .map(|node| node.value)
            .collect()
    }
}
