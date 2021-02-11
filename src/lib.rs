extern crate linkify;
extern crate url;

use url::Url;
use linkify::{LinkFinder, LinkKind};

#[cfg(test)]
mod find_and_replace {
    use super::*;

    #[test]
    fn naive() {
        assert_eq!(
            find_and_replace(
                &"https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa".to_string()
            ),  
            "https://test.test/"
        );
    }
    #[test]
    fn should_preserve_query() {
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc".to_string()
            ),  
            "https://test.test/?abc=abc"
        );
    }
    #[test]
    fn multiple_params() {
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc&fbclid=flksj".to_string()
            ),  
            "https://test.test/?abc=abc"
        );
    }
    #[test]
    fn multiple_links() {
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj".to_string()
            ),  
            "https://test.test/?abc=abc\nhttps://test.test/?abc=abc"
        );
    }
    #[test]
    fn multiple_links_and_text() {
        assert_eq!(
            find_and_replace(
                &"some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj".to_string()
            ),  
            "some text here https://test.test/?abc=abc here \nand herehttps://test.test/?abc=abc"
        );
    }


}

pub fn find_and_replace(str: &String)-> String {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    let mut res = str.clone();
    for link in finder.links(str){
        let l = Url::parse(link.as_str()).unwrap();

        let query: Vec<(_, _)> = l.query_pairs()
        .filter(|p| p.0 != "fbclid" && p.0 != "utm_source" && p.0 != "utm_campaign" && p.0 != "utm_medium")
        .collect();
        
        let mut l2 = l.clone();
        l2.set_query(None);
  
        for pair in query {
            l2.query_pairs_mut()
                .append_pair(&pair.0.to_string()[..], &pair.1.to_string()[..]);
        }

        res = res.replace(link.as_str(), l2.as_str());
    }
    
    return res;
}


