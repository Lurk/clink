use crate::config::ClinkConfig;
use crate::mode::Mode;
use crate::provider::{CompiledProvider, CompiledRules, check_provider};
use chrono::prelude::*;
use linkify::{LinkFinder, LinkKind};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use rand::RngExt;
use url::Url;

const QUERY_COMPONENT_KEY: &AsciiSet = &CONTROLS.add(b' ').add(b'#').add(b'&').add(b'=').add(b'+');
const QUERY_COMPONENT_VALUE: &AsciiSet = &CONTROLS.add(b' ').add(b'#').add(b'&').add(b'+');

pub struct CleanResult {
    pub text: String,
    pub urls_cleaned: u32,
    pub params_removed: u32,
    pub exits_unwrapped: u32,
}

pub struct Clink {
    config: ClinkConfig,
    global_rules: CompiledRules,
    scoped_providers: Vec<CompiledProvider>,
    finder: LinkFinder,
}

impl Clink {
    pub fn new(config: ClinkConfig) -> Self {
        for (name, cfg) in &config.providers {
            for warning in check_provider(name, cfg) {
                eprintln!("clink: warning: {warning}");
            }
        }

        let global_rules = config
            .providers
            .get("global")
            .map_or_else(|| CompiledRules::new(&[]), |p| CompiledRules::new(&p.rules));

        let scoped_providers: Vec<CompiledProvider> = config
            .providers
            .iter()
            .filter(|(name, _)| name.as_str() != "global")
            .filter_map(|(_, cfg)| CompiledProvider::new(cfg))
            .collect();

        let mut finder = LinkFinder::new();
        finder.kinds(&[LinkKind::Url]);

        if config.verbose {
            println!("Compiled {} scoped providers", scoped_providers.len());
        }

        Clink {
            config,
            global_rules,
            scoped_providers,
            finder,
        }
    }

