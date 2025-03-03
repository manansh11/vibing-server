use crate::error::ServerResult;
use crate::http::{Method, Request, Response, Status};
use crate::router::Router;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A map of file extensions to content types
fn content_type_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    
    // Text types
    map.insert("html", "text/html");
    map.insert("htm", "text/html");
    map.insert("css", "text/css");
    map.insert("js", "text/javascript");
    map.insert("txt", "text/plain");
    map.insert("md", "text/markdown");
    map.insert("csv", "text/csv");
    
    // Application types
    map.insert("json", "application/json");
    map.insert("xml", "application/xml");
    map.insert("pdf", "application/pdf");
    map.insert("zip", "application/zip");
    map.insert("tar", "application/x-tar");
    map.insert("gz", "application/gzip");
    map.insert("wasm", "application/wasm");
    
    // Image types
    map.insert("png", "image/png");
    map.insert("jpg", "image/jpeg");
    map.insert("jpeg", "image/jpeg");
    map.insert("gif", "image/gif");
    map.insert("svg", "image/svg+xml");
    map.insert("webp", "image/webp");
    map.insert("ico", "image/x-icon");
    
    // Audio types
    map.insert("mp3", "audio/mpeg");
    map.insert("wav", "audio/wav");
    map.insert("ogg", "audio/ogg");
    
    // Video types
    map.insert("mp4", "video/mp4");
    map.insert("webm", "video/webm");
    
    // Font types
    map.insert("ttf", "font/ttf");
    map.insert("otf", "font/otf");
    map.insert("woff", "font/woff");
    map.insert("woff2", "font/woff2");
    
    map
}

/// Get the content type for a file based on its extension
fn get_content_type(path: &Path) -> &'static str {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    content_type_map().get(ext).copied().unwrap_or("application/octet-stream")
}

/// Configuration for the static file server
#[derive(Clone, Debug)]
pub struct StaticFileConfig {
    /// The root directory to serve files from
    pub root_dir: PathBuf,
    
    /// The URL path prefix to serve files from
    pub path_prefix: String,
    
    /// The index file to serve for directory requests
    pub index_file: String,
    
    /// Whether to follow symlinks
    pub follow_symlinks: bool,
    
    /// Whether to show directory listings
    pub directory_listing: bool,
    
    /// Maximum file size to serve
    pub max_file_size: usize,
    
    /// Cache control header value
    pub cache_control: String,
}

impl Default for StaticFileConfig {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("static"),
            path_prefix: "/static".to_string(),
            index_file: "index.html".to_string(),
            follow_symlinks: false,
            directory_listing: false,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            cache_control: "public, max-age=3600".to_string(),
        }
    }
}

