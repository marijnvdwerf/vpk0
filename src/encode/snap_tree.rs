//! Top-down greedy tree derivation matching the original Pokemon Snap
//! VPK0 tool.
//!
//! Instead of standard Huffman, the original tool builds the tree by
//! recursively splitting contiguous width ranges when a split has
//! positive local bit-saving.

use super::huffman::HuffCode;
use super::lzss::LzssPass;
use super::BitSize;
use crate::format::{TreeEntry, VpkTree};
use std::collections::HashMap;

pub(super) type CodeMap = HashMap<BitSize, (BitSize, HuffCode)>;

pub(super) struct SnapTree {
    pub tree: VpkTree,
    pub map: CodeMap,
}

pub(super) fn build_trees(lzss: &LzssPass) -> (SnapTree, SnapTree) {
    let offsets = build_one(&lzss.moveback_bitfreq);
    let lengths = build_one(&lzss.size_bitfreq);
    (offsets, lengths)
}

fn build_one(freq_map: &HashMap<BitSize, u64>) -> SnapTree {
    if freq_map.is_empty() {
        return SnapTree {
            tree: VpkTree::empty(),
            map: CodeMap::new(),
        };
    }

    let max_w = *freq_map.keys().max().unwrap() as usize;
    let min_w = *freq_map.keys().min().unwrap() as usize;

    // Build cumulative histogram. cum[w] = total count with bit_size <= w.
    // Index 0 is valid (cum[0] = count of bit_size 0).
    let mut cum = vec![0i64; max_w + 2];
    cum[0] = *freq_map.get(&0u8).unwrap_or(&0) as i64;
    for w in 1..=max_w {
        cum[w] = cum[w - 1] + *freq_map.get(&(w as u8)).unwrap_or(&0) as i64;
    }

    let freq = |lo: usize, hi: usize| -> i64 {
        if lo == 0 {
            cum[hi]
        } else {
            cum[hi] - cum[lo - 1]
        }
    };

    // Recursive top-down splitter. Returns Vec<TreeEntry> in post-order.
    fn split(
        lo: usize,
        hi: usize,
        cum: &[i64],
        freq_fn: &dyn Fn(usize, usize) -> i64,
    ) -> Vec<TreeEntry> {
        if lo == hi {
            return vec![TreeEntry::Leaf(hi as u8)];
        }

        let total = freq_fn(lo, hi);
        let mut best_gain: i64 = 0;
        let mut best_k: Option<usize> = None;

        for k in lo..hi {
            let left_freq = freq_fn(lo, k);
            let gain = (hi as i64 - k as i64) * left_freq - total - 10;
            if gain > best_gain {
                best_gain = gain;
                best_k = Some(k);
            }
        }

        let best_k = match best_k {
            Some(k) => k,
            None => return vec![TreeEntry::Leaf(hi as u8)],
        };

        let left = split(lo, best_k, cum, freq_fn);
        let right = split(best_k + 1, hi, cum, freq_fn);

        let mut combined = left;
        let r_offset = combined.len();
        let left_root = r_offset - 1;

        for entry in &right {
            match entry {
                TreeEntry::Node { left, right } => {
                    combined.push(TreeEntry::Node {
                        left: left + r_offset,
                        right: right + r_offset,
                    });
                }
                TreeEntry::Leaf(v) => {
                    combined.push(TreeEntry::Leaf(*v));
                }
            }
        }
        let right_root = combined.len() - 1;
        combined.push(TreeEntry::Node {
            left: left_root,
            right: right_root,
        });

        combined
    }

    let entries = split(min_w, max_w, &cum, &freq);
    let tree = VpkTree::from_entries(entries);

    // Build code map by walking the tree.
    let mut map = CodeMap::new();
    build_codes(&tree, &mut map);

    SnapTree { tree, map }
}

fn build_codes(tree: &VpkTree, map: &mut CodeMap) {
    fn walk(tree: &VpkTree, idx: usize, code: u32, len: u8, map: &mut CodeMap) {
        match tree.entry(idx) {
            TreeEntry::Leaf(bit_size) => {
                map.insert(bit_size, (bit_size, HuffCode::create(code, len)));
            }
            TreeEntry::Node { left, right } => {
                walk(tree, left, code << 1, len + 1, map);
                walk(tree, right, (code << 1) | 1, len + 1, map);
            }
        }
    }
    if tree.len() > 0 {
        walk(tree, tree.len() - 1, 0, 0, map);
    }
}
