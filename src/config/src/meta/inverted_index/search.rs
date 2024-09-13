use anyhow::Result;
use bitvec::vec::BitVec;
use fst::{
    automaton::{StartsWith, Str},
    Automaton, IntoStreamer, Streamer,
};
use futures::{AsyncRead, AsyncSeek};

use super::{
    reader::{Contains, IndexReader},
    ColumnIndexMeta,
};

pub struct SubstringSearch<'b> {
    terms: Vec<String>,
    meta: &'b ColumnIndexMeta,
}

impl<'b> SubstringSearch<'b> {
    pub fn new(terms: &[String], meta: &'b ColumnIndexMeta) -> Self {
        SubstringSearch {
            terms: terms.to_owned(),
            meta,
        }
    }

    pub async fn search<R>(&mut self, index_reader: &mut IndexReader<R>) -> Result<BitVec<u8>>
    where
        R: AsyncRead + AsyncSeek + Unpin + Send,
    {
        self.filter();
        let matchers = self
            .terms
            .iter()
            .map(|term| Contains::new(term))
            .collect::<Vec<Contains>>();
        inverted_index_search(index_reader, &matchers, self.meta).await
    }

    fn filter(&mut self) {
        self.terms.retain(|term| term.len() <= self.meta.max_len);
    }
}

pub struct PrefixSearch<'b> {
    terms: Vec<String>,
    meta: &'b ColumnIndexMeta,
}

impl<'b> PrefixSearch<'b> {
    pub fn new(terms: &[String], meta: &'b ColumnIndexMeta) -> Self {
        PrefixSearch {
            terms: terms.to_owned(),
            meta,
        }
    }

    pub async fn search<R>(&mut self, index_reader: &mut IndexReader<R>) -> Result<BitVec<u8>>
    where
        R: AsyncRead + AsyncSeek + Unpin + Send,
    {
        self.filter();
        let matchers = self
            .terms
            .iter()
            .map(|term| Str::new(term).starts_with())
            .collect::<Vec<StartsWith<Str>>>();
        inverted_index_search(index_reader, &matchers, self.meta).await
    }

    fn filter(&mut self) {
        self.terms.retain(|term| {
            // Check if the term is within the min and max length
            term.len() <= self.meta.max_len
                && term.len() >= self.meta.min_len
                // Check if the term is within the min and max values
                && self.meta.min_val.as_slice() <= term.as_bytes()
                && term.as_bytes() <= self.meta.max_val.as_slice()
        });
    }
}

pub struct ExactSearch<'b> {
    terms: Vec<String>,
    meta: &'b ColumnIndexMeta,
}

impl<'b> ExactSearch<'b> {
    pub fn new(terms: &[String], meta: &'b ColumnIndexMeta) -> Self {
        ExactSearch {
            terms: terms.to_owned(),
            meta,
        }
    }

    pub async fn search<R>(&mut self, index_reader: &mut IndexReader<R>) -> Result<BitVec<u8>>
    where
        R: AsyncRead + AsyncSeek + Unpin + Send,
    {
        self.filter();
        let matchers = self
            .terms
            .iter()
            .map(|term| Str::new(term))
            .collect::<Vec<Str>>();
        inverted_index_search(index_reader, &matchers, self.meta).await
    }

    fn filter(&mut self) {
        self.terms.retain(|term| {
            // Check if the term is within the min and max length
            term.len() <= self.meta.max_len
                && term.len() >= self.meta.min_len
                // Check if the term is within the min and max values
                && self.meta.min_val.as_slice() <= term.as_bytes()
                && term.as_bytes() <= self.meta.max_val.as_slice()
        });
    }
}

pub async fn inverted_index_search<A, R>(
    index_reader: &mut IndexReader<R>,
    matchers: &[A],
    column_index_meta: &ColumnIndexMeta,
) -> Result<BitVec<u8>>
where
    R: AsyncRead + AsyncSeek + Unpin + Send,
    A: Automaton,
{
    let fst_offset = column_index_meta.base_offset + column_index_meta.relative_fst_offset as u64;
    let fst_size = column_index_meta.fst_size;
    let fst_map = index_reader.fst(fst_offset, fst_size).await?;
    let mut res = BitVec::<u8>::new();

    for matcher in matchers {
        // Stream for matched keys and their bitmap offsets
        let mut stream = fst_map.search(matcher).into_stream();
        // We do not care about the key at this point, only the offset
        while let Some((_, value)) = stream.next() {
            let bitmap = index_reader.get_bitmap(column_index_meta, value).await?;

            // Resize if the res map is smaller than the bitmap
            if res.len() < bitmap.len() {
                res.resize(bitmap.len(), false);
            }
            // bitwise OR to combine the bitmaps of all the terms
            res |= bitmap;
        }
    }
    Ok(res)
}