/// Add static file routes to a router
pub fn add_static_file_routes(router: &mut Router, config: StaticFileConfig) {
    // Create local copies of the configuration
    let root_dir = config.root_dir.clone();
    let path_prefix = config.path_prefix.clone();
    let index_file = config.index_file.clone();
    let follow_symlinks = config.follow_symlinks;
    let directory_listing = config.directory_listing;
    let max_file_size = config.max_file_size;
    let cache_control = config.cache_control.clone();
    
    // Wildcard route to match all requests to the path prefix
    let wildcard_path = format!("{}/*", path_prefix);
    
    // Clone these values specifically for the first closure
    let root_dir_wild = root_dir.clone();
    let path_prefix_wild = path_prefix.clone();
    let index_file_wild = index_file.clone();
    let cache_control_wild = cache_control.clone();
    let directory_listing_wild = directory_listing;
    let follow_symlinks_wild = follow_symlinks;
    let max_file_size_wild = max_file_size;
    
    router.get(&wildcard_path, move |req| {
        // Extract the path from the request
        let path = req.uri.strip_prefix(&path_prefix_wild).unwrap_or(&req.uri);
        let path = path.trim_start_matches('/');
        
        // Construct the filesystem path
        let mut fs_path = root_dir_wild.clone();
        for segment in path.split('/') {
            // Skip empty segments and prevent directory traversal
            if segment.is_empty() || segment == "." || segment == ".." {
                continue;
            }
            fs_path.push(segment);
        }
        
        // Check if the path exists
        if !fs_path.exists() {
            let mut response = Response::new(Status::NotFound);
            response.set_body(format!("File not found: {}", path).as_bytes());
            return Ok(response);
        }
        
        // Check if it's a directory
        if fs_path.is_dir() {
            // Try to serve the index file
            let index_path = fs_path.join(&index_file_wild);
            if index_path.exists() && index_path.is_file() {
                fs_path = index_path;
            } else if directory_listing_wild {
                // Generate a directory listing
                return serve_directory_listing(&fs_path, &path_prefix_wild, path);
            } else {
                // Directory listing not allowed
                let mut response = Response::new(Status::Forbidden);
                response.set_body(b"Directory listing not allowed");
                return Ok(response);
            }
        }
        
        // Check if it's a symlink and whether symlinks are allowed
        if fs_path.is_symlink() && !follow_symlinks_wild {
            let mut response = Response::new(Status::Forbidden);
            response.set_body(b"Symlinks not allowed");
            return Ok(response);
        }
        
        // Try to read the file
        match fs::read(&fs_path) {
            Ok(contents) => {
                // Check file size
                if contents.len() > max_file_size_wild {
                    let mut response = Response::new(Status::PayloadTooLarge);
                    response.set_body(b"File too large");
                    return Ok(response);
                }
                
                // Set content type based on file extension
                let content_type = get_content_type(&fs_path);
                
                // Create the response
                let mut response = Response::new(Status::Ok);
                response.set_header("Content-Type", content_type);
                response.set_header("Cache-Control", &cache_control_wild);
                response.set_body(&contents);
                
                Ok(response)
            }
            Err(_) => {
                let mut response = Response::new(Status::InternalServerError);
                response.set_body(b"Error reading file");
                Ok(response)
            }
        }
    });
    
    // Serve the root path prefix - create new clones for this closure
    let root_dir_root = root_dir.clone();
    let path_prefix_root = path_prefix.clone();
    let index_file_root = index_file.clone();
    let cache_control_root = cache_control.clone();
    let directory_listing_root = directory_listing;
    
    router.get(&path_prefix, move |req| {
        // Try to serve the index file from the root directory
        let index_path = root_dir_root.join(&index_file_root);
        if index_path.exists() && index_path.is_file() {
            match fs::read(&index_path) {
                Ok(contents) => {
                    let content_type = get_content_type(&index_path);
                    
                    let mut response = Response::new(Status::Ok);
                    response.set_header("Content-Type", content_type);
                    response.set_header("Cache-Control", &cache_control_root);
                    response.set_body(&contents);
                    
                    Ok(response)
                }
                Err(_) => {
                    let mut response = Response::new(Status::InternalServerError);
                    response.set_body(b"Error reading index file");
                    Ok(response)
                }
            }
        } else if directory_listing_root {
            // Generate a directory listing for the root directory
            serve_directory_listing(&root_dir_root, &path_prefix_root, "")
        } else {
            // Directory listing not allowed
            let mut response = Response::new(Status::Forbidden);
            response.set_body(b"Directory listing not allowed");
            Ok(response)
        }
    });
}

