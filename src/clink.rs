use crate::expand_string::expand_string;
use crate::mode::Mode;
use crate::ClinkConfig;
use chrono::prelude::*;
use linkify::{LinkFinder, LinkKind};
use rand::Rng;
use std::collections::HashMap;
use std::rc::Rc;
use url::form_urlencoded::Parse;
use url::Url;

pub struct Clink {
    config: ClinkConfig,
    exit_map: HashMap<Rc<str>, Rc<[Rc<str>]>>,
    finder: LinkFinder,
}

impl Clink {
    pub fn new(config: ClinkConfig) -> Self {
        let exit_map = build_exit_map(&config.exit);
        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);

        if config.verbose {
            println!("Exit map: {exit_map:#?}")
        }

        Clink {
            config,
            exit_map,
            finder,
        }
    }

    pub fn find_and_replace(&self, str: &str) -> String {
        let mut res = str.to_string();
        for link in self.finder.links(str) {
            let mut l = Url::parse(self.unwrap_exit_params(link.as_str()).as_str())
                .expect("url to be parsable");
            let query = self.process_query(
                l.query_pairs(),
                l.domain().map(|d| d.strip_prefix("www.").unwrap_or(d)),
            );
            l.set_query(None);
            if !query.is_empty() {
                let mut query_pairs = l.query_pairs_mut();
                for (key, value) in query {
                    query_pairs.append_pair(key.as_str(), value.as_str());
                }
            }
            res = res.replace(link.as_str(), l.as_str());
        }
        res
    }

    fn process_query(&self, query: Parse<'_>, domain: Option<&str>) -> Vec<(String, String)> {
        match self.config.mode {
            Mode::Remove => self.filter(query, domain),
            Mode::Replace => self.replace(query, domain),
            Mode::YourMom => {
                let date = Utc::now();
                if date.month() == 5 && date.day() == 9 {
                    self.filter(query, domain)
                } else {
                    let mut tmp = self.filter(query, domain);
                    tmp.push(("utm_source".to_string(), "your_mom".to_string()));
                    tmp
                }
            }
            Mode::Evil => {
                let mut rng = rand::thread_rng();
                query
                    .map(|(key, value)| {
                        if self.config.params.contains(&key.to_string()) {
                            (
                                key.to_string(),
                                swap_two_chars(
                                    &value,
                                    rng.gen_range(0..value.to_string().len()),
                                    rng.gen_range(0..value.to_string().len()),
                                ),
                            )
                        } else {
                            (key.to_string(), value.to_string())
                        }
                    })
                    .collect()
            }
        }
    }

    fn filter(&self, query: Parse<'_>, domain: Option<&str>) -> Vec<(String, String)> {
        query
            .filter(|(key, _)| {
                !self.config.params.contains(&key.to_string())
                    && if let Some(domain) = domain {
                        return !self.config.params.contains(&format!("{domain}``{key}"));
                    } else {
                        true
                    }
            })
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    fn replace(&self, query: Parse<'_>, domain: Option<&str>) -> Vec<(String, String)> {
        query
            .map(|(key, value)| {
                if self.config.params.contains(&key.to_string()) {
                    (key.to_string(), self.config.replace_to.clone())
                } else if let Some(domain) = domain {
                    if self.config.params.contains(&format!("{domain}``{key}")) {
                        (key.to_string(), self.config.replace_to.clone())
                    } else {
                        (key.to_string(), value.to_string())
                    }
                } else {
                    (key.to_string(), value.to_string())
                }
            })
            .collect()
    }

    fn unwrap_exit_params(&self, url: &str) -> String {
        let l = Url::parse(url).unwrap();
        let domain = l.domain().unwrap_or("");
        let path = join_url(domain, l.path());
        if let Some(params) = self.exit_map.get(&path) {
            let exit = l
                .query_pairs()
                .filter(|p| params.contains(&p.0.clone().into()))
                .map(|p| p.1.to_string())
                .take(1)
                .collect::<String>();
            if !exit.is_empty() {
                return exit;
            }
        }
        url.to_string()
    }
}

fn swap_two_chars(s: &str, a: usize, b: usize) -> String {
    let mut char_vector: Vec<char> = s.chars().collect();
    char_vector.swap(a, b);
    char_vector.iter().collect()
}

fn join_url(domain: &str, path: &str) -> Rc<str> {
    format!("{}{}", domain, path).into()
}

fn build_exit_map(input: &[Vec<Rc<str>>]) -> HashMap<Rc<str>, Rc<[Rc<str>]>> {
    let mut map: HashMap<Rc<str>, Rc<[Rc<str>]>> = HashMap::new();
    for row in input.iter() {
        let expanded = expand_string(&row[0]);
        for url in expanded.into_iter() {
            map.insert(url.into(), row[1..].to_vec().into());
        }
    }
    map
}

#[cfg(test)]
mod find_and_replace {

    use std::collections::HashSet;

    use super::*;