    pub fn find_and_replace(&self, input: &str) -> CleanResult {
        let mut urls_cleaned: u32 = 0;
        let mut params_removed: u32 = 0;
        let mut exits_unwrapped: u32 = 0;

        // Splice each cleaned URL back at its linkify byte range rather than
        // search-and-replace on the whole string. Naive `String::replace`
        // would match the original link text wherever it appears — and a
        // shorter URL that's a textual prefix of a neighbouring URL would
        // get its replacement bleed into the longer one.
        let mut res = String::with_capacity(input.len());
        let mut last_end = 0usize;

        for link in self.finder.links(input) {
            res.push_str(&input[last_end..link.start()]);
            let Some((mut l, was_exit)) = self.parse_link(link.as_str()) else {
                // linkify is more permissive than url::Url (e.g. accepts
                // out-of-range ports). Keep the original text rather than
                // crashing the daemon on user clipboard content.
                res.push_str(link.as_str());
                last_end = link.end();
                continue;
            };
            if was_exit {
                exits_unwrapped += 1;
            }
            let normalized_original = l.to_string();
            #[allow(clippy::cast_possible_truncation)]
            let original_param_count = l.query_pairs().count() as u32;

            let matching_providers = self.find_matching_providers(l.as_str());

            let query = self.process_query(
                l.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())),
                &matching_providers,
            );
            #[allow(clippy::cast_possible_truncation)]
            let new_param_count = query.len() as u32;
            let removed = original_param_count.saturating_sub(new_param_count);
            l.set_query(None);
            if !query.is_empty() {
                let qs = query
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}={}",
                            utf8_percent_encode(k, QUERY_COMPONENT_KEY),
                            utf8_percent_encode(v, QUERY_COMPONENT_VALUE),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("&");
                l.set_query(Some(&qs));
            }
            let new_url = l.as_str();
            let query_changed = new_url != normalized_original;
            if query_changed || was_exit {
                urls_cleaned += 1;
                params_removed += removed;
                res.push_str(new_url);
            } else {
                res.push_str(link.as_str());
            }
            last_end = link.end();
        }
        res.push_str(&input[last_end..]);

        CleanResult {
            text: res,
            urls_cleaned,
            params_removed,
            exits_unwrapped,
        }
    }

    fn find_matching_providers(&self, url: &str) -> Vec<&CompiledProvider> {
        self.scoped_providers
            .iter()
            .filter(|p| p.matches_url(url))
            .collect()
    }

    fn process_query(
        &self,
        query: impl Iterator<Item = (String, String)>,
        providers: &[&CompiledProvider],
    ) -> Vec<(String, String)> {
        match self.config.mode {
            Mode::Remove => self.filter(query, providers),
            Mode::Replace => self.replace(query, providers),
            Mode::YourMom => {
                let date = Utc::now();
                if date.month() == 5 && date.day() == 9 {
                    self.filter(query, providers)
                } else {
                    let mut tmp = self.filter(query, providers);
                    tmp.push(("utm_source".to_string(), "your_mom".to_string()));
                    tmp
                }
            }
            Mode::Evil => {
                let mut rng = rand::rng();
                query
                    .map(|(key, value)| {
                        if self.is_tracked(&key, providers) {
                            // char count, not byte length — multibyte values
                            // would otherwise yield out-of-range indices into
                            // the char vector; 0/1-char values can't swap.
                            let char_count = value.chars().count();
                            if char_count < 2 {
                                (key, value)
                            } else {
                                let a = rng.random_range(0..char_count);
                                let b = rng.random_range(0..char_count);
                                (key, swap_two_chars(&value, a, b))
                            }
                        } else {
                            (key, value)
                        }
                    })
                    .collect()
            }
        }
    }

    fn is_tracked(&self, key: &str, providers: &[&CompiledProvider]) -> bool {
        self.global_rules.is_tracked(key) || providers.iter().any(|p| p.rules.is_tracked(key))
    }

    fn filter(
        &self,
        query: impl Iterator<Item = (String, String)>,
        providers: &[&CompiledProvider],
    ) -> Vec<(String, String)> {
        query
            .filter(|(key, _)| !self.is_tracked(key, providers))
            .collect()
    }

    fn replace(
        &self,
        query: impl Iterator<Item = (String, String)>,
        providers: &[&CompiledProvider],
    ) -> Vec<(String, String)> {
        query
            .map(|(key, value)| {
                if self.is_tracked(&key, providers) {
                    (key, self.config.replace_to.clone())
                } else {
                    (key, value)
                }
            })
            .collect()
    }

    fn try_unwrap_redirect(&self, url: &str) -> (String, bool) {
        for provider in &self.scoped_providers {
            if provider.matches_url(url) {
                if let Some(dest) = provider.try_redirect(url) {
                    return (dest, true);
                }
            }
        }
        (url.to_string(), false)
    }

    // Redirect captures (e.g. exit.sc's `url=...` value) aren't guaranteed to
    // be valid URLs once decoded — junk values would otherwise crash the
    // daemon. Fall back to the original linkify-found link in that case.
    //
    // Loop the unwrap so chained redirectors (e.g. exit.sc → google.com/url
    // → real URL) collapse to the innermost destination. The bound caps
    // pathological cycles (a malicious provider with a self-capturing
    // redirection regex would otherwise spin forever).
    //
    // Returns None when the original link itself isn't url::Url-parseable —
    // linkify is more permissive than url::Url, so the caller must fall back
    // to the raw clipboard text rather than panicking.
    fn parse_link(&self, link: &str) -> Option<(Url, bool)> {
        const MAX_UNWRAPS: u32 = 5;
        let mut current = link.to_string();
        let mut any_unwrap = false;
        for _ in 0..MAX_UNWRAPS {
            let (unwrapped, was_exit) = self.try_unwrap_redirect(&current);
            if !was_exit {
                break;
            }
            if Url::parse(&unwrapped).is_err() {
                // Garbage at this step — keep what we already unwrapped.
                break;
            }
            current = unwrapped;
            any_unwrap = true;
        }
        if any_unwrap {
            return Some((
                Url::parse(&current).expect("validated parseable above"),
                true,
            ));
        }
        Url::parse(link).ok().map(|u| (u, false))
    }
}

fn swap_two_chars(s: &str, a: usize, b: usize) -> String {
    let mut char_vector: Vec<char> = s.chars().collect();
    char_vector.swap(a, b);
    char_vector.iter().collect()
}

