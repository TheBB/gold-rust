use std::collections::HashMap;

use symbol_table::GlobalSymbol;


// Boxable
// ----------------------------------------------------------------

pub trait Boxable<T> where T: Sized {
    fn to_box(self) -> Box<T>;
}

impl<T> Boxable<T> for Box<T> {
    fn to_box(self) -> Box<T> { self }
}

impl<T> Boxable<T> for T {
    fn to_box(self) -> Box<T> { Box::new(self) }
}


// Splattable
// ----------------------------------------------------------------

pub struct Splat<T> {
    pub object: T
}

pub trait Splattable<T> {
    fn splat(&self) -> Splat<T>;
}


// ToVec
// ----------------------------------------------------------------

pub trait ToVec<T> {
    fn to_vec(self) -> Vec<T>;
}

impl<T> ToVec<T> for () {
    fn to_vec(self) -> Vec<T> {
        vec![]
    }
}

impl<A,T> ToVec<T> for (A,)
where
    T: From<A>,
{
    fn to_vec(self) -> Vec<T> {
        vec![
            T::from(self.0),
        ]
    }
}

impl<A,B,T> ToVec<T> for (A,B)
where
    T: From<A>,
    T: From<B>,
{
    fn to_vec(self) -> Vec<T> {
        vec![
            T::from(self.0),
            T::from(self.1),
        ]
    }
}

impl<A,B,C,T> ToVec<T> for (A,B,C)
where
    T: From<A>,
    T: From<B>,
    T: From<C>,
{
    fn to_vec(self) -> Vec<T> {
        vec![
            T::from(self.0),
            T::from(self.1),
            T::from(self.2),
        ]
    }
}

impl<A,B,C,D,T> ToVec<T> for (A,B,C,D)
where
    T: From<A>,
    T: From<B>,
    T: From<C>,
    T: From<D>,
{
    fn to_vec(self) -> Vec<T> {
        vec![
            T::from(self.0),
            T::from(self.1),
            T::from(self.2),
            T::from(self.3),
        ]
    }
}

impl<A,B,C,D,E,T> ToVec<T> for (A,B,C,D,E)
where
    T: From<A>,
    T: From<B>,
    T: From<C>,
    T: From<D>,
    T: From<E>,
{
    fn to_vec(self) -> Vec<T> {
        vec![
            T::from(self.0),
            T::from(self.1),
            T::from(self.2),
            T::from(self.3),
            T::from(self.4),
        ]
    }
}


// ToMap
// ----------------------------------------------------------------

pub trait ToMap<K,V> {
    fn to_map(self) -> HashMap<K,V>;
}

impl<K,V> ToMap<K,V> for () {
    fn to_map(self) -> HashMap<K,V> {
        HashMap::new()
    }
}

impl<Ak,Av,V> ToMap<GlobalSymbol,V> for ((Ak,Av),)
where
    Ak: AsRef<str>,
    V: From<Av>,
{
    fn to_map(self) -> HashMap<GlobalSymbol,V> {
        let mut ret = HashMap::new();
        ret.insert(GlobalSymbol::new(self.0.0), V::from(self.0.1));
        ret
    }
}

impl<Ak,Av,Bk,Bv,V> ToMap<GlobalSymbol,V> for ((Ak,Av), (Bk,Bv))
where
    Ak: AsRef<str>,
    Bk: AsRef<str>,
    V: From<Av>,
    V: From<Bv>,
{
    fn to_map(self) -> HashMap<GlobalSymbol,V> {
        let mut ret = HashMap::new();
        ret.insert(GlobalSymbol::new(self.0.0), V::from(self.0.1));
        ret.insert(GlobalSymbol::new(self.1.0), V::from(self.1.1));
        ret
    }
}

impl<Ak,Av,Bk,Bv,Ck,Cv,V> ToMap<GlobalSymbol,V> for ((Ak,Av), (Bk,Bv), (Ck,Cv))
where
    Ak: AsRef<str>,
    Bk: AsRef<str>,
    Ck: AsRef<str>,
    V: From<Av>,
    V: From<Bv>,
    V: From<Cv>,
{
    fn to_map(self) -> HashMap<GlobalSymbol,V> {
        let mut ret = HashMap::new();
        ret.insert(GlobalSymbol::new(self.0.0), V::from(self.0.1));
        ret.insert(GlobalSymbol::new(self.1.0), V::from(self.1.1));
        ret.insert(GlobalSymbol::new(self.2.0), V::from(self.2.1));
        ret
    }
}

impl<Ak,Av,Bk,Bv,Ck,Cv,Dk,Dv,V> ToMap<GlobalSymbol,V> for ((Ak,Av), (Bk,Bv), (Ck,Cv), (Dk,Dv))
where
    Ak: AsRef<str>,
    Bk: AsRef<str>,
    Ck: AsRef<str>,
    Dk: AsRef<str>,
    V: From<Av>,
    V: From<Bv>,
    V: From<Cv>,
    V: From<Dv>,
{
    fn to_map(self) -> HashMap<GlobalSymbol,V> {
        let mut ret = HashMap::new();
        ret.insert(GlobalSymbol::new(self.0.0), V::from(self.0.1));
        ret.insert(GlobalSymbol::new(self.1.0), V::from(self.1.1));
        ret.insert(GlobalSymbol::new(self.2.0), V::from(self.2.1));
        ret.insert(GlobalSymbol::new(self.3.0), V::from(self.3.1));
        ret
    }
}

impl<Ak,Av,Bk,Bv,Ck,Cv,Dk,Dv,Ek,Ev,V> ToMap<GlobalSymbol,V> for ((Ak,Av), (Bk,Bv), (Ck,Cv), (Dk,Dv), (Ek,Ev))
where
    Ak: AsRef<str>,
    Bk: AsRef<str>,
    Ck: AsRef<str>,
    Dk: AsRef<str>,
    Ek: AsRef<str>,
    V: From<Av>,
    V: From<Bv>,
    V: From<Cv>,
    V: From<Dv>,
    V: From<Ev>,
{
    fn to_map(self) -> HashMap<GlobalSymbol,V> {
        let mut ret = HashMap::new();
        ret.insert(GlobalSymbol::new(self.0.0), V::from(self.0.1));
        ret.insert(GlobalSymbol::new(self.1.0), V::from(self.1.1));
        ret.insert(GlobalSymbol::new(self.2.0), V::from(self.2.1));
        ret.insert(GlobalSymbol::new(self.3.0), V::from(self.3.1));
        ret.insert(GlobalSymbol::new(self.4.0), V::from(self.4.1));
        ret
    }
}
