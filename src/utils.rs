pub(crate) fn query_split(s: &str) -> Vec<(&str, &str)> {
    s.split('&').filter_map(|p| p.split_once('=')).collect()
}

pub(crate) fn query_split_opt(s: Option<&str>) -> Vec<(&str, &str)> {
    match s {
        Some(s) => query_split(s),
        None => vec![],
    }
}
