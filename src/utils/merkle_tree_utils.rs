use alloy::primitives::{B256, keccak256};
use anyhow::{Result, anyhow, ensure};
use std::collections::HashMap;
use serde_json::Value;

// Helper function to calculate log2 ceiling
fn log2ceil(n: usize) -> usize {
    if n <= 1 {
        0
    } else {
        (n - 1).ilog2() as usize + 1
    }
}

#[derive(Debug)]
pub struct MerkleTreeData {
    pub root: Vec<u8>,
    pub proofs: Value,
    pub root_hex: String,
}


pub fn create_merkle_tree(addresses: &[String]) -> Result<MerkleTreeData> {

    let mut leaves = Vec::with_capacity(addresses.len());
    for addr in addresses {
        let data = hex::decode(&addr[2..])
            .map_err(|e| anyhow!("Bad hex in {}: {}", addr, e))?;
        ensure!(data.len() == 20, "Addr {} not 20 bytes", addr);
        leaves.push((addr.clone(), B256::from_slice(keccak256(&data).as_slice())));
    }

    let mut layers: Vec<Vec<B256>> = Vec::with_capacity(log2ceil(leaves.len()));
    layers.push(leaves.iter().map(|(_, h)| *h).collect());

    let mut buf = Vec::with_capacity(64);
    while layers.last().unwrap().len() > 1 {
        let cur = &layers.last().unwrap();
        let mut next = Vec::with_capacity((cur.len()+1)/2);
        for pair in cur.chunks(2) {
            if let [l, r] = pair {
                let (a, b) = if l < r { (l, r) } else { (r, l) };
                buf.clear();
                buf.extend_from_slice(a.as_slice());
                buf.extend_from_slice(b.as_slice());
                next.push(B256::from_slice(keccak256(&buf).as_slice()));
            } else {
                next.push(pair[0]);
            }
        }
        layers.push(next);
    }

    
    let root = layers.last().unwrap()[0];
    let root_hex = format!("0x{}", hex::encode(root.as_slice()));

    
    let mut proofs = HashMap::with_capacity(leaves.len());
    for (i, (addr, _leaf)) in leaves.iter().enumerate() {
        let mut idx = i;
        let mut proof = Vec::with_capacity(layers.len());
        for level in &layers[..layers.len()-1] {
            let sibling = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            if sibling < level.len() {
                proof.push(format!("0x{}", hex::encode(level[sibling].as_slice())));
            }
            idx /= 2;
        }
        proofs.insert(addr.clone(), proof);
    }

    Ok(MerkleTreeData {
        root: root.as_slice().to_vec(),
        proofs: serde_json::to_value(proofs)?,
        root_hex,
    })
}
