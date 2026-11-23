pub struct IndexedStrings {
    strings: Vec<String>,
}

impl IndexedStrings {
    pub fn new(strings: Vec<String>) -> Self {
        IndexedStrings { strings }
    }

    /// The string at a header index; `None` for negative or out-of-range
    /// indices.
    pub fn get(&self, index: i32) -> Option<&String> {
        self.strings.get(usize::try_from(index).ok()?)
    }
}
