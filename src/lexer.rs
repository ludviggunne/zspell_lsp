use lsp_types::{Position, Range};
use std::str::{CharIndices, Lines};
use streaming_iterator::StreamingIterator;

pub struct Word<'a> {
    pub text: &'a str,
    pub range: Range,
}

#[derive(Clone, Copy)]
struct CharPos {
    char: char,
    position: Position,
    offset: usize,
}

struct CharPosIter<'a> {
    lines: Lines<'a>,
    current_line: &'a str,
    chars: CharIndices<'a>,
    position: Position,
}

impl<'a> CharPosIter<'a> {
    pub fn new(text: &'a str) -> Option<Self> {
        let mut lines = text.lines();
        let (current_line, chars) = match lines.next() {
            None => return None,
            Some(line) => (line, line.char_indices()),
        };

        Some(Self {
            lines,
            chars,
            current_line,
            position: Position::default(),
        })
    }
}

impl<'a> Iterator for CharPosIter<'a> {
    type Item = CharPos;

    fn next(&mut self) -> Option<Self::Item> {
        match self.chars.next() {
            Some((offset, char)) => {
                let charpos = CharPos {
                    char,
                    position: self.position,
                    offset,
                };
                self.position.character += 1;
                Some(charpos)
            }
            None => match self.lines.next() {
                None => None,
                Some(line) => {
                    self.current_line = line;
                    self.chars = line.char_indices();
                    self.position.line += 1;
                    self.position.character = 0;
                    self.next()
                }
            },
        }
    }
}

pub struct Lexer<'a> {
    iter: CharPosIter<'a>,
    current_word: Option<Word<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Option<Self> {
        let iter = CharPosIter::new(text);
        match iter {
            None => None,
            Some(iter) => Some(Self {
                iter,
                current_word: None,
            }),
        }
    }

    fn make_word_at_line(
        line: &'a str,
        begin: CharPos,
        end: CharPos,
    ) -> Word<'a> {
        Word {
            text: &line[begin.offset..end.offset],
            range: Range {
                start: begin.position,
                end: end.position,
            },
        }
    }
}

fn is_wordchar(c: char) -> bool {
    c.is_alphabetic() || c == '\''
}

impl<'a> StreamingIterator for Lexer<'a> {
    type Item = Word<'a>;

    fn get(&self) -> Option<&Self::Item> {
        match &self.current_word {
            None => None,
            Some(word) => Some(&word),
        }
    }

    fn advance(&mut self) {
        let begin = loop {
            match self.iter.next() {
                None => {
                    self.current_word = None;
                    return;
                }
                Some(charpos) => {
                    if is_wordchar(charpos.char) {
                        break charpos;
                    }
                }
            }
        };

        let current_line = self.iter.current_line;

        let mut end = 'find_end: {
            let mut tmp = match self.iter.next() {
                None => break 'find_end begin,
                Some(charpos) => {
                    if is_wordchar(charpos.char) {
                        charpos
                    } else {
                        break 'find_end begin;
                    }
                }
            };

            loop {
                match self.iter.next() {
                    None => break 'find_end tmp,
                    Some(charpos) => {
                        if !is_wordchar(charpos.char) {
                            break 'find_end tmp;
                        }
                        tmp = charpos;
                    }
                }
            }
        };

        end.position.character += 1;
        end.offset += end.char.len_utf8();

        self.current_word =
            Some(Self::make_word_at_line(current_line, begin, end));
    }
}

#[cfg(test)]
mod test {

    use super::*;

    fn case<'a>(lexer: &'a mut Lexer, expected_word: &str, line: u32) {
        let word = lexer.next().unwrap();
        assert_eq!(word.text, expected_word);
        assert_eq!(word.range.start.line, line);
    }

    #[test]
    #[rustfmt::skip]
    fn lexer() {
        let text = concat!(
            "\n",
            "This is the first line.\n",
            "This isn't the first line.\n",
            "\n"
        );

        let mut lexer = Lexer::new(text).unwrap();

        case(&mut lexer, "This", 1);
        case(&mut lexer, "is", 1);
        case(&mut lexer, "the", 1);
        case(&mut lexer, "first", 1);
        case(&mut lexer, "line", 1);
        case(&mut lexer, "This", 2);
        case(&mut lexer, "isn't", 2);
        case(&mut lexer, "the", 2);
        case(&mut lexer, "first", 2);
        case(&mut lexer, "line", 2);
        assert!(matches!(lexer.next(), None));
    }
}
