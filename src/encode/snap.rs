//! Lazy-match LZSS encoder matching the original Pokemon Snap VPK0 tool.
//!
//! Uses a hash-chain match finder with 1-position lazy lookahead: if
//! the longest match at `pos+1` is strictly longer than at `pos`, emit
//! a literal at `pos` and defer.

use super::lzss::{LzssByte, LzssPass, LzssSettings};
use crate::format::VpkMethod;

const HASH_BITS: usize = 16;
const HASH_SIZE: usize = 1 << HASH_BITS;
const HASH_MASK: usize = HASH_SIZE - 1;
const NIL: i32 = -1;

pub(super) fn compress(data: &[u8], settings: LzssSettings, method: VpkMethod) -> LzssPass {
    let n = data.len();
    let max_window = settings.window_size();
    let max_match = settings.max_encoded();

    let mut pass = LzssPass::new_with_size(n, &settings);

    if n == 0 {
        return pass;
    }

    let mut head = vec![NIL; HASH_SIZE];
    let mut prev = vec![NIL; n];

    let h3 = |pos: usize| -> usize {
        let b0 = data[pos] as usize;
        let b1 = data[pos + 1] as usize;
        let b2 = data[pos + 2] as usize;
        (b0.wrapping_mul(2654435761) ^ (b1 << 8) ^ b2) & HASH_MASK
    };

    let best_match_at = |pos: usize, head: &[i32], prev: &[i32]| -> (usize, usize) {
        if pos + 3 > n {
            return (0, 0);
        }
        let key = h3(pos);
        let window_start = pos.saturating_sub(max_window);
        let limit = max_match.min(n - pos);
        let mut best_len = 0usize;
        let mut best_off = 0usize;
        let mut j = head[key];
        while j >= 0 && (j as usize) >= window_start {
            let ju = j as usize;
            if data[ju + best_len] == data[pos + best_len] {
                let mut k = 0;
                while k < limit && data[ju + k] == data[pos + k] {
                    k += 1;
                }
                if k > best_len {
                    best_len = k;
                    best_off = pos - ju;
                    if k == limit {
                        break;
                    }
                }
            }
            j = prev[ju];
        }
        (best_off, best_len)
    };

    let insert = |pos: usize, head: &mut [i32], prev: &mut [i32]| {
        if pos + 3 > n {
            return;
        }
        let key = h3(pos);
        prev[pos] = head[key];
        head[key] = pos as i32;
    };

    let mut i = 0;
    while i < n {
        let (off_i, len_i) = best_match_at(i, &head, &prev);
        insert(i, &mut head, &mut prev);

        if len_i >= 3 {
            // Lazy check: peek one position ahead.
            if i + 1 < n {
                let (_, len_next) = best_match_at(i + 1, &head, &prev);
                if len_next > len_i {
                    pass.add_uncoded(data[i]);
                    i += 1;
                    continue;
                }
            }

            let byte = match method {
                VpkMethod::TwoSample => LzssByte::EncTwoSample(len_i, off_i.into()),
                VpkMethod::OneSample => LzssByte::Encoded(len_i, off_i),
            };
            pass.add(byte);

            for p in (i + 1)..(i + len_i) {
                insert(p, &mut head, &mut prev);
            }
            i += len_i;
        } else {
            pass.add_uncoded(data[i]);
            i += 1;
        }
    }

    pass
}
