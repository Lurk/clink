use std::collections::HashMap;
use lazy_static::lazy_static;

#[cfg(test)]
mod is_hit {
    use super::*;

    #[test]
    fn existing_keys() {
      for param in get_params(){
        assert!(is_hit(param));
      }
    }

    #[test]
    fn not_exiting_keys(){
      assert!(!is_hit("not_exiting_key"));
    }
}

lazy_static! {
  pub static ref GET_PARAMS: HashMap<&'static str, bool> = {
      let mut map = HashMap::new();
      map.insert("fbclid", true);
      map.insert("gclid", true);
      map.insert("gclsrc", true);
      map.insert("dclid", true);
      map.insert("zanpid", true);
      map.insert("utm_source", true);
      map.insert("utm_campaign", true);
      map.insert("utm_medium", true);
      map.insert("utm_term", true);
      map.insert("utm_content", true);
      map
  };
}

pub fn is_hit(name: &str) -> bool {
  GET_PARAMS.contains_key(name)
}

pub fn get_params() -> Vec<&'static &'static str>{
  GET_PARAMS.keys().collect()
}