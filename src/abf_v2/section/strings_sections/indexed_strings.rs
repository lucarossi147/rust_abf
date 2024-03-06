pub struct  IndexedStrings {
    strings: Vec<String>,
}

impl IndexedStrings {
    pub fn new(strings: Vec<String>) -> Self {
        IndexedStrings { strings }
    }
    pub fn get(&self, index: i32) -> Option<&String> {
        self.strings.get(index as usize)
    }
}