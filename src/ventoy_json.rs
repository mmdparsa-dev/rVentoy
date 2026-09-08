use serde_json::Value;

pub struct VentoyJson {
    pub root: Value,
}

impl VentoyJson {
    pub fn new() -> Self {
        Self { root: Value::Null }
    }

    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        let root: Value = serde_json::from_str(json_str)?;
        Ok(Self { root })
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.root.get(key)?.as_str()
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.root.get(key)?.as_i64()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.root.get(key)?.as_bool()
    }
}
