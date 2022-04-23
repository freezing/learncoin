use crate::{Block, BlockHash, BlockHeader, Sha256};
use std::fmt::Write;

// TODO: Should be moved to the commands/ folder.
pub struct Graphwiz {}

impl Graphwiz {
    /// Generate the Graphwiz syntax such that there are two types of nodes:
    ///   - A node representing a block in the active chain
    ///   - A node representing a block in the secondary chain
    ///
    /// See https://graphviz.org/ for more info on the syntax details.
    ///
    ///  We would like to end up with the following:
    ///
    ///  digraph G {
    ///    subgraph cluster_0 {
    ///      style=filled;
    ///      color=lightgrey;
    ///      node [style=filled,color=white];
    ///      a0 -> a1 -> a2 -> a3;
    ///      label = "Active";
    ///    }
    ///
    ///    s0 -> s1;
    ///    s1 -> s2;
    ///  }
    ///
    /// We assume that all blocks have a parent.
    pub fn blockchain(
        all: Vec<BlockHeader>,
        active_blocks: &Vec<Block>,
        suffix_suffix: usize,
    ) -> String {
        let mut code = String::new();

        let genesis_block_parent = BlockHash::new(Sha256::from_raw([0; 32]));

        let active_blocks_code = active_blocks
            .iter()
            .map(|block| format!(r#""{}""#, Self::hash_suffix(block.id(), suffix_suffix)))
            .collect::<Vec<String>>()
            .join(" -> ");
        let all_blocks_code = all
            .iter()
            .filter(|block_header| !Self::is_active(block_header, active_blocks))
            .map(|block_header| {
                let parent = Self::hash_suffix(&block_header.previous_block_hash(), suffix_suffix);
                let child = Self::hash_suffix(&block_header.hash(), suffix_suffix);
                // Don't print the parent of the genesis block.
                if block_header.previous_block_hash() == genesis_block_parent {
                    format!(r#""{}""#, child)
                } else {
                    format!(r#""{}" -> "{}";"#, parent, child)
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        writeln!(&mut code, "digraph G {{").unwrap();

        writeln!(&mut code, "  subgraph cluster_0 {{").unwrap();
        writeln!(&mut code, "    style=filled;").unwrap();
        writeln!(&mut code, "    color=lightgrey;").unwrap();
        writeln!(&mut code, "    node [style=filled,color=white];").unwrap();
        writeln!(&mut code, "    {};", active_blocks_code).unwrap();
        writeln!(&mut code, "    label = \"Active\";").unwrap();
        writeln!(&mut code, "  }}").unwrap();

        writeln!(&mut code, "  {}", all_blocks_code).unwrap();
        writeln!(&mut code, "}}").unwrap();

        code
    }

    fn hash_suffix(hash: &BlockHash, suffix_length: usize) -> String {
        // Safety: BlockHash string representation matches the ASCII reprsentation, so it's safe
        // to unwrap the UTF-8 string slice.
        let s = hash.to_string();
        s.as_str()
            .get((s.len() - suffix_length)..)
            .unwrap()
            .to_string()
    }

    fn is_active(header: &BlockHeader, active_blocks: &Vec<Block>) -> bool {
        active_blocks
            .iter()
            .any(|active| *active.id() == header.hash())
    }
}
