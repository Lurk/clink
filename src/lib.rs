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
                &"https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa".to_string(),
                &ProcessTypes::REMOVE
            ),  
            "https://test.test/"
        );
        assert_eq!(
            find_and_replace(
                &"https://test.test/?fbclid=dsadsa&utm_source=fafa&utm_campaign=fafas&utm_medium=adsa".to_string(),
                &ProcessTypes::YourMom
            ),  
            "https://test.test/?fbclid=your_mom&utm_source=your_mom&utm_campaign=your_mom&utm_medium=your_mom"
        );

    }
    #[test]
    fn should_preserve_query() {
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc".to_string(),
                &ProcessTypes::REMOVE
            ),  
            "https://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc".to_string(),
                &ProcessTypes::YourMom
            ),  
            "https://test.test/?abc=abc"
        );

    }
    #[test]
    fn multiple_params() {
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc&fbclid=flksj".to_string(),
                &ProcessTypes::REMOVE
            ),  
            "https://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc&fbclid=flksj".to_string(),
                &ProcessTypes::YourMom
            ),  
            "https://test.test/?abc=abc&fbclid=your_mom"
        );

    }
    #[test]
    fn multiple_links() {
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj".to_string(),
                &ProcessTypes::REMOVE
            ),  
            "https://test.test/?abc=abc\nhttps://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                &"https://test.test/?abc=abc&fbclid=flksj\nhttps://test.test/?abc=abc&fbclid=flksj".to_string(),
                &ProcessTypes::YourMom
            ),  
            "https://test.test/?abc=abc&fbclid=your_mom\nhttps://test.test/?abc=abc&fbclid=your_mom"
        );

    }
    #[test]
    fn multiple_links_and_text() {
        assert_eq!(
            find_and_replace(
                &"some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj".to_string(),
                &ProcessTypes::REMOVE
            ),  
            "some text here https://test.test/?abc=abc here \nand herehttps://test.test/?abc=abc"
        );
        assert_eq!(
            find_and_replace(
                &"some text here https://test.test/?abc=abc&fbclid=flksj here \nand herehttps://test.test/?abc=abc&fbclid=flksj".to_string(),
                &ProcessTypes::YourMom
            ),  
            "some text here https://test.test/?abc=abc&fbclid=your_mom here \nand herehttps://test.test/?abc=abc&fbclid=your_mom"
        );

    }


}

pub fn find_and_replace(str: &String, process_type: &ProcessTypes)-> String {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);
    let mut res = str.clone();
    for link in finder.links(str){
        let l = Url::parse(link.as_str()).unwrap();

        let query: Vec<(_, _)> = process_query(l.query_pairs(), process_type);
        
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

pub enum ProcessTypes{
    REMOVE,
    YourMom
}


fn process_query(query: url::form_urlencoded::Parse<'_>, process_type: &ProcessTypes)-> Vec<(String, String)>{
    match process_type {
        ProcessTypes::REMOVE => {
            return query
            .filter(|p| !is_hit(&p.0))
            .map(|p| (p.0.to_string(), p.1.to_string()))
            .collect()
        }
        ProcessTypes::YourMom => {
            return query.map(|p| if is_hit(&p.0) {(p.0.to_string(), "your_mom".to_string())} else {(p.0.to_string(), p.1.to_string())}).collect();
        }
    } 
}


#[cfg(test)]
mod is_hit {
    use super::*;

    #[test]
    fn test_all() {
        assert!(is_hit("fbclid"));
        assert!(is_hit("utm_source"));
        assert!(is_hit("utm_campaign"));
        assert!(is_hit("utm_medium"));
    }
}
fn is_hit(p: &str) -> bool {
    p == "fbclid" || p == "utm_source" || p == "utm_campaign" || p == "utm_medium"
}

