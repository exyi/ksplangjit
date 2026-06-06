pub use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
pub use std::collections::{BTreeMap, BTreeSet};
pub use std::{assert_matches, debug_assert_matches, fmt, mem, cmp, hint};
pub use std::borrow::Cow;
pub use std::sync::Arc;
pub use std::ops::{RangeInclusive, RangeBounds};
pub use smallvec::{SmallVec, smallvec, ToSmallVec};

pub use num_integer::Integer;

pub type IRange = RangeInclusive<i64>;
pub type URange = RangeInclusive<u64>;

pub use crate::compiler::utils::{AssertInto, SaturatingInto};
