use std::borrow::Cow;
use std::iter::Enumerate;
use std::str::Chars;

#[derive(Debug)]
pub enum ParseError {
    QuoteNotClosed(usize, String),
}

struct CmdSplitter<'a> {
    it: Enumerate<Chars<'a>>,
}

#[cfg(unix)]
const ESCAPE_CHAR: char = '\\';

#[cfg(windows)]
const ESCAPE_CHAR: char = '`';

impl<'a> Iterator for CmdSplitter<'a> {
    type Item = Result<(usize, Cow<'a, str>), ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (mut pos, mut ch) = self.it.next()?;

        while ch.is_whitespace() {
            let (n_pos, n_ch) = match self.it.next() {
                Some(v) => v,
                None => return None,
            };
            pos = n_pos;
            ch = n_ch;
        }

        let start_pos = pos;
        let mut buf = String::with_capacity(20);

        // inv ch is word char
        while !ch.is_whitespace() {
            if ch == '"' || ch == '\'' {
                let quote_ch = ch;
                loop {
                    let (_, n_ch) = match self.it.next() {
                        Some(v) => v,
                        None => return Some(Err(ParseError::QuoteNotClosed(start_pos, buf))),
                    };
                    ch = n_ch;
                    if ch == quote_ch {
                        break;
                    }
                    buf.push(ch)
                }
                match self.it.next() {
                    Some((_, n_ch)) => ch = n_ch,
                    None => return Some(Ok((start_pos, Cow::Owned(buf)))),
                }
            } else if ch == ESCAPE_CHAR {
                match self.it.next() {
                    Some((_, quoted_char)) => buf.push(quoted_char),
                    None => {
                        buf.push(ch);
                        return Some(Ok((start_pos, Cow::Owned(buf))));
                    }
                }
                match self.it.next() {
                    Some((_, n_ch)) => ch = n_ch,
                    None => return Some(Ok((start_pos, Cow::Owned(buf)))),
                }
            } else {
                buf.push(ch);
                let (_, n_ch) = match self.it.next() {
                    Some(v) => v,
                    None => return Some(Ok((start_pos, Cow::Owned(buf)))),
                };
                ch = n_ch;
            }
        }
        Some(Ok((start_pos, Cow::Owned(buf))))
    }
}

pub fn parse_line(line: &str) -> impl Iterator<Item = Result<(usize, Cow<str>), ParseError>> {
    CmdSplitter {
        it: line.chars().enumerate(),
    }
}

#[cfg(test)]
mod test {
    use crate::interactive::cmdparse::parse_line;

    fn parse_to_vec(line: &str) -> Vec<(usize, String)> {
        parse_line(line)
            .map(|v| v.map(|(p, c)| (p, c.to_string())))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn test_1() {
        assert_eq!(
            vec![(0usize, "account".to_string()), (8, "info".to_string())],
            parse_to_vec("account info")
        );
        assert_eq!(
            vec![(0usize, "account".to_string()), (8, "unlock".to_string())],
            parse_to_vec("account unlock   ")
        );

        assert_eq!(
            vec![
                (0usize, "golemcli".to_string()),
                (9, "tasks".to_string()),
                (15, "Create".to_string()),
                (22, "/tmp/aala.json".into())
            ],
            parse_to_vec("golemcli tasks Create /tmp/aala.json")
        );

        assert_eq!(
            vec![
                (0usize, "golemcli".to_string()),
                (9, "tasks".to_string()),
                (15, "Create".to_string()),
                (22, "ala ma smok-a".into()),
                (38, "--test".into())
            ],
            parse_to_vec("golemcli tasks Create 'ala ma smok-a' --test")
        );

        assert_eq!(
            vec![
                (0, "tasks".into()),
                (6, "create ".into()),
                (15, "ala ma smok-a".into()),
                (31, "--test".into())
            ],
            parse_to_vec("tasks create\\  \"ala ma smok-a\" --t\\est")
        );
    }
}
