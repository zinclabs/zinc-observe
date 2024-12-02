// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

mod o2_tokenizer;

pub use o2_tokenizer::O2Tokenizer;
use tantivy::tokenizer::{TextAnalyzer, Token};

pub const O2_TOKENIZER: &str = "o2";

pub fn o2_tokenizer_build() -> TextAnalyzer {
    tantivy::tokenizer::TextAnalyzer::builder(O2Tokenizer::default())
        .filter(tantivy::tokenizer::RemoveLongFilter::limit(40))
        .filter(tantivy::tokenizer::LowerCaser)
        .build()
}

pub fn o2_collect_tokens(text: &str) -> Vec<String> {
    let mut a = TextAnalyzer::from(O2Tokenizer::default());
    let mut token_stream = a.token_stream(text);
    let mut tokens: Vec<String> = Vec::new();
    let mut add_token = |token: &Token| {
        tokens.push(token.text.to_lowercase());
    };
    token_stream.process(&mut add_token);
    tokens
}
