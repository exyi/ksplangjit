use std::{any::{Any, TypeId}, borrow::Borrow, fmt::Debug, panic::RefUnwindSafe};

use num_traits::{Bounded, CheckedMul, One, SaturatingAdd, SaturatingMul, SaturatingSub, Zero};
use smallvec::Array;

use crate::prelude::*;

pub const EMPTY_RANGE: RangeInclusive<i64> = 1..=0;
pub const FULL_RANGE: RangeInclusive<i64> = i64::MIN..=i64::MAX;


pub fn range_size(r: &RangeInclusive<i64>) -> u128 {
    if r.is_empty() {
        0
    } else {
        r.end().abs_diff(*r.start()) as u128 + 1
    }
}

pub fn u64neg(a: u64) -> i64 {
    (a as i64).wrapping_neg()
}

pub fn abs_range(r: impl Borrow<RangeInclusive<i64>>) -> RangeInclusive<u64> {
    let (a, b) = r.borrow().clone().into_inner();
    if (a >= 0) == (b >= 0) {
        let (a, b) = sort_tuple(a.abs_diff(0), b.abs_diff(0));
        a..=b
    } else {
        0..=cmp::max(a.abs_diff(0), b.abs_diff(0))
    }
}

#[inline]
pub fn add_range(a: &RangeInclusive<i64>, b: &RangeInclusive<i64>) -> RangeInclusive<i64> {
    let start = a.start().saturating_add(b.start());
    let end = a.end().saturating_add(b.end());
    start..=end
}

#[inline]
pub fn sub_range(a: &RangeInclusive<i64>, b: &RangeInclusive<i64>) -> RangeInclusive<i64> {
    let start = a.start().saturating_sub(b.end());
    let end = a.end().saturating_sub(b.start());
    start..=end
}

/// Returns true if the range does not include both negative and positive numbers
#[inline]
pub fn range_is_signless(r: &RangeInclusive<i64>) -> bool {
    *r.start() >= 0 || *r.end() <= 0
}

#[inline]
pub fn range_sign(r: &RangeInclusive<i64>) -> i64 {
    if *r.start() >= 0 {
        1
    } else if *r.end() <= 0 {
        -1
    } else {
        0
    }
}

pub fn mul_range(a: &RangeInclusive<i64>, b: &RangeInclusive<i64>) -> (RangeInclusive<i64>, bool) {
    let candidates = [
        a.start().saturating_mul(b.start()),
        a.start().saturating_mul(b.end()),
        a.end().saturating_mul(b.start()),
        a.end().saturating_mul(b.end()),
    ];
    let may_overflow = a.start().checked_mul(b.start()).is_none() ||
                             a.start().checked_mul(b.end()).is_none() ||
                             a.end().checked_mul(b.start()).is_none() ||
                             a.end().checked_mul(b.end()).is_none();
    let min = *candidates.iter().min().unwrap();
    let max = *candidates.iter().max().unwrap();
    (min..=max, may_overflow)
}

pub fn union_range(a: impl Borrow<RangeInclusive<i64>>, b: impl Borrow<RangeInclusive<i64>>) -> RangeInclusive<i64> {
    let a = a.borrow();
    let b = b.borrow();
    let start = cmp::min(*a.start(), *b.start());
    let end = cmp::max(*a.end(), *b.end());
    start..=end
}

pub fn intersect_range<T: Ord + Zero + One + Clone>(a: impl Borrow<RangeInclusive<T>>, b: impl Borrow<RangeInclusive<T>>) -> RangeInclusive<T> {
    let a = a.borrow();
    let b = b.borrow();
    let start = cmp::max(a.start(), b.start()).clone();
    let end = cmp::min(a.end(), b.end()).clone();
    if start > end {
        T::one()..=T::zero()
    } else {
        start..=end
    }
}

pub fn range_2_i64(r: RangeInclusive<u64>) -> RangeInclusive<i64> {
    let (a, b) = r.into_inner();
    if a > i64::MAX as u64 {
        1..=0
    } else if b > i64::MAX as u64 {
        a as i64..=i64::MAX
    } else {
        a as i64..=b as i64
    }
}

pub fn sort_tuple<T: Ord>(a: T, b: T) -> (T, T) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

pub fn eval_combi_u64<F: Fn(u64, u64) -> Option<u64>>(
    a: RangeInclusive<u64>,
    b: RangeInclusive<u64>,
    max_combination: u64,
    f: F,
) -> Option<RangeInclusive<u64>> {
    if a.is_empty() || b.is_empty() {
        return Some(1..=0);
    }

    let size_a = a.end().abs_diff(*a.start()).saturating_add(1);
    let size_b = b.end().abs_diff(*b.start()).saturating_add(1);
    if size_a.saturating_mul(size_b) <= max_combination {
        let mut values = HashSet::default();
        for x in a.clone() {
            for y in b.clone() {
                if let Some(value) = f(x, y) {
                    values.insert(value);
                }
            }
        }
        if values.is_empty() {
            return Some(1..=0);
        }
        let min = *values.iter().min().unwrap();
        let max = *values.iter().max().unwrap();
        Some(min..=max)
    } else {
        None
    }
}



pub trait SaturatingInto<T> {
    fn saturating_into(self) -> T;
}

impl <T, U> SaturatingInto<U> for T
where T: Clone + TryFrom<U> + Ord,
      U: TryFrom<T> + Bounded + Clone
{
    fn saturating_into(self) -> U {
        if let Ok(min) = T::try_from(U::min_value()) {
            if self < min {
                return U::min_value();
            }
        }
        if let Ok(max) = T::try_from(U::max_value()) {
            if self > max {
                return U::max_value();
            }
        }
        let Ok(result) = self.try_into() else {
            unreachable!("saturating_into: conversion failed unexpectedly")
        };
        result
    }
}

