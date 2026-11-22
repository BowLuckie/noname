use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Interner {
    map: HashMap<Box<str>, Sym>,
    strs: Vec<Box<str>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Sym {
    inner: u32,
}

impl Interner {
    pub fn intern(&mut self, s: &str) -> Sym {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }

        let sym = Sym {
            inner: self.strs.len() as u32,
        };

        self.strs.push(s.into());
        self.map.insert(s.into(), sym);

        sym
    }

    pub fn resolve(&self, sym: Sym) -> &str {
        &self.strs[sym.inner as usize]
    }
}
