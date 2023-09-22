use crate::expand_string::expand_string;
use crate::mode::Mode;
use crate::ClinkConfig;
use chrono::prelude::*;
use linkify::{LinkFinder, LinkKind};
use rand::Rng;
use std::collections::HashMap;
use std::rc::Rc;
use url::Url;

pub struct Clink {
    config: ClinkConfig,
    index: HashMap<Rc<str>, bool>,
    exit_map: HashMap<Rc<str>, Rc<[Rc<str>]>>,
    finder: LinkFinder,
}

impl Clink {
    pub fn new(config: ClinkConfig) -> Self {
        let index = create_index(&config.params);
        let exit_map = build_exit_map(&config.exit);
        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);

        Clink {
            config,
            index,
            exit_map,
            finder,
        }
    }

    pub fn find_and_replace(&self, str: &str) -> String {
        let mut res = str.to_string();
        for link in self.finder.links(str) {
            let mut l = Url::parse(self.unwrap_exit_params(link.as_str()).as_str()).unwrap();
            let query: Vec<(_, _)> = self.process_query(l.query_pairs());
            l.set_query(None);
            for pair in query {
                l.query_pairs_mut()
                    .append_pair(&pair.0.to_string()[..], &pair.1.to_string()[..]);
            }
            res = res.replace(link.as_str(), l.as_str());
        }
        res
    }

    fn process_query(&self, query: url::form_urlencoded::Parse<'_>) -> Vec<(String, String)> {
        match self.config.mode {
            Mode::Remove => self.filter(query),
            Mode::Replace => self.replace(query),
            Mode::YourMom => {
                let date = Utc::now();
                if date.month() == 5 && date.day() == 9 {
                    self.filter(query)
                } else {
                    let mut tmp = self.filter(query);
                    tmp.push(("utm_source".to_string(), "your_mom".to_string()));
                    tmp
                }
            }
            Mode::Evil => {
                let mut rng = rand::thread_rng();
                query
                    .map(|p| {
                        if self.index.contains_key::<Rc<str>>(&p.0.clone().into()) {
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

    fn filter(&self, query: url::form_urlencoded::Parse<'_>) -> Vec<(String, String)> {
        query
            .filter(|p| !self.index.contains_key::<Rc<str>>(&p.0.clone().into()))
            .map(|p| (p.0.to_string(), p.1.to_string()))
            .collect()
    }

    fn replace(&self, query: url::form_urlencoded::Parse<'_>) -> Vec<(String, String)> {
        query
            .map(|p| {
                if self.index.contains_key::<Rc<str>>(&p.0.clone().into()) {
                    (p.0.to_string(), self.config.replace_to.clone())
                } else {
                    (p.0.to_string(), p.1.to_string())
                }
            })
            .collect()
    }

    fn unwrap_exit_params(&self, url: &str) -> String {
        let l = Url::parse(url).unwrap();
        let domain = l.domain().unwrap_or("");
        let path = join_url(domain, l.path());
        if let Some(params) = self.exit_map.get(&path) {
            return l
                .query_pairs()
                .filter(|p| params.contains(&p.0.clone().into()))
                .map(|p| p.1.to_string())
                .take(1)
                .collect::<String>();
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

fn create_index(vec: &[Rc<str>]) -> HashMap<Rc<str>, bool> {
    let mut map: HashMap<Rc<str>, bool> = HashMap::new();
    for key in vec.iter().cloned() {
        map.insert(key, true);
    }
    map
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
            params: vec!["foo".into()],
            exit: vec![],
        });
        assert_eq!(
            clink.find_and_replace("https://test.test/?foo=dsadsa",),
            "https://test.test/?foo=clink"
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
}

#[cfg(test)]
mod swap {
    use super::*;

    #[test]
    fn test_all() {
        assert_eq!(swap_two_chars("0123456789", 2, 7), "0173456289");
    }
}
