use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct CommandTree {
    pub version: u32,
    pub base_path: String,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct Resource {
    pub name: String,
    pub ops: Vec<Operation>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct Operation {
    pub name: String,
    pub method: String,
    pub path: String,
    pub deprecated: bool,
    pub params: Vec<Param>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct Param {
    pub name: String,
    pub flag: String,
}

pub fn load_command_tree() -> CommandTree {
    let raw = include_str!("../schemas/command_tree.json");
    serde_json::from_str(raw).expect("invalid command_tree.json")
}