/// Serve a directory listing
fn serve_directory_listing(dir_path: &Path, path_prefix: &str, relative_path: &str) -> ServerResult<Response> {
    // Read the directory
    let entries = match fs::read_dir(dir_path) {
        Ok(entries) => entries,
        Err(_) => {
            let mut response = Response::new(Status::InternalServerError);
            response.set_body(b"Error reading directory");
            return Ok(response);
        }
    };
    
    // Build the HTML for the directory listing
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><title>Directory Listing</title>");
    html.push_str("<style>body{font-family:sans-serif;max-width:800px;margin:0 auto;padding:20px;line-height:1.6;}");
    html.push_str("h1{border-bottom:1px solid #ddd;padding-bottom:10px;}");
    html.push_str("ul{list-style-type:none;padding:0;}");
    html.push_str("li{margin-bottom:8px;}");
    html.push_str("a{text-decoration:none;color:#2980b9;}");
    html.push_str("a:hover{text-decoration:underline;}</style>");
    html.push_str("</head><body>");
    
    // Directory title
    if relative_path.is_empty() {
        html.push_str("<h1>Index of /</h1>");
    } else {
        html.push_str(&format!("<h1>Index of /{}</h1>", relative_path));
    }
    
    // Parent directory link
    if !relative_path.is_empty() {
        let parent_path = relative_path.rsplitn(2, '/').nth(1).unwrap_or("");
        let parent_url = if parent_path.is_empty() {
            format!("{}", path_prefix)
        } else {
            format!("{}/{}", path_prefix, parent_path)
        };
        html.push_str(&format!("<p><a href=\"{}\">..</a> (Parent Directory)</p>", parent_url));
    }
    
    // List of files and directories
    html.push_str("<ul>");
    
    let mut entries_vec = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            entries_vec.push(entry);
        }
    }
    
    // Sort entries: directories first, then files, alphabetically
    entries_vec.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        
        if a_is_dir && !b_is_dir {
            std::cmp::Ordering::Less
        } else if !a_is_dir && b_is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.file_name().cmp(&b.file_name())
        }
    });
    
    for entry in entries_vec {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        
        // Skip hidden files
        if file_name_str.starts_with('.') {
            continue;
        }
        
        let file_type = entry.file_type();
        if let Ok(file_type) = file_type {
            let is_dir = file_type.is_dir();
            
            let entry_url = if relative_path.is_empty() {
                format!("{}/{}", path_prefix, file_name_str)
            } else {
                format!("{}/{}/{}", path_prefix, relative_path, file_name_str)
            };
            
            let display_name = if is_dir {
                format!("{}/", file_name_str)
            } else {
                file_name_str.to_string()
            };
            
            html.push_str(&format!("<li><a href=\"{}\">{}</a></li>", entry_url, display_name));
        }
    }
    
    html.push_str("</ul></body></html>");
    
    // Create the response
    let mut response = Response::new(Status::Ok);
    response.set_header("Content-Type", "text/html");
    response.set_body(html.as_bytes());
    
    Ok(response)
}

/// Create a static file server middleware
pub fn static_files_middleware(
    config: StaticFileConfig,
) -> impl Fn(&Request, crate::middleware::MiddlewareNext) -> ServerResult<Response> + Send + Sync {
    // Clone all the configuration values that need to be moved into the closure
    let root_dir = config.root_dir.clone();
    let path_prefix = config.path_prefix.clone();
    let index_file = config.index_file.clone();
    let follow_symlinks = config.follow_symlinks;
    let directory_listing = config.directory_listing;
    let max_file_size = config.max_file_size;
    let cache_control = config.cache_control.clone();
    
    move |req, next| {
        // Check if the request is for a static file
        if req.method == Method::Get && req.uri.starts_with(&path_prefix) {
            // Extract the path from the request
            let path = req.uri.strip_prefix(&path_prefix).unwrap_or(&req.uri);
            let path = path.trim_start_matches('/');
            
            // Construct the filesystem path
            let mut fs_path = root_dir.clone();
            for segment in path.split('/') {
                // Skip empty segments and prevent directory traversal
                if segment.is_empty() || segment == "." || segment == ".." {
                    continue;
                }
                fs_path.push(segment);
            }
            
            // If the path exists, serve it
            if fs_path.exists() {
                // Check if it's a directory
                if fs_path.is_dir() {
                    // Try to serve the index file
                    let index_path = fs_path.join(&index_file);
                    if index_path.exists() && index_path.is_file() {
                        fs_path = index_path;
                    } else if directory_listing {
                        // Generate a directory listing
                        return serve_directory_listing(&fs_path, &path_prefix, path);
                    } else {
                        // Directory listing not allowed, pass to next middleware
                        return next(req);
                    }
                }
                
                // Check if it's a symlink and whether symlinks are allowed
                if fs_path.is_symlink() && !follow_symlinks {
                    return next(req);
                }
                
                // Try to read the file
                match fs::read(&fs_path) {
                    Ok(contents) => {
                        // Check file size
                        if contents.len() > max_file_size {
                            let mut response = Response::new(Status::PayloadTooLarge);
                            response.set_body(b"File too large");
                            return Ok(response);
                        }
                        
                        // Set content type based on file extension
                        let content_type = get_content_type(&fs_path);
                        
                        // Create the response
                        let mut response = Response::new(Status::Ok);
                        response.set_header("Content-Type", content_type);
                        response.set_header("Cache-Control", &cache_control);
                        response.set_body(&contents);
                        
                        return Ok(response);
                    }
                    Err(_) => {
                        // Error reading file, pass to next middleware
                        return next(req);
                    }
                }
            }
        }
        
        // Not a static file request or file not found, pass to next middleware
        next(req)
    }
}