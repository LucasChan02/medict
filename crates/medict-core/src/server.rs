use std::sync::Arc;
use std::thread;

use percent_encoding::percent_decode_str;
use tiny_http::{Header, Response, Server, StatusCode};
use url::Url;

use crate::error::{MedictError, Result};
use crate::state::AppCore;
use crate::util::mime_for_path;

pub fn start(core: Arc<AppCore>) -> Result<String> {
    let server = Server::http("127.0.0.1:0")
        .map_err(|error| MedictError::Message(format!("Cannot start the content server: {error}")))?;
    let address = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| MedictError::Message("The content server has no TCP address.".to_string()))?;
    let token = core.server_token().to_string();
    let base_url = format!("http://127.0.0.1:{}/__medict/{token}/", address.port());
    core.set_base_url(base_url.clone());

    thread::Builder::new()
        .name("medict-content-server".to_string())
        .spawn(move || {
            for request in server.incoming_requests() {
                handle_request(&core, request, &token);
            }
        })
        .map_err(MedictError::from)?;
    Ok(base_url)
}

fn handle_request(core: &Arc<AppCore>, request: tiny_http::Request, token: &str) {
    if request.method() != &tiny_http::Method::Get {
        let _ = request.respond(Response::empty(StatusCode(405)));
        return;
    }
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{}", request.url())) else {
        let _ = request.respond(Response::empty(StatusCode(400)));
        return;
    };
    let path = percent_decode_str(url.path()).decode_utf8_lossy().into_owned();
    let prefix = format!("/__medict/{token}/");
    let Some(route) = path.strip_prefix(&prefix) else {
        let _ = request.respond(Response::empty(StatusCode(404)));
        return;
    };

    if route == "health" {
        respond_text(request, 200, "text/plain; charset=utf-8".to_string(), b"ok".to_vec());
        return;
    }
    if route == "entry" {
        let dict_id = query_value(&url, "dict_id");
        let word = query_value(&url, "word");
        match core.entry_html(&dict_id, &word) {
            Ok(Some(body)) => respond_text(
                request,
                200,
                "text/html; charset=utf-8".to_string(),
                body.into_bytes(),
            ),
            Ok(None) => respond_text(
                request,
                404,
                "text/plain; charset=utf-8".to_string(),
                b"The word was not found.".to_vec(),
            ),
            Err(error) => respond_text(
                request,
                500,
                "text/plain; charset=utf-8".to_string(),
                error.to_string().into_bytes(),
            ),
        }
        return;
    }
    if let Some(tail) = route.strip_prefix("resource/") {
        let (dict_id, resource_path) = tail.split_once('/').unwrap_or((tail, ""));
        if let Some(bytes) = core.resource(dict_id, resource_path) {
            let content_type = mime_for_path(std::path::Path::new(resource_path)).to_string();
            respond_text(request, 200, content_type, bytes);
        } else {
            let _ = request.respond(Response::empty(StatusCode(404)));
        }
        return;
    }

    let referer = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Referer"))
        .map(|header| header.value.as_str().to_string())
        .unwrap_or_default();
    if let Some(dict_id) = dictionary_from_referer(&referer, token) {
        if let Some(bytes) = core.resource(&dict_id, route) {
            let content_type = mime_for_path(std::path::Path::new(route)).to_string();
            respond_text(request, 200, content_type, bytes);
            return;
        }
    }
    let _ = request.respond(Response::empty(StatusCode(404)));
}

fn respond_text(request: tiny_http::Request, status: u16, content_type: String, bytes: Vec<u8>) {
    let mut response = Response::from_data(bytes).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes("Content-Type", content_type) {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("Cache-Control", "no-store") {
        response.add_header(header);
    }
    if let Ok(header) = Header::from_bytes("X-Content-Type-Options", "nosniff") {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

fn query_value(url: &Url, name: &str) -> String {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
        .unwrap_or_default()
}

fn dictionary_from_referer(referer: &str, token: &str) -> Option<String> {
    let url = Url::parse(referer).ok()?;
    let path = url.path();
    if let Some(dict_id) = url
        .query_pairs()
        .find(|(key, _)| key == "dict_id")
        .map(|(_, value)| value.into_owned())
    {
        return Some(dict_id);
    }
    let prefix = format!("/__medict/{token}/resource/");
    path.strip_prefix(&prefix)?
        .split_once('/')
        .map(|(dict_id, _)| dict_id.to_string())
}
