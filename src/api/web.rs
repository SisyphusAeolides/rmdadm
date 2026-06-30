use axum::{
    response::{Html, IntoResponse, Response},
    http::StatusCode,
};
use std::path::PathBuf;
use tokio::fs;

/// Serve the main dashboard HTML
pub async fn serve_dashboard() -> Response {
    // Try multiple locations for web files
    let paths = [
        "/usr/share/rmdadm/web/templates/index.html",
        "web/templates/index.html",
        "/home/Sisyphus/rmdadm/web/templates/index.html",
    ];
    
    for path in &paths {
        if let Ok(content) = fs::read_to_string(path).await {
            return Html(content).into_response();
        }
    }
    
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load dashboard - web files not found"
    ).into_response()
}

/// Serve static files (CSS, JS, images)
pub async fn serve_static(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    // Try multiple base directories
    let base_dirs = [
        "/usr/share/rmdadm/web/static",
        "web/static",
        "/home/Sisyphus/rmdadm/web/static",
    ];
    
    for base_dir in &base_dirs {
        let file_path = PathBuf::from(base_dir).join(&path);
        
        // Security: prevent directory traversal
        if !file_path.starts_with(base_dir) {
            continue;
        }

        if let Ok(content) = fs::read(&file_path).await {
            let mime_type = get_mime_type(&path);
            return (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, mime_type)],
                content
            ).into_response();
        }
    }
    
    (StatusCode::NOT_FOUND, "File not found").into_response()
}

fn get_mime_type(path: &str) -> &'static str {
    if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}