    #[test]
    fn naive_default() {
        let clink = Clink::new(ClinkConfig::default());

        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
            ),
            "https://test.test/"
        );
    }

    #[test]
    fn naive_your_mom() {
        let clink = Clink::new(ClinkConfig::new(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
            ),
            "https://test.test/?utm_source=your_mom"
        );
    }
    #[test]
    fn naive_evil() {
        let clink = Clink::new(ClinkConfig::new(Mode::YourMom));

        assert_ne!(
            clink.find_and_replace(
                "https://test.test/?fbclid=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_source=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_campaign=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_medium=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs",
            ),
            "https://test.test/?fbclid=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_source=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_campaign=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_medium=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs"
        );
    }
    #[test]
    fn should_preserve_query() {
        let clink = Clink::new(ClinkConfig::default());
        assert_eq!(
            clink.find_and_replace("https://test.test/?abc=abc",),
            "https://test.test/?abc=abc"
        );
        let clink = Clink::new(ClinkConfig::new(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace("https://test.test/?abc=abc",),
            "https://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn multiple_params() {
        let clink = Clink::new(ClinkConfig::default());
        assert_eq!(
            clink.find_and_replace("https://test.test/?abc=abc&fbclid=flksj",),
            "https://test.test/?abc=abc"
        );
        let clink = Clink::new(ClinkConfig::new(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace("https://test.test/?abc=abc&fbclid=flksj",),
            "https://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn multiple_links() {
        let clink = Clink::new(ClinkConfig::default());
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj",
            ),
            "https://test.test/?abc=abc\nhttps://test.test/?abc=abc"
        );
        let clink = Clink::new(ClinkConfig::new(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj",
            ),
            "https://test.test/?abc=abc&utm_source=your_mom\nhttps://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn multiple_links_and_text() {
        let clink = Clink::new(ClinkConfig::default());
        assert_eq!(
            clink.find_and_replace(
                "some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj",
            ),
            "some text here https://test.test/?abc=abc here \nand herehttps://test.test/?abc=abc"
        );
        let clink = Clink::new(ClinkConfig::new(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace(
                "some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj",
            ),
            "some text here https://test.test/?abc=abc&utm_source=your_mom here \nand herehttps://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn replace() {
        let clink = Clink::new(ClinkConfig::new(Mode::Replace));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
            ),
            "https://test.test/?fbclid=clink&utm_source=clink&utm_campaign=clink&utm_medium=clink"
        );
    }

    #[test]
    fn custom_params() {
        let clink = Clink::new(ClinkConfig {
            mode: Mode::Replace,
            replace_to: "clink".to_string(),
            sleep_duration: 150,
            params: HashSet::from(["foo".into()]),
            exit: vec![],
            verbose: false,
        });
        assert_eq!(
            clink.find_and_replace("https://test.test/?foo=dsadsa",),
            "https://test.test/?foo=clink"
        );
    }

    #[test]
    fn youtube_sanitize() {
        let clink = Clink::new(ClinkConfig::default());

        assert_eq!(
            clink.find_and_replace("https://youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH",),
            "https://youtu.be/dQw4w9WgXcQ"
        );

        assert_eq!(
            clink.find_and_replace("https://www.youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH",),
            "https://www.youtu.be/dQw4w9WgXcQ"
        );

        assert_eq!(
            clink.find_and_replace("https://youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69",),
            "https://youtu.be/dQw4w9WgXcQ?t=69"
        );

        assert_eq!(
            clink.find_and_replace(
                "https://youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69&fbclid=clid",
            ),
            "https://youtu.be/dQw4w9WgXcQ?t=69"
        );

        assert_eq!(
            clink.find_and_replace("https://test.test/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69",),
            "https://test.test/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69"
        );

        let clink = Clink::new(ClinkConfig::new(Mode::Replace));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa&si=qweasd",
            ),
            "https://test.test/?fbclid=clink&utm_source=clink&utm_campaign=clink&utm_medium=clink&si=qweasd"
        );

        assert_eq!(
            clink.find_and_replace(
                "https://youtu.be/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa&si=qweasd",
            ),
            "https://youtu.be/?fbclid=clink&utm_source=clink&utm_campaign=clink&utm_medium=clink&si=clink"
        );

        let clink = Clink::new(ClinkConfig::new(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace("https://test.test/?si=dsadsa",),
            "https://test.test/?si=dsadsa&utm_source=your_mom"
        );

        assert_eq!(
            clink.find_and_replace("https://youtu.be/?si=dsadsa",),
            "https://youtu.be/?utm_source=your_mom"
        );
    }
}

#[cfg(test)]
mod unwrap_exit_params {
    use crate::{clink::Clink, ClinkConfig};

    #[test]
    fn has_exit_url() {
        let clink = Clink::new(ClinkConfig::default());
        assert_eq!(
            clink.unwrap_exit_params(
                "https://exit.sc/?url=https%3A%2F%2Fopen.spotify.com%2Fartist%2F3tEV3J5gW5BDMrJqE3NaBy%3Fsi%3D1mLk6MZSRGuol8rgwCe_Cg"
            ),
            "https://open.spotify.com/artist/3tEV3J5gW5BDMrJqE3NaBy?si=1mLk6MZSRGuol8rgwCe_Cg"
        );

        assert_eq!(
        clink.unwrap_exit_params(
            "https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjMuu2zrreBAxUt2gIHHaDVC_gQyCl6BAgqEAM&url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DdQw4w9WgXcQ&usg=AOvVaw0aHtehaphMhOCAkCydRLZU&opi=89978449"
        ),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    )
    }

    #[test]
    fn has_no_exit_url() {
        let clink = Clink::new(ClinkConfig::default());
        assert_eq!(
            clink.unwrap_exit_params(
                "https://open.spotify.com/artist/3tEV3J5gW5BDMrJqE3NaBy?si=1mLk6MZSRGuol8rgwCe_Cg"
            ),
            "https://open.spotify.com/artist/3tEV3J5gW5BDMrJqE3NaBy?si=1mLk6MZSRGuol8rgwCe_Cg"
        );
    }

    #[test]
    fn has_exit_url_but_no_exit_param() {
        let clink = Clink::new(ClinkConfig::default());
        assert_eq!(
            clink.unwrap_exit_params("https://exit.sc/?foo=bar"),
            "https://exit.sc/?foo=bar"
        );
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
