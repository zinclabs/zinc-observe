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

use std::str::CharIndices;

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// Tokenize the text by splitting on whitespaces and punctuation.
#[derive(Clone, Default)]
pub struct O2Tokenizer {
    token: Token,
}

/// TokenStream produced by the `O2Tokenizer`.
pub struct O2TokenStream<'a> {
    text: &'a str,
    chars: CharIndices<'a>,
    token: &'a mut Token,
    buffer: Vec<&'a str>,
    buffer_offset: usize,
}

impl Tokenizer for O2Tokenizer {
    type TokenStream<'a> = O2TokenStream<'a>;
    fn token_stream<'a>(&'a mut self, text: &'a str) -> O2TokenStream<'a> {
        self.token.reset();
        O2TokenStream {
            text,
            chars: text.char_indices(),
            token: &mut self.token,
            buffer: Vec::new(),
            buffer_offset: 0,
        }
    }
}

impl O2TokenStream<'_> {
    // search for the end of the current token.
    fn search_token_end(&mut self) -> (usize, char) {
        (self.chars)
            .find(|(_, c)| !c.is_ascii_alphanumeric())
            .unwrap_or((self.text.len(), ' '))
    }

    // get the next token from the buffer
    fn get_buffer_token(&mut self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }
        let token = self.buffer.remove(0);
        self.token.offset_from = self.buffer_offset;
        self.token.offset_to = self.buffer_offset + token.len();
        self.token.text.push_str(token);
        self.buffer_offset += token.len();
        true
    }
}

impl TokenStream for O2TokenStream<'_> {
    fn advance(&mut self) -> bool {
        self.token.text.clear();
        self.token.position = self.token.position.wrapping_add(1);
        if self.get_buffer_token() {
            return true;
        }
        while let Some((offset_from, c)) = self.chars.next() {
            if c.is_alphanumeric() {
                let (offset_to, stop_c) = self.search_token_end();
                if self.text[offset_from..offset_to]
                    .chars()
                    .any(char::is_uppercase)
                {
                    // handle camel case
                    self.buffer
                        .extend(split_camel_case(&self.text[offset_from..offset_to]));
                    self.buffer_offset = offset_from;
                    if !stop_c.is_ascii() {
                        self.buffer
                            .push(&self.text[offset_to..offset_to + stop_c.len_utf8()]);
                    }
                    if self.get_buffer_token() {
                        return true;
                    }
                } else if !stop_c.is_ascii() {
                    self.buffer
                        .push(&self.text[offset_to..offset_to + stop_c.len_utf8()]);
                    self.buffer_offset = offset_to;
                }
                self.token.offset_from = offset_from;
                self.token.offset_to = offset_to;
                self.token.text.push_str(&self.text[offset_from..offset_to]);
                return true;
            }
        }
        false
    }

    fn token(&self) -> &Token {
        self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token
    }
}

fn split_camel_case(input: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start = 0;
    let mut prev_was_upper = false;
    for (i, c) in input.char_indices() {
        if c.is_uppercase() {
            if i != 0
                && (!prev_was_upper
                    || (i + 1 < input.len()
                        && input[i + 1..]
                            .chars()
                            .next()
                            .map(|c| c.is_lowercase())
                            .unwrap_or(false)))
            {
                words.push(&input[start..i]);
                start = i;
            }
            prev_was_upper = true;
        } else {
            prev_was_upper = false;
        }
    }

    words.push(&input[start..]);
    words
}

#[cfg(test)]
mod tests {
    use tantivy::tokenizer::{TextAnalyzer, Token};

    use super::*;

    /// This is a function that can be used in tests and doc tests
    /// to assert a token's correctness.
    fn assert_token(token: &Token, position: usize, text: &str, from: usize, to: usize) {
        assert_eq!(
            token.position, position,
            "expected position {position} but {token:?}"
        );
        assert_eq!(token.text, text, "expected text {text} but {token:?}");
        assert_eq!(
            token.offset_from, from,
            "expected offset_from {from} but {token:?}"
        );
        assert_eq!(token.offset_to, to, "expected offset_to {to} but {token:?}");
    }

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let mut a = TextAnalyzer::from(O2Tokenizer::default());
        let mut token_stream = a.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);
        tokens
    }

    #[test]
    fn test_o2_tokenizer() {
        let tokens = token_stream_helper(
            "Hello, happy tax payer!민족어대사전中文的ErrorExceptionAndHTTPResponse",
        );
        assert_eq!(tokens.len(), 18);
        assert_token(&tokens[0], 0, "Hello", 0, 5);
        assert_token(&tokens[1], 1, "happy", 7, 12);
        assert_token(&tokens[2], 2, "tax", 13, 16);
        assert_token(&tokens[3], 3, "payer", 17, 22);
        assert_token(&tokens[4], 4, "민", 23, 26);
        assert_token(&tokens[5], 5, "족", 26, 29);
        assert_token(&tokens[6], 6, "어", 29, 32);
        assert_token(&tokens[7], 7, "대", 32, 35);
        assert_token(&tokens[8], 8, "사", 35, 38);
        assert_token(&tokens[9], 9, "전", 38, 41);
        assert_token(&tokens[10], 10, "中", 41, 44);
        assert_token(&tokens[11], 11, "文", 44, 47);
        assert_token(&tokens[12], 12, "的", 47, 50);
        assert_token(&tokens[13], 13, "Error", 50, 55);
        assert_token(&tokens[14], 14, "Exception", 55, 64);
        assert_token(&tokens[15], 15, "And", 64, 67);
        assert_token(&tokens[16], 16, "HTTP", 67, 71);
        assert_token(&tokens[17], 17, "Response", 71, 79);
    }
}
