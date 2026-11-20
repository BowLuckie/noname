use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Literals {
    map: HashMap<Box<str>, StrId>,
    strs: Vec<Box<str>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StrId {
    inner: u32,
}

impl Literals {
    pub fn intern_lit(&mut self, s: &str) -> StrId {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }

        let sym = StrId {
            inner: self.strs.len() as u32,
        };

        self.strs.push(s.into());
        self.map.insert(s.into(), sym);

        sym
    }

    pub fn resolve_lit(&self, sym: StrId) -> &str {
        &self.strs[sym.inner as usize]
    }
}
