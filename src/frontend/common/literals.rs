use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Literals {
    map: HashMap<Box<str>, StrId>,
    strs: Vec<Box<str>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StrId {
    inner: u32,
}

impl Literals {
    pub fn intern_str(&mut self, s: &str) -> StrId {
        if let Some(&str_id) = self.map.get(s) {
            return str_id;
        }

        let str_id = StrId {
            inner: self.strs.len() as u32,
        };

        self.strs.push(s.into());
        self.map.insert(s.into(), str_id);

        str_id
    }

    pub fn resolve_str(&self, str_id: StrId) -> &str {
        &self.strs[str_id.inner as usize]
    }

    pub fn len(&self, id: StrId) -> usize {
        self.strs[id.inner as usize].len()
    }
}
