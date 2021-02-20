use std::collections::HashMap;

#[cfg(test)]
mod is_hit {
    use super::*;

    #[test]
    fn existing_keys() {
        let index = create_index(&get_default_params());
        for param in get_default_params() {
            assert!(is_hit(&param, &index));
        }
    }

    #[test]
    fn not_exiting_keys() {
        let index = create_index(&get_default_params());
        assert!(!is_hit("not_exiting_key", &index));
    }
}

pub fn get_default_params() -> Vec<String> {
    vec![
        "fbclid".to_string(),
        "gclid".to_string(),
        "gclsrc".to_string(),
        "dclid".to_string(),
        "zanpid".to_string(),
        "utm_source".to_string(),
        "utm_campaign".to_string(),
        "utm_medium".to_string(),
        "utm_term".to_string(),
        "utm_content".to_string(),
    ]
}

pub fn create_index(vec: &[String]) -> HashMap<String, bool> {
    let mut map: HashMap<String, bool> = HashMap::new();
    for key in vec.iter() {
        map.insert(key.to_string(), true);
    }
    map
}

pub fn is_hit(name: &str, params: &HashMap<String, bool>) -> bool {
    params.contains_key(name)
}
