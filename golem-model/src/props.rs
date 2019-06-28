pub use rust_decimal::Decimal;
pub use semver::Version;
use std::sync::Arc;
use std::hash::Hash;
use std::fmt;
use core::fmt::{Pointer, Debug};
use std::collections::{HashSet, HashMap};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::str::FromStr;
use failure::Fail;

pub type DateTime = chrono::DateTime<chrono::Utc>;

pub enum PropValue {
    String(String),
    Bool(bool),
    Number(f64),
    Decimal(Decimal),
    DateTime(DateTime),
    Version(Version),
    Array(Vec<String>)
}

#[derive(Hash, Eq, Ord, PartialOrd, PartialEq)]
pub enum PropKey {
    Simple(Arc<str>),
    Complex {
        base: Arc<str>,
        args : Vec<String>
    }
}

pub struct PropertySet {
    map : HashMap<PropKey, PropValue>
}

impl PropertySet {

    pub fn new() -> Self {
        PropertySet { map: HashMap::new() }
    }

    #[inline]
    pub fn get(&self, k :&PropKey) -> Option<&PropValue> {
        self.get(k)
    }

    #[inline]
    pub fn put(&mut self, k : PropKey, v : PropValue) {
        let _ = self.map.insert(k, v);
    }

    pub fn items(&self) -> impl Iterator<Item=(&PropKey, &PropValue)> {
        self.map.iter()
    }

}

impl PropKey {

    #[inline]
    pub fn base(&self) -> &Arc<str> {
        match self {
            PropKey::Simple(v) => v,
            PropKey::Complex { base, ..} => base
        }
    }

    pub fn with_args(&self, args : impl IntoIterator<Item=impl Into<String>>) -> Self {
        let base = self.base().clone();
        let args = args.into_iter().map(Into::into).collect();

        PropKey::Complex { base, args }
    }

    pub fn new(s : &str) -> Self {
        PropKey::Simple(alloc_str(s))
    }

}


#[derive(Fail, Debug)]
pub enum ParseError {
    #[fail(display = "property key is empty")]
    EmptyBase

}

impl FromStr for PropKey {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(n)= s.find("{") {
            if n == 0 {
                return Err(ParseError::EmptyBase)
            }
            unimplemented!()
        }
        else {
            Ok(PropKey::Simple(alloc_str(s)))
        }
    }
}

thread_local! {
    static KEYS: RefCell<HashSet<Arc<str>>> = RefCell::new(HashSet::new());
}

fn alloc_str(s : &str) -> Arc<str> {
    KEYS.with(|keys| {
        let mut map = keys.borrow_mut();

        if let Some(k) = map.get(s) {
            k.clone()
        }
        else {
            let k : Arc<str> = s.into();

            let _ = map.insert(k.clone());

            k
        }
    })
}

impl fmt::Display for PropKey {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PropKey::Simple(s) => {
                write!(f, "{}", s)?;
            }
            PropKey::Complex { base, args} => {
                write!(f, "{}", base)?;
                for arg in args {
                    write!(f, "{}{}{}", '{', arg, '}')?;
                }
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use super::*;


    #[test]
    fn test_arc() {
        let a : PropKey= PropKey::new("key1");
        let c : PropKey = PropKey::new("key1").with_args(vec!["v1", "v2"]);

        eprintln!("a={}, c={}, n={}", a, c, Arc::strong_count(a.base()));
    }
}
