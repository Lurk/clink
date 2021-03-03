use crate::mode::Mode;
use crate::params::is_hit;
use crate::ClinkConfig;
use chrono::prelude::*;
use linkify::{LinkFinder, LinkKind};
use rand::Rng;
use std::{collections::HashMap, path::PathBuf};
use url::Url;

#[cfg(test)]
mod find_and_replace {
    use super::*;
    use crate::params::{create_index, get_default_params};

    #[test]
    fn naive() {
        assert_eq!(
            find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
                &ClinkConfig {
                    mode: Mode::Remove,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params(),
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/"
        );
        assert_eq!(
            find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
                &ClinkConfig {
                    mode: Mode::YourMom,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?fbclid=your_mom&utm_source=your_mom&utm_campaign=your_mom&utm_medium=your_mom"
        );
        assert_ne!(
            find_and_replace(
                "https://test.test/?fbclid=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_source=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_campaign=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_medium=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs",
                &ClinkConfig {
                    mode: Mode::Evil,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?fbclid=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_source=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_campaign=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_medium=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs"
        );
    }
    #[test]
    fn should_preserve_query() {
        assert_eq!(
            find_and_replace(
                "https://test.test/?abc=abc",
                &ClinkConfig {
                    mode: Mode::Remove,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                "https://test.test/?abc=abc",
                &ClinkConfig {
                    mode: Mode::YourMom,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?abc=abc"
        );
    }
    #[test]
    fn multiple_params() {
        assert_eq!(
            find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj",
                &ClinkConfig {
                    mode: Mode::Remove,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj",
                &ClinkConfig {
                    mode: Mode::YourMom,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?abc=abc&fbclid=your_mom"
        );
    }
    #[test]
    fn multiple_links() {
        assert_eq!(
            find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj",
                &ClinkConfig {
                    mode: Mode::Remove,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?abc=abc\nhttps://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj",
                &ClinkConfig {
                    mode: Mode::YourMom,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?abc=abc&fbclid=your_mom\nhttps://test.test/?abc=abc&fbclid=your_mom"
        );
    }
    #[test]
    fn multiple_links_and_text() {
        assert_eq!(
            find_and_replace(
                "some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj",
                & ClinkConfig {
                    mode: Mode::Remove,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "some text here https://test.test/?abc=abc here \nand herehttps://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                "some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj",
                &ClinkConfig {
                    mode: Mode::YourMom,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "some text here https://test.test/?abc=abc&fbclid=your_mom here \nand herehttps://test.test/?abc=abc&fbclid=your_mom"
        );
    }
    #[test]
    fn custom_text_for_your_mom_mode() {
        assert_eq!(
            find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
                &ClinkConfig {
                    mode: Mode::YourMom,
                    your_mom: "foo".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: get_default_params()
                },
                &create_index(&get_default_params())
            ),
            "https://test.test/?fbclid=foo&utm_source=foo&utm_campaign=foo&utm_medium=foo"
        );
    }

    #[test]
    fn custom_params() {
        assert_eq!(
            find_and_replace(
                "https://test.test/?foo=dsadsa",
                &ClinkConfig {
                    mode: Mode::YourMom,
                    your_mom: "your_mom".to_string(),
                    except_mothers_day: false,
                    sleep_duration: 150,
                    params: vec!["foo".to_string()]
                },
                &create_index(&vec!["foo".to_string()])
            ),
            "https://test.test/?foo=your_mom"
        );
    }
}

pub fn find_and_replace(str: &str, config: &ClinkConfig, index: &HashMap<String, bool>) -> String {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    let mut res = str.to_string();
    for link in finder.links(str) {
        let l = Url::parse(link.as_str()).unwrap();

        let query: Vec<(_, _)> = process_query(l.query_pairs(), config, index);

        let mut l2 = l.clone();
        l2.set_query(None);

        for pair in query {
            l2.query_pairs_mut()
                .append_pair(&pair.0.to_string()[..], &pair.1.to_string()[..]);
        }

        res = res.replace(link.as_str(), l2.as_str());
    }

    res
}

fn process_query(
    query: url::form_urlencoded::Parse<'_>,
    config: &ClinkConfig,
    index: &HashMap<String, bool>,
) -> Vec<(String, String)> {
    match config.mode {
        Mode::Remove => filter(query, index),
        Mode::YourMom => {
            if config.except_mothers_day {
                let date = Utc::today();
                if date.month() == 5 && date.day() == 9 {
                    filter(query, index)
                } else {
                    your_mom(query, index, config)
                }
            } else {
                your_mom(query, index, config)
            }
        }
        Mode::Evil => {
            let mut rng = rand::thread_rng();
            query
                .map(|p| {
                    if is_hit(&p.0, index) {
                        (
                            p.0.to_string(),
                            swap_two_chars(
                                &p.1,
                                rng.gen_range(0..p.1.to_string().len()),
                                rng.gen_range(0..p.1.to_string().len()),
                            ),
                        )
                    } else {
                        (p.0.to_string(), p.1.to_string())
                    }
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod swap {
    use super::*;

    #[test]
    fn test_all() {
        assert_eq!(swap_two_chars("0123456789", 2, 7), "0173456289");
    }
}

fn swap_two_chars(s: &str, a: usize, b: usize) -> String {
    let mut char_vector: Vec<char> = s.chars().collect();
    char_vector.swap(a, b);
    char_vector.iter().collect()
}

fn filter(
    query: url::form_urlencoded::Parse<'_>,
    index: &HashMap<String, bool>,
) -> Vec<(String, String)> {
    query
        .filter(|p| !is_hit(&p.0, index))
        .map(|p| (p.0.to_string(), p.1.to_string()))
        .collect()
}

fn your_mom(
    query: url::form_urlencoded::Parse<'_>,
    index: &HashMap<String, bool>,
    config: &ClinkConfig,
) -> Vec<(String, String)> {
    query
        .map(|p| {
            if is_hit(&p.0, &index) {
                (p.0.to_string(), config.your_mom.clone())
            } else {
                (p.0.to_string(), p.1.to_string())
            }
        })
        .collect()
}

pub fn fallback_config_path(path: Option<PathBuf>) -> PathBuf {
    let p = match path {
        Some(p) => p.join("clink"),
        None => std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf(),
    };

    p.join("config.toml")
}
