/// Expands string with groups.
///
/// Example:
/// "(a.|)foo.(bar|baz.(qux|wux)|sar)(.b|.c)"
///
/// Which you can imagine as a graph:
///
///               --------""---------
///               |                  |
///               a.                 ""
///               |                  |
///               --------------------
///                        |
///                 ------foo.------
///                 |      |       |
///                bar    baz.    sar
///                 |    __|__     |
///                 |   |     |    |
///                 |  qux   wux   |
///                 |   |     |    |
///                 ----------------
///                      __|__
///                     |     |
///                    .b    .c
///
///
/// which then will be collected into the following vector:
/// ```
/// [
///     "a.foo.bar.b",
///     "a.foo.bar.c",
///     "a.foo.baz.qux.b",
///     "a.foo.baz.qux.c",
///     "a.foo.baz.wux.b",
///     "a.foo.baz.wux.c",
///     "a.foo.sar.b",
///     "a.foo.sar.c",
///     "foo.bar.b",
///     "foo.bar.c",
///     "foo.baz.qux.b",
///     "foo.baz.qux.c",
///     "foo.baz.wux.b",
///     "foo.baz.wux.c",
///     "foo.sar.b",
///     "foo.sar.c"
/// ]
/// ```
///
// Cap the cross-product expansion. Legitimate legacy patterns expand to a
// handful of strings (the largest in real configs is ~16 — see the doctest).
// 1024 leaves several orders of magnitude of headroom while preventing a
// pathological pattern like `(a|b)` repeated many times from blowing memory.
const MAX_EXPANSIONS: usize = 1024;

/// # Examples
///
/// ```
/// use crate::expand_string::expand_string;
/// assert_eq!(
///     expand_string("(a.|)foo.(bar|baz.(qux|wux)|sar)(.b|.c)").unwrap(),
///     vec![
///         "a.foo.bar.b",
///         "a.foo.bar.c",
///         "a.foo.baz.qux.b",
///         "a.foo.baz.qux.c",
///         "a.foo.baz.wux.b",
///         "a.foo.baz.wux.c",
///         "a.foo.sar.b",
///         "a.foo.sar.c",
///         "foo.bar.b",
///         "foo.bar.c",
///         "foo.baz.qux.b",
///         "foo.baz.qux.c",
///         "foo.baz.wux.b",
///         "foo.baz.wux.c",
///         "foo.sar.b",
///         "foo.sar.c"
///     ]
/// );
/// ```
pub fn expand_string(input: &str) -> Result<Vec<String>, String> {
    let mut expander: Expander = Expander::new();
    let mut accumulator: String = String::new();
    let mut just_closed: bool = false;
    for char in input.chars() {
        match char {
            '(' => {
                just_closed = false;
                expander.push(accumulator.clone());
                accumulator.clear();
                expander.open_group();
            }

            ')' => {
                just_closed = true;
                expander.push(accumulator.clone());
                accumulator.clear();
                expander.close_group()?;
            }
            '|' => {
                if !just_closed {
                    expander.push(accumulator.clone());
                    accumulator.clear();
                }
                just_closed = false;
            }
            _ => {
                just_closed = false;
                accumulator.push(char);
            }
        }
    }
    if !accumulator.is_empty() {
        expander.push(accumulator);
    }
    expander
        .stack
        .pop()
        .ok_or_else(|| "expansion produced empty result".to_string())
}

#[derive(Debug)]
struct Expander {
    stack: Vec<Vec<String>>,
    group: Option<Vec<String>>,
}

impl Expander {
    fn new() -> Expander {
        Expander {
            stack: vec![vec![String::new()]],
            group: None,
        }
    }

    fn push(&mut self, item: String) {
        if let Some(group) = &mut self.group {
            group.push(item);
        } else if let Some(group) = self.stack.last_mut() {
            for group_item in group.iter_mut() {
                group_item.push_str(item.as_str());
            }
        }
    }

    fn close_group(&mut self) -> Result<(), String> {
        if let (Some(group), Some(mut parent_group)) = (self.group.take(), self.stack.pop()) {
            if self.stack.is_empty() {
                let mut permutations: Vec<String> = vec![];

                for parent_item in &parent_group {
                    for group_item in &group {
                        permutations.push(format!(
                            "{}{}",
                            parent_item.as_str(),
                            group_item.as_str()
                        ));
                        if permutations.len() > MAX_EXPANSIONS {
                            return Err(format!(
                                "expansion exceeded cap of {MAX_EXPANSIONS} strings"
                            ));
                        }
                    }
                }
                self.stack.push(permutations);
                self.group = None;
            } else {
                let parent_item = parent_group.pop().unwrap();
                for group_item in &group {
                    parent_group.push(format!("{}{}", parent_item.as_str(), group_item.as_str()));
                    if parent_group.len() > MAX_EXPANSIONS {
                        return Err(format!(
                            "expansion exceeded cap of {MAX_EXPANSIONS} strings"
                        ));
                    }
                }
                self.group = Some(parent_group);
            }
        }
        Ok(())
    }

    fn open_group(&mut self) {
        if let Some(group) = &self.group {
            self.stack.push(group.clone());
        }
        self.group = Some(vec![]);
    }
}

#[cfg(test)]
mod expand_url {
    use crate::expand_string::expand_string;

    #[test]
    fn no_groups() {
        assert_eq!(expand_string("foo").unwrap(), vec!["foo"]);
    }

    #[test]
    fn one_group() {
        assert_eq!(
            expand_string("foo.(bar|baz)").unwrap(),
            vec!["foo.bar", "foo.baz"]
        );
    }

    #[test]
    fn nested_groups() {
        assert_eq!(
            expand_string("foo.(bar|baz.(qux|wux)|sar)").unwrap(),
            vec!["foo.bar", "foo.baz.qux", "foo.baz.wux", "foo.sar"]
        );
    }

    #[test]
    fn start_group() {
        assert_eq!(
            expand_string("(a.|)foo.(bar|baz.(qux|wux)|sar)(.b|.c)").unwrap(),
            vec![
                "a.foo.bar.b",
                "a.foo.bar.c",
                "a.foo.baz.qux.b",
                "a.foo.baz.qux.c",
                "a.foo.baz.wux.b",
                "a.foo.baz.wux.c",
                "a.foo.sar.b",
                "a.foo.sar.c",
                "foo.bar.b",
                "foo.bar.c",
                "foo.baz.qux.b",
                "foo.baz.qux.c",
                "foo.baz.wux.b",
                "foo.baz.wux.c",
                "foo.sar.b",
                "foo.sar.c"
            ]
        );
    }

    #[test]
    fn empty_node() {
        assert_eq!(
            expand_string("foo(|.bar|.baz)").unwrap(),
            vec!["foo", "foo.bar", "foo.baz"]
        );
    }

    // 11 nested 2-way alternations → 2^11 = 2048 strings, well past the cap.
    // expand_string runs on user-supplied legacy config values, so a typo or a
    // copy-pasted ClearURLs-style regex must not blow memory on migration.
    #[test]
    fn caps_pathological_expansion() {
        let pattern = "(a|b)".repeat(11);
        let result = expand_string(&pattern);
        assert!(
            result.is_err(),
            "exponential expansion must be rejected, got Ok with {} entries",
            result.map(|v| v.len()).unwrap_or(0)
        );
    }
}
