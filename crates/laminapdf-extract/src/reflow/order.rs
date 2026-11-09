//! How far the structure tree's order agrees with the layout's: on each page, the tree's blocks
//! in the tree's order against XY-Cut++'s order of the same boxes, by Kendall's τ (the share of
//! block pairs both put the same way round, less the share they put opposite ways). A tree that
//! reads a page's blocks backwards scores −1 wherever its blocks sit; a page laid out in a way
//! XY-Cut++ misreads costs the tree a few pairs, not the verdict.

use std::collections::BTreeMap;

use laminapdf_model::RectPt;
use laminapdf_syntax::CancelToken;

use super::tree::TreeFacts;
use crate::error::ExtractError;
use crate::lines::mostly_rtl;
use crate::model::Block;
use crate::xycut;

/// The fewest tagged blocks a page needs to count: fewer have no order worth comparing.
const MIN_BLOCKS: usize = 3;

/// The most blocks of one page compared, the first ones: τ looks at every pair.
const MAX_BLOCKS: usize = 400;

/// XY-Cut++'s deepest cut, as extraction's default limit.
const MAX_DEPTH: u32 = 64;

/// Kendall's τ of `ranks` (the layout's rank of each block, in the tree's order) against
/// 0, 1, 2, …: 1 when both agree, −1 when one reverses the other.
pub(crate) fn tau(ranks: &[usize]) -> f64 {
    let (mut agree, mut disagree) = (0_u64, 0_u64);
    for (i, a) in ranks.iter().enumerate() {
        for b in ranks.iter().skip(i + 1) {
            if b > a {
                agree += 1;
            } else {
                disagree += 1;
            }
        }
    }
    let pairs = agree + disagree;
    if pairs == 0 {
        return 1.0;
    }
    #[expect(clippy::cast_precision_loss, reason = "pair counts stay far below 2^52")]
    let (agree, disagree, pairs) = (agree as f64, disagree as f64, pairs as f64);
    (agree - disagree) / pairs
}

/// How many lines a block has: its runs' distinct baselines, to the point.
fn lines(block: &Block) -> usize {
    let mut tops: Vec<i64> = block
        .spans
        .iter()
        .map(|s| {
            #[expect(clippy::cast_possible_truncation, reason = "page coordinates are small")]
            let y = s.bbox.y1.get().round() as i64;
            y
        })
        .collect();
    tops.sort_unstable();
    tops.dedup();
    tops.len().max(1)
}

/// The tree's agreement with the layout over the pages of `blocks` whose blocks come from the
/// tree, weighted by their tagged blocks; `None` when no page has enough of them.
pub(crate) fn agreement(
    blocks: &[Block],
    facts: &TreeFacts,
    cancel: &CancelToken,
) -> Result<Option<f64>, ExtractError> {
    let mut pages: BTreeMap<usize, Vec<&Block>> = BTreeMap::new();
    for block in blocks {
        let tagged = block
            .struct_ref
            .as_ref()
            .is_some_and(|r| facts.order.contains_key(&laminapdf_cos::ObjRef::new(r.obj, r.r#gen)));
        if tagged {
            pages.entry(block.page).or_default().push(block);
        }
    }
    let (mut sum, mut weight) = (0.0, 0.0);
    for page in pages.values().filter(|p| p.len() >= MIN_BLOCKS) {
        let page = page.get(..MAX_BLOCKS).unwrap_or(page);
        let boxes: Vec<RectPt> = page.iter().map(|b| b.bbox).collect();
        let counts: Vec<usize> = page.iter().map(|b| lines(b)).collect();
        let text: String = page.iter().map(|b| b.text.as_str()).collect();
        let ordered =
            xycut::order(&boxes, &counts, mostly_rtl(&text), MAX_DEPTH, cancel).map_err(|_| ExtractError::Cancelled)?;
        let mut ranks = vec![0; page.len()];
        for (rank, &i) in ordered.order.iter().enumerate() {
            if let Some(slot) = ranks.get_mut(i) {
                *slot = rank;
            }
        }
        #[expect(clippy::cast_precision_loss, reason = "block counts stay far below 2^52")]
        let n = page.len() as f64;
        sum += tau(&ranks) * n;
        weight += n;
    }
    Ok((weight > 0.0).then(|| sum / weight))
}

#[cfg(test)]
mod tests {
    use super::tau;

    #[test]
    fn tau_is_one_for_the_same_order_and_minus_one_for_its_reverse() {
        assert!((tau(&[0, 1, 2, 3]) - 1.0).abs() < 1e-9);
        assert!((tau(&[3, 2, 1, 0]) + 1.0).abs() < 1e-9);
        // One swapped pair of six: (5 − 1) / 6.
        assert!((tau(&[1, 0, 2, 3]) - 4.0 / 6.0).abs() < 1e-9);
    }
}