#[cfg(test)]
fn test_config(mode: Mode) -> ClinkConfig {
    let id = std::thread::current().id();
    let tmp = std::env::temp_dir().join(format!("clink_test_cfg_{id:?}.toml"));
    std::fs::write(&tmp, crate::config::DEFAULT_CONFIG_TEMPLATE).unwrap();
    let mut cfg = crate::config::load_config(&tmp).unwrap();
    cfg.mode = mode;
    let _ = std::fs::remove_file(&tmp);
    // Mirror production: run::execute always calls resolve_patterns after
    // load_config, so the builtin fallback supplies the tracking rules.
    let empty_cache_dir = std::env::temp_dir().join(format!("clink_test_cache_{id:?}"));
    let _ = std::fs::remove_dir_all(&empty_cache_dir);
    crate::remote::resolve_patterns(&mut cfg, &empty_cache_dir);
    cfg
}

#[cfg(test)]
mod find_and_replace {

    use std::collections::HashMap;

    use super::*;

    #[test]
    fn naive_default() {
        let clink = Clink::new(test_config(Mode::Remove));

        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
            ).text,
            "https://test.test/"
        );
    }

    #[test]
    fn naive_your_mom() {
        let clink = Clink::new(test_config(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
            ).text,
            "https://test.test/?utm_source=your_mom"
        );
    }
    #[test]
    fn naive_evil() {
        let clink = Clink::new(test_config(Mode::Evil));

        let input = "https://test.test/?fbclid=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&utm_source=IwAR3l6qn8TzOT254dIa7jBAM1dG3OHn3f8ZoRGsADTmqG1Zfmmko-oRhE8Qs&keep=untouched";
        let out = clink.find_and_replace(input).text;

        let parsed = Url::parse(&out).unwrap();
        let pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            vec!["fbclid", "utm_source", "keep"],
            "Evil must preserve every key in original order"
        );
        let keep = pairs.iter().find(|(k, _)| k == "keep").unwrap();
        assert_eq!(keep.1, "untouched", "untracked params must not be mangled");
        assert_ne!(out, input, "tracked param values should be mangled");
    }
    #[test]
    fn should_preserve_query() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink.find_and_replace("https://test.test/?abc=abc",).text,
            "https://test.test/?abc=abc"
        );
        let clink = Clink::new(test_config(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace("https://test.test/?abc=abc",).text,
            "https://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn multiple_params() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace("https://test.test/?abc=abc&fbclid=flksj",)
                .text,
            "https://test.test/?abc=abc"
        );
        let clink = Clink::new(test_config(Mode::YourMom));
        assert_eq!(
            clink
                .find_and_replace("https://test.test/?abc=abc&fbclid=flksj",)
                .text,
            "https://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn multiple_links() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj",
            ).text,
            "https://test.test/?abc=abc\nhttps://test.test/?abc=abc"
        );
        let clink = Clink::new(test_config(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj",
            ).text,
            "https://test.test/?abc=abc&utm_source=your_mom\nhttps://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn multiple_links_and_text() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink.find_and_replace(
                "some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj",
            ).text,
            "some text here https://test.test/?abc=abc here \nand herehttps://test.test/?abc=abc"
        );
        let clink = Clink::new(test_config(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace(
                "some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj",
            ).text,
            "some text here https://test.test/?abc=abc&utm_source=your_mom here \nand herehttps://test.test/?abc=abc&utm_source=your_mom"
        );
    }
    #[test]
    fn replace() {
        let clink = Clink::new(test_config(Mode::Replace));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa",
            ).text,
            "https://test.test/?fbclid=clink&utm_source=clink&utm_campaign=clink&utm_medium=clink"
        );
    }

    #[test]
    fn custom_params() {
        let mut providers = HashMap::new();
        providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["foo".into()],
                ..Default::default()
            },
        );
        let clink = Clink::new(ClinkConfig {
            mode: Mode::Replace,
            replace_to: "clink".to_string(),
            sleep_duration: 150,
            providers,
            verbose: false,
            remote: None,
        });
        assert_eq!(
            clink
                .find_and_replace("https://test.test/?foo=dsadsa",)
                .text,
            "https://test.test/?foo=clink"
        );
    }

    #[test]
    fn overlapping_providers_both_apply() {
        let mut providers = HashMap::new();
        providers.insert(
            "shop_a".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://([a-z0-9-]+\.)*?shop\.example".into()),
                rules: vec!["aff".into()],
                ..Default::default()
            },
        );
        providers.insert(
            "shop_b".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://shop\.example".into()),
                rules: vec!["ref".into()],
                ..Default::default()
            },
        );
        let clink = Clink::new(ClinkConfig {
            mode: Mode::Remove,
            replace_to: "clink".to_string(),
            sleep_duration: 150,
            providers,
            verbose: false,
            remote: None,
        });
        assert_eq!(
            clink
                .find_and_replace("https://shop.example/item?aff=1&ref=2&keep=ok")
                .text,
            "https://shop.example/item?keep=ok",
            "rules from all matching providers should be applied"
        );
    }

    #[test]
    fn exception_protects_url_from_stripping() {
        let mut providers = HashMap::new();
        providers.insert(
            "shop".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://shop\.example".into()),
                rules: vec!["ref".into()],
                redirections: vec![],
                exceptions: vec![r"^https?://shop\.example/admin".into()],
            },
        );
        let clink = Clink::new(ClinkConfig {
            mode: Mode::Remove,
            replace_to: "clink".to_string(),
            sleep_duration: 150,
            providers,
            verbose: false,
            remote: None,
        });
        assert_eq!(
            clink
                .find_and_replace("https://shop.example/admin?ref=keep")
                .text,
            "https://shop.example/admin?ref=keep",
            "excepted URL must not have ref stripped"
        );
        assert_eq!(
            clink
                .find_and_replace("https://shop.example/item?ref=bye")
                .text,
            "https://shop.example/item",
            "non-excepted URL must still have ref stripped"
        );
    }

    #[test]
    fn exception_blocks_redirect_unwrap() {
        let mut providers = HashMap::new();
        providers.insert(
            "exitsc".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://exit\.sc".into()),
                rules: vec![],
                redirections: vec![r"url=([^&]+)".into()],
                exceptions: vec![r"^https?://exit\.sc/admin".into()],
            },
        );
        let clink = Clink::new(ClinkConfig {
            mode: Mode::Remove,
            replace_to: "clink".to_string(),
            sleep_duration: 150,
            providers,
            verbose: false,
            remote: None,
        });
        let excepted = "https://exit.sc/admin?url=https%3A%2F%2Fexample.com";
        let result = clink.find_and_replace(excepted);
        assert_eq!(result.exits_unwrapped, 0, "exception must block unwrap");
        assert!(
            !result.text.starts_with("https://example.com"),
            "destination must not replace the excepted URL, got {}",
            result.text
        );
    }

    #[test]
    fn youtube_sanitize() {
        let clink = Clink::new(test_config(Mode::Remove));

        assert_eq!(
            clink
                .find_and_replace("https://youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH",)
                .text,
            "https://youtu.be/dQw4w9WgXcQ"
        );

        assert_eq!(
            clink
                .find_and_replace("https://www.youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH",)
                .text,
            "https://www.youtu.be/dQw4w9WgXcQ"
        );

        assert_eq!(
            clink
                .find_and_replace("https://youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69",)
                .text,
            "https://youtu.be/dQw4w9WgXcQ?t=69"
        );

        assert_eq!(
            clink
                .find_and_replace(
                    "https://youtu.be/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69&fbclid=clid",
                )
                .text,
            "https://youtu.be/dQw4w9WgXcQ?t=69"
        );

        assert_eq!(
            clink
                .find_and_replace("https://test.test/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69",)
                .text,
            "https://test.test/dQw4w9WgXcQ?si=NblIBgit-qHN7MoH&t=69"
        );

        let clink = Clink::new(test_config(Mode::Replace));
        assert_eq!(
            clink.find_and_replace(
                "https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa&si=qweasd",
            ).text,
            "https://test.test/?fbclid=clink&utm_source=clink&utm_campaign=clink&utm_medium=clink&si=qweasd"
        );

        assert_eq!(
            clink.find_and_replace(
                "https://youtu.be/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa&si=qweasd",
            ).text,
            "https://youtu.be/?fbclid=clink&utm_source=clink&utm_campaign=clink&utm_medium=clink&si=clink"
        );

        let clink = Clink::new(test_config(Mode::YourMom));
        assert_eq!(
            clink.find_and_replace("https://test.test/?si=dsadsa",).text,
            "https://test.test/?si=dsadsa&utm_source=your_mom"
        );

        assert_eq!(
            clink.find_and_replace("https://youtu.be/?si=dsadsa",).text,
            "https://youtu.be/?utm_source=your_mom"
        );
    }

    #[test]
    fn preserves_unwise_characters_in_query() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace("https://foo.foo/?param[]=1&param[]=2&fbclid=abc")
                .text,
            "https://foo.foo/?param[]=1&param[]=2"
        );
    }

    #[test]
    fn evil_empty_value_does_not_panic() {
        let clink = Clink::new(test_config(Mode::Evil));
        let out = clink
            .find_and_replace("https://test.test/?fbclid=&keep=x")
            .text;
        let parsed = Url::parse(&out).unwrap();
        let pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let fbclid = pairs
            .iter()
            .find(|(k, _)| k == "fbclid")
            .expect("fbclid must survive");
        assert_eq!(
            fbclid.1, "",
            "empty tracked value must stay empty, not panic"
        );
    }

    #[test]
    fn evil_single_char_value_does_not_panic() {
        let clink = Clink::new(test_config(Mode::Evil));
        let out = clink.find_and_replace("https://test.test/?fbclid=a").text;
        let parsed = Url::parse(&out).unwrap();
        let fbclid = parsed
            .query_pairs()
            .find(|(k, _)| k == "fbclid")
            .expect("fbclid must survive");
        assert_eq!(fbclid.1, "a", "single-char value must survive unchanged");
    }

    #[test]
    fn evil_multibyte_value_does_not_panic() {
        // Multibyte UTF-8: byte length > char count. Using byte length to index
        // into a char vector would panic.
        let clink = Clink::new(test_config(Mode::Evil));
        let input = "https://test.test/?fbclid=%F0%9F%8E%89%F0%9F%8E%8A";
        // Must not panic. We only care that the URL parses back out.
        let out = clink.find_and_replace(input).text;
        let parsed = Url::parse(&out).unwrap();
        assert!(
            parsed.query_pairs().any(|(k, _)| k == "fbclid"),
            "fbclid must still be present"
        );
    }

    #[test]
    fn clean_result_counts_urls_cleaned() {
        let clink = Clink::new(test_config(Mode::Remove));
        let result = clink.find_and_replace("https://test.test/?fbclid=abc");
        assert_eq!(result.urls_cleaned, 1);
        assert_eq!(result.text, "https://test.test/");
    }

    #[test]
    fn clean_result_counts_params_removed() {
        let clink = Clink::new(test_config(Mode::Remove));
        let result = clink
            .find_and_replace("https://test.test/?fbclid=abc&utm_source=x&utm_medium=y&keep=yes");
        assert_eq!(result.params_removed, 3);
        assert_eq!(result.text, "https://test.test/?keep=yes");
    }

    #[test]
    fn clean_result_counts_exits_unwrapped() {
        let clink = Clink::new(test_config(Mode::Remove));
        let result = clink.find_and_replace("https://exit.sc/?url=https%3A%2F%2Fexample.com");
        assert_eq!(result.exits_unwrapped, 1);
    }

    // Regex-captured redirect destinations aren't guaranteed to be valid URLs.
    // A link like `https://exit.sc/?url=not_a_url` would otherwise crash the
    // daemon — and the daemon restart loop would re-read the same clipboard.
    #[test]
    fn redirect_unwrap_unparseable_destination_does_not_panic() {
        let clink = Clink::new(test_config(Mode::Remove));
        let input = "https://exit.sc/?url=not_a_url";
        let result = clink.find_and_replace(input);
        assert_eq!(
            result.text, input,
            "unparseable redirect destination must leave the original link untouched"
        );
        assert_eq!(
            result.exits_unwrapped, 0,
            "a failed unwrap must not be counted as a successful exit"
        );
    }

    #[test]
    fn shorter_link_does_not_corrupt_neighbour_when_it_is_a_prefix() {
        // Cleaning the first URL must not bleed into the second. With a
        // naive `String::replace`, the substring `https://test.test/?fbclid=a`
        // also matches inside `https://test.test/?fbclid=ab`, leaving a
        // stray `b` stranded after the second URL gets rewritten.
        let clink = Clink::new(test_config(Mode::Remove));
        let input = "https://test.test/?fbclid=a https://test.test/?fbclid=ab";
        let result = clink.find_and_replace(input);
        assert_eq!(
            result.text, "https://test.test/ https://test.test/",
            "splice-by-range must not let one link's replacement leak into another"
        );
        assert_eq!(result.urls_cleaned, 2);
    }

    #[test]
    fn chained_redirect_unwraps_to_inner_destination() {
        // Wrap a youtube.com/redirect link inside a google.com/url redirect;
        // the daemon should peel both layers in one pass, not just the outer.
        let clink = Clink::new(test_config(Mode::Remove));
        let input = "https://www.google.com/url?q=https%3A%2F%2Fwww.youtube.com%2Fredirect%3Fq%3Dhttps%253A%252F%252Fexample.com&sa=t";
        let result = clink.find_and_replace(input);
        assert_eq!(
            result.text, "https://example.com/",
            "chained redirect should unwrap to the innermost destination"
        );
        assert_eq!(
            result.exits_unwrapped, 1,
            "a chain that resolves should still count as one cleaned URL"
        );
    }

    #[test]
    fn chained_redirect_with_unparseable_inner_keeps_outer_unwrap() {
        // First hop unwraps to a valid URL; the would-be second hop captures
        // garbage. We must keep the first-level unwrap rather than discarding
        // it — and must not panic on the bad inner.
        let clink = Clink::new(test_config(Mode::Remove));
        // exit.sc → google.com/url?q=not_a_url. The outer unwrap yields a
        // valid google URL; the inner google `q=` capture is garbage and
        // should be retained as-is at the first hop.
        let input = "https://exit.sc/?url=https%3A%2F%2Fwww.google.com%2Furl%3Fq%3Dnot_a_url";
        // Just must not panic and must produce a parseable URL in output.
        let result = clink.find_and_replace(input);
        assert!(
            url::Url::parse(&result.text).is_ok(),
            "output must remain a parseable URL, got {}",
            result.text
        );
        assert_eq!(result.exits_unwrapped, 1);
    }

    #[test]
    fn instagram_igsh_stripped() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace(
                    "https://www.instagram.com/reel/DW5mth4tEGv/?igsh=MXN0MGRuMGtyODJwNQ=="
                )
                .text,
            "https://www.instagram.com/reel/DW5mth4tEGv/"
        );
    }

    #[test]
    fn preserves_equals_in_query_values() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace("https://foo.foo/?token=abc123==&fbclid=abc")
                .text,
            "https://foo.foo/?token=abc123=="
        );
    }

    #[test]
    fn clean_result_no_changes() {
        let clink = Clink::new(test_config(Mode::Remove));
        let result = clink.find_and_replace("https://test.test/?keep=yes");
        assert_eq!(result.urls_cleaned, 0);
        assert_eq!(result.params_removed, 0);
        assert_eq!(result.exits_unwrapped, 0);
    }

    #[test]
    fn normalization_only_not_counted_as_cleaned() {
        let clink = Clink::new(test_config(Mode::Remove));
        let result = clink.find_and_replace("https://example.com");
        assert_eq!(result.urls_cleaned, 0);
        assert_eq!(result.params_removed, 0);
        assert_eq!(result.text, "https://example.com");
    }

    #[test]
    fn clean_result_multiple_urls() {
        let clink = Clink::new(test_config(Mode::Remove));
        let result = clink.find_and_replace(
            "https://test.test/?fbclid=a\nhttps://test.test/?utm_source=b&utm_medium=c",
        );
        assert_eq!(result.urls_cleaned, 2);
        assert_eq!(result.params_removed, 3);
    }

    #[test]
    fn twitter_srcset_param_not_stripped_by_unanchored_src_rule() {
        // Twitter's bundled ClearURLs rule `(?:ref_?)?src` must match
        // anchored. Without anchoring, `?srcset=…` would be stripped because
        // "src" is a substring of "srcset".
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace("https://twitter.com/user/status/1?srcset=app&keep=yes")
                .text,
            "https://twitter.com/user/status/1?srcset=app&keep=yes",
            "srcset param must not be stripped by anchored `src` rule"
        );
    }

    #[test]
    fn twitter_src_param_stripped() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace("https://twitter.com/user/status/1?src=app&keep=yes")
                .text,
            "https://twitter.com/user/status/1?keep=yes"
        );
    }

    #[test]
    fn fbclid_stripped_case_insensitively() {
        let clink = Clink::new(test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace("https://test.test/?Fbclid=abc&keep=yes")
                .text,
            "https://test.test/?keep=yes"
        );
    }

    // linkify is more permissive than url::Url — e.g. it accepts
    // `https://example.com:65536/foo` (port out of u16 range) but url::Url
    // rejects it. The daemon used to panic on these, then restart and re-read
    // the same clipboard, looping. Skip the link instead.
    #[test]
    fn unparseable_link_leaves_clipboard_untouched_no_panic() {
        let clink = Clink::new(test_config(Mode::Remove));
        let input = "before https://example.com:65536/foo?fbclid=abc after";
        let result = clink.find_and_replace(input);
        assert_eq!(
            result.text, input,
            "unparseable URL must leave the original text untouched"
        );
        assert_eq!(result.urls_cleaned, 0);
        assert_eq!(result.params_removed, 0);
    }

    #[test]
    fn unparseable_link_does_not_block_neighbour_url() {
        let clink = Clink::new(test_config(Mode::Remove));
        let input = "https://example.com:65536/foo https://test.test/?fbclid=abc";
        let result = clink.find_and_replace(input);
        assert_eq!(
            result.text, "https://example.com:65536/foo https://test.test/",
            "second URL should still be cleaned even if the first is unparseable"
        );
        assert_eq!(result.urls_cleaned, 1);
    }

    #[test]
    fn lookalike_domains_do_not_match_shipped_providers() {
        // Shipped patterns in default_config.toml must require a host-end
        // boundary; otherwise `amazon.com.attacker.com/?sp_csd=...` would
        // get its tracking param stripped, mangling the URL the user pasted.
        let clink = Clink::new(test_config(Mode::Remove));

        let amazon_lookalike = "https://amazon.com.attacker.com/?sp_csd=secret&keep=me";
        let amazon_result = clink.find_and_replace(amazon_lookalike);
        assert_eq!(
            amazon_result.params_removed, 0,
            "amazon's url_pattern must not match lookalike host"
        );

        let exit_lookalike = "https://exit.sc.attacker.com/?url=https%3A%2F%2Fexample.com";
        let exit_result = clink.find_and_replace(exit_lookalike);
        assert_eq!(
            exit_result.exits_unwrapped, 0,
            "exit.sc's url_pattern must not match lookalike host"
        );
    }

    #[test]
    fn clink_extras_survive_cache_without_them() {
        // After `clink update` the cache contains only the translated remote
        // (e.g. ClearURLs), which doesn't have niche redirectors like exit.sc
        // or clink-curated extras like the explicit Amazon rules.
        // Those entries live in the user's config template, so they must
        // still apply even when the cache replaces the builtin layer.
        let id = std::thread::current().id();
        let cfg_path = std::env::temp_dir().join(format!("clink_test_extras_cfg_{id:?}.toml"));
        let cache_dir = std::env::temp_dir().join(format!("clink_test_extras_cache_{id:?}"));
        let _ = std::fs::remove_dir_all(&cache_dir);
        std::fs::create_dir_all(&cache_dir).unwrap();

        std::fs::write(&cfg_path, crate::config::DEFAULT_CONFIG_TEMPLATE).unwrap();

        let cache_toml = "[providers.global]\nrules = [\"fbclid\", \"gclid\"]\n";
        std::fs::write(cache_dir.join("remote_patterns.toml"), cache_toml).unwrap();

        let mut cfg = crate::config::load_config(&cfg_path).unwrap();
        crate::remote::resolve_patterns(&mut cfg, &cache_dir);
        let clink = Clink::new(cfg);

        assert_eq!(
            clink
                .find_and_replace("https://exit.sc/?url=https%3A%2F%2Fexample.com")
                .text,
            "https://example.com/",
            "exit.sc redirect must still unwrap when cache lacks it"
        );
        assert_eq!(
            clink
                .find_and_replace("https://www.amazon.com/dp/X?sp_csd=abc&keep=me")
                .text,
            "https://www.amazon.com/dp/X?keep=me",
            "amazon sp_csd must still strip when cache lacks it"
        );

        let _ = std::fs::remove_file(&cfg_path);
        let _ = std::fs::remove_dir_all(&cache_dir);
    }
}

