use std::collections::HashMap;

use crate::frontend::Token;

#[derive(Debug, Default, Clone)]
pub struct Literals {
    map: HashMap<Box<str>, StrId>,
    strs: Vec<Box<str>>,
    fstrs: Vec<FString>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StrId {
    inner: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FStrId {
    inner: u32,
}

#[derive(Debug, Clone)]
pub enum FPart {
    Text(StrId),
    Expr(Vec<Token>),
}

#[derive(Debug, Default, Clone)]
pub struct FString {
    pub parts: Vec<FPart>,
}

impl From<Vec<FPart>> for FString {
    fn from(parts: Vec<FPart>) -> Self {
        Self { parts }
    }
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

    pub fn resolve_fstr(&self, fstr_id: FStrId) -> &FString {
        &self.fstrs[fstr_id.inner as usize]
    }

    pub fn len(&self, id: StrId) -> usize {
        self.strs[id.inner as usize].len()
    }

    pub fn add_fstr(&mut self, fs: FString) -> FStrId {
        let fstr_id = FStrId {
            inner: self.fstrs.len() as u32,
        };
        self.fstrs.push(fs);

        fstr_id
    }
}
