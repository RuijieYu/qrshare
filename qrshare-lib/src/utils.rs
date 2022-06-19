use std::mem::replace;

use hyper::{Response, StatusCode};

pub fn query_split(s: &str) -> Vec<(&str, &str)> {
    s.split('&').filter_map(|p| p.split_once('=')).collect()
}

pub fn query_split_opt(s: Option<&str>) -> Vec<(&str, &str)> {
    match s {
        Some(s) => query_split(s),
        None => vec![],
    }
}

/// Swap provided status code and the internal response status code, and return
/// the swapped values.
pub fn swap_status<Body>(
    mut resp: Response<Body>,
    s: StatusCode,
) -> (Response<Body>, StatusCode) {
    let s = replace(resp.status_mut(), s);
    (resp, s)
}

/// Create a response just from a status code
pub fn status<Body>(s: StatusCode) -> Response<Body>
where
    Body: From<&'static str>,
{
    swap_status(Response::new("".into()), s).0
}