#[cfg(test)]
mod unwrap_exit_params {
    use crate::{clink::Clink, mode::Mode};

    #[test]
    fn has_exit_url() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink.try_unwrap_redirect(
                "https://exit.sc/?url=https%3A%2F%2Fopen.spotify.com%2Fartist%2F3tEV3J5gW5BDMrJqE3NaBy%3Fsi%3D1mLk6MZSRGuol8rgwCe_Cg"
            ).0,
            "https://open.spotify.com/artist/3tEV3J5gW5BDMrJqE3NaBy?si=1mLk6MZSRGuol8rgwCe_Cg"
        );

        assert_eq!(
        clink.try_unwrap_redirect(
            "https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjMuu2zrreBAxUt2gIHHaDVC_gQyCl6BAgqEAM&url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DdQw4w9WgXcQ&usg=AOvVaw0aHtehaphMhOCAkCydRLZU&opi=89978449"
        ).0,
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    )
    }

    #[test]
    fn has_no_exit_url() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink.try_unwrap_redirect(
                "https://open.spotify.com/artist/3tEV3J5gW5BDMrJqE3NaBy?si=1mLk6MZSRGuol8rgwCe_Cg"
            ).0,
            "https://open.spotify.com/artist/3tEV3J5gW5BDMrJqE3NaBy?si=1mLk6MZSRGuol8rgwCe_Cg"
        );
    }

    #[test]
    fn has_exit_url_google_it() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink
                .try_unwrap_redirect("https://www.google.it/url?url=https%3A%2F%2Fexample.com&sa=t")
                .0,
            "https://example.com"
        );
    }

    #[test]
    fn has_exit_url_google_q_param() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink
                .try_unwrap_redirect(
                    "https://www.google.com/url?q=https%3A%2F%2Fexample.com&sa=t&usg=abc123"
                )
                .0,
            "https://example.com"
        );
    }

    #[test]
    fn has_exit_url_bing() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink
                .try_unwrap_redirect("https://bing.com/ck/a?u=https%3A%2F%2Fexample.com&foo=bar")
                .0,
            "https://example.com"
        );
    }

    #[test]
    fn amazon_com_tracking_stripped() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace(
                    "https://www.amazon.com/dp/B08N5WRWNW?sp_csd=d2lkZ2V0TmFtZQ&pd_rd_w=abc&keep=me"
                )
                .text,
            "https://www.amazon.com/dp/B08N5WRWNW?keep=me"
        );
    }

    #[test]
    fn youtube_music_si_stripped() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink
                .find_and_replace(
                    "https://music.youtube.com/watch?v=dQw4w9WgXcQ&si=NblIBgit-qHN7MoH"
                )
                .text,
            "https://music.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }

    #[test]
    fn has_exit_url_but_no_exit_param() {
        let clink = Clink::new(super::test_config(Mode::Remove));
        assert_eq!(
            clink.try_unwrap_redirect("https://exit.sc/?foo=bar").0,
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