pub trait AssertInto<T> {
    fn assert_into(self) -> T;
}
impl <T, U> AssertInto<U> for T
where U: TryFrom<T>,
      T: Clone + Debug
{
    #[inline]
    fn assert_into(self) -> U {
        #[cfg(debug_assertions)]
        let Ok(result) = self.clone().try_into() else {
            panic!("assert_into: conversion {} -> {} failed unexpectedly: {:?}", std::any::type_name::<T>(), std::any::type_name::<U>(), self);
        };
        #[cfg(not(debug_assertions))]
        let Ok(result) = self.try_into() else { unreachable!() };
        result
    }
}

pub trait RemoveAll {
    fn remove_all(&mut self, ixs: &[usize]);
}

macro_rules! impl_remove_all {
    ($arr:expr, $ixs:expr) => {{
        debug_assert!($ixs.is_sorted());
        let mut ix = 0;
        let mut iter = $ixs.iter().peekable();
        $arr.retain(|_| {
            if let Some(&&next) = iter.peek() && next <= ix {
                debug_assert_eq!(next, ix);
                iter.next();
                ix += 1;
                false
            } else {
                ix += 1;
                true
            }
        });
        debug_assert_eq!($arr.len(), ix - $ixs.len());
    }}
}
impl<T> RemoveAll for Vec<T> {
    fn remove_all(&mut self, ixs: &[usize]) {
        impl_remove_all!(self, ixs)
    }
}
impl<A: Array> RemoveAll for SmallVec<A> {
    fn remove_all(&mut self, ixs: &[usize]) {
        impl_remove_all!(self, ixs)
    }
}

pub trait AnnotationObj: Any + Debug + RefUnwindSafe {
    fn type_name(&self) -> &str { std::any::type_name::<Self>() }
}

#[derive(Default, Clone)]
pub struct Annotations {
    data: Option<Box<HashMap<TypeId, Arc<dyn AnnotationObj>>>>
}

impl Annotations {
    pub fn new() -> Self { Self::default() }
    pub fn len(&self) -> usize {
        let Some(data) = &self.data else { return 0 };
        data.len()
    }
    pub fn get<T: 'static + AnnotationObj>(&self) -> Option<&T> {
        let x = self.data.as_ref()?.get(&TypeId::of::<T>())?;
        let x = x.as_ref();
        let xy: &dyn Any = x;
        xy.downcast_ref()
    }
    pub fn set<T: 'static + AnnotationObj>(&mut self, x: T) {
        let data = self.data.get_or_insert_with(|| Box::new(HashMap::default()));
        data.insert(TypeId::of::<T>(), Arc::new(x));
    }
    pub fn remove<T: 'static + AnnotationObj>(&mut self) -> bool {
        if let Some(data) = &mut self.data {
            data.remove(&TypeId::of::<T>()).is_some()
        } else {
            false
        }
    }
}

pub fn all_equal<T: PartialEq>(mut it: impl Iterator<Item = T>) -> bool {
    let Some(first) = it.next() else { return true };
    for x in it {
        if first != x {
            return false
        }
    }
    true
}

impl PartialEq for Annotations {
    fn eq(&self, _other: &Self) -> bool {
        true // hack...
    }
}

impl Debug for Annotations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some(data) = &self.data else { return write!(f, "{{}}") };

        let mut map = f.debug_map();
        for instance in data.as_ref().values() {
            map.entry(&instance.as_ref().type_name(), instance.as_ref());
        }
        map.finish()
    }
}


#[derive(Copy, Clone, Eq, PartialEq)]
pub struct NumFmt<T>(pub T);

impl<T: Into<i128> + Bounded + Clone> fmt::Display for NumFmt<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num: i128 = self.0.clone().into();
        if num.abs() <= 1050 {
            return write!(f, "{}", num)
        }
        if num == T::min_value().into() {
            return write!(f, "MIN")
        }
        if num == T::max_value().into() {
            return write!(f, "MAX")
        }
        if num <= T::min_value().into() + 256 {
            return write!(f, "MIN+{}", num.abs_diff(T::min_value().into()))
        }
        if num >= T::max_value().into() - 256 {
            return write!(f, "MAX-{}", num.abs_diff(T::max_value().into()))
        }

        let decimal = num.to_string();
        let hex = format!("{:X}", num.unsigned_abs());

        if hex.chars().filter(|&x| x != '0').collect::<BTreeSet<char>>().len() < decimal.chars().filter(|&x| x != '0').collect::<BTreeSet<char>>().len() {
            let sign = if num < 0 { "-" } else { "" };
            write!(f, "{sign}0x{hex}")
        } else {
            write!(f, "{decimal}")
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct RangeFmt<T>(pub RangeInclusive<T>);

impl<T: Bounded + Into<i128> + Eq + Clone> fmt::Display for RangeFmt<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = &self.0;
        if r.start() == &T::min_value() {
            if r.end() == &T::max_value() {
                write!(f, " ..= ")
            } else {
                write!(f, " ..={}", NumFmt(r.end().clone()))
            }
        } else {
            if r.end() == &T::max_value() {
                write!(f, "{}..= ", NumFmt(r.start().clone()))
            } else {
                write!(f, "{}..={}", NumFmt(r.start().clone()), NumFmt(r.end().clone()))
            }
        }
    }
}

pub fn int_to_letters(mut num: u64) -> String {
    if num == 0 { return "0".to_owned() }

    let mut chars = Vec::new();
    while num > 0 {
        num -= 1;
        chars.push((b'A' + (num % 26) as u8) as char);
        num /= 26;
    }

    chars.iter().rev().collect()
}
