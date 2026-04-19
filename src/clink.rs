use crate::config::ClinkConfig;
use crate::mode::Mode;
use crate::provider::{CompiledProvider, CompiledRules};
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
        let mut res = input.to_string();
        let mut urls_cleaned: u32 = 0;
        let mut params_removed: u32 = 0;
        let mut exits_unwrapped: u32 = 0;

        for link in self.finder.links(input) {
            let (unwrapped, was_exit) = self.try_unwrap_redirect(link.as_str());
            if was_exit {
                exits_unwrapped += 1;
            }
            let mut l = Url::parse(unwrapped.as_str()).expect("url to be parsable");
            let normalized_original = l.to_string();
            #[allow(clippy::cast_possible_truncation)]
            let original_param_count = l.query_pairs().count() as u32;

            let matching_providers = self.find_matching_providers(unwrapped.as_str());

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
                res = res.replace(link.as_str(), new_url);
            }
        }

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
                            (
                                key,
                                swap_two_chars(
                                    &value,
                                    rng.random_range(0..value.len()),
                                    rng.random_range(0..value.len()),
                                ),
                            )
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
    let template = include_str!("default_config.toml");
    std::fs::write(&tmp, template).unwrap();
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

        let template = include_str!("default_config.toml");
        std::fs::write(&cfg_path, template).unwrap();

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
