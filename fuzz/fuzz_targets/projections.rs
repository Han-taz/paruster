#![no_main]

use kordoc_ir::{ChunkOptions, IrBlock};
use libfuzzer_sys::fuzz_target;
use serde::de::{DeserializeSeed, Error as _, IgnoredAny, SeqAccess, Visitor};
use std::fmt;

const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
const MAX_BLOCKS: usize = 100_000;

struct BlocksSeed;

impl<'de> DeserializeSeed<'de> for BlocksSeed {
    type Value = Vec<IrBlock>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(BlocksVisitor)
    }
}

struct BlocksVisitor;

impl<'de> Visitor<'de> for BlocksVisitor {
    type Value = Vec<IrBlock>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded array of IR blocks")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut blocks = Vec::new();
        loop {
            if blocks.len() == MAX_BLOCKS {
                if sequence.next_element::<IgnoredAny>()?.is_some() {
                    return Err(A::Error::custom("IR block array exceeds fuzz limit"));
                }
                return Ok(blocks);
            }
            match sequence.next_element::<IrBlock>()? {
                Some(block) => blocks.push(block),
                None => return Ok(blocks),
            }
        }
    }
}

fn parse_bounded_blocks(data: &[u8]) -> Option<Vec<IrBlock>> {
    if data.len() > MAX_JSON_BYTES {
        return None;
    }
    let mut deserializer = serde_json::Deserializer::from_slice(data);
    let blocks = BlocksSeed.deserialize(&mut deserializer).ok()?;
    deserializer.end().ok()?;
    Some(blocks)
}

fuzz_target!(|data: &[u8]| {
    let Some(mut blocks) = parse_bounded_blocks(data) else {
        return;
    };

    let _ = kordoc_core::blocks_to_markdown(&blocks);
    let _ = kordoc_core::blocks_to_pages(&blocks, None, kordoc_core::blocks_to_markdown);
    let _ = kordoc_core::blocks_to_chunks(&blocks, ChunkOptions::default());
    let _ = kordoc_core::table::classifier::classify_table_tree(&mut blocks);
});
