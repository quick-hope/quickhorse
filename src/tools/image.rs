//! ImageTool - Image file operations and analysis
//!
//! Provides:
//! - Image file reading (PNG, JPG, GIF, BMP, WebP)
//! - Image metadata extraction (dimensions, format, size)
//! - Base64 encoding for multimodal API support
//! - Image description via vision-capable models

use crate::tools::{Tool, ToolContext, ToolResult, build_schema};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::ImageFormat;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::BufReader;
use std::path::Path;

/// ImageTool - Image file operations
pub struct ImageTool;

impl ImageTool {
    pub fn new() -> Self {
        Self
    }
}

/// Image action type
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImageAction {
    /// Get image information (dimensions, format, size)
    Info,
    /// Encode image to base64
    Encode,
    /// Analyze image with vision model (requires provider support)
    Analyze,
}

/// Image input parameters
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ImageInput {
    /// Path to image file
    pub file_path: String,
    /// Action to perform (info, encode, analyze)
    #[serde(default = "default_action")]
    pub action: ImageAction,
}

fn default_action() -> ImageAction {
    ImageAction::Info
}

/// Image output result
#[derive(Debug, Clone, Serialize)]
pub struct ImageOutput {
    /// File path
    pub file_path: String,
    /// Image dimensions (width, height)
    pub dimensions: Option<(u32, u32)>,
    /// Image format (PNG, JPEG, etc.)
    pub format: Option<String>,
    /// File size in bytes
    pub size_bytes: u64,
    /// Base64 encoded image data (for encode action)
    pub base64: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Image description (for analyze action, if supported)
    pub description: Option<String>,
}

/// Supported image formats
const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff"];

/// Get MIME type for image format
fn get_mime_type(format: ImageFormat) -> String {
    match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Gif => "image/gif",
        ImageFormat::Bmp => "image/bmp",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Tiff => "image/tiff",
        _ => "application/octet-stream",
    }.to_string()
}

/// Get format name string from ImageFormat
fn format_name_from_image_format(format: ImageFormat) -> String {
    match format {
        ImageFormat::Png => "PNG",
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::Gif => "GIF",
        ImageFormat::Bmp => "BMP",
        ImageFormat::WebP => "WebP",
        ImageFormat::Tiff => "TIFF",
        _ => "Unknown",
    }.to_string()
}

/// Detect image format from file extension
fn detect_format(path: &Path) -> Option<ImageFormat> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "gif" => Some(ImageFormat::Gif),
        "bmp" => Some(ImageFormat::Bmp),
        "webp" => Some(ImageFormat::WebP),
        "tiff" | "tif" => Some(ImageFormat::Tiff),
        _ => None,
    }
}

/// Check if file is a supported image format
fn is_supported_image(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        SUPPORTED_FORMATS.contains(&ext_lower.as_str())
    } else {
        false
    }
}

/// Get image dimensions using image crate
fn get_image_dimensions(path: &Path) -> Result<(u32, u32), Box<dyn Error>> {
    let reader = BufReader::new(fs::File::open(path)?);
    let img = image::load(reader, image::ImageFormat::from_path(path)?)?;
    Ok((img.width(), img.height()))
}

#[async_trait]
impl Tool for ImageTool {
    fn name(&self) -> &str {
        "Image"
    }

    fn description(&self) -> String {
        "Read image files, get metadata (dimensions, format, size), encode to base64 for multimodal API support."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<ImageInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        // Parse input
        let params: ImageInput = serde_json::from_value(input)?;

        let path = Path::new(&params.file_path);

        // Check file exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "Image file not found: {}",
                params.file_path
            )));
        }

        // Check if supported format
        if !is_supported_image(path) {
            let ext = path.extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Ok(ToolResult::error(format!(
                "Unsupported image format: {}. Supported: {}",
                ext,
                SUPPORTED_FORMATS.join(", ")
            )));
        }

        // Get file size
        let size_bytes = match fs::metadata(path) {
            Ok(m) => m.len(),
            Err(e) => return Ok(ToolResult::error(format!("Failed to get file size: {}", e))),
        };

        // Detect format
        let format = detect_format(path);
        let format_name = format
            .map(|f| format_name_from_image_format(f))
            .unwrap_or_else(|| "unknown".to_string());
        let mime_type = format
            .map(|f| get_mime_type(f))
            .unwrap_or_else(|| "application/octet-stream".to_string());

        match params.action {
            ImageAction::Info => {
                // Get image dimensions
                let dimensions = match get_image_dimensions(path) {
                    Ok(d) => Some(d),
                    Err(e) => {
                        // Still return info even if dimensions fail
                        eprintln!("Warning: Failed to get dimensions: {}", e);
                        None
                    }
                };

                let output = ImageOutput {
                    file_path: params.file_path.clone(),
                    dimensions,
                    format: Some(format_name.clone()),
                    size_bytes,
                    base64: None,
                    mime_type: Some(mime_type),
                    description: None,
                };

                Ok(ToolResult::success(serde_json::to_string(&output).unwrap_or_else(|_| {
                    format!("Image info: {} ({}, {} bytes)", params.file_path, format_name, size_bytes)
                })))
            }

            ImageAction::Encode => {
                // Read and encode image
                let data = match fs::read(path) {
                    Ok(d) => d,
                    Err(e) => return Ok(ToolResult::error(format!("Failed to read image: {}", e))),
                };

                let base64_str = STANDARD.encode(&data);

                // Also get dimensions if possible
                let dimensions = match get_image_dimensions(path) {
                    Ok(d) => Some(d),
                    Err(_) => None,
                };

                let output = ImageOutput {
                    file_path: params.file_path.clone(),
                    dimensions,
                    format: Some(format_name.clone()),
                    size_bytes,
                    base64: Some(base64_str.clone()),
                    mime_type: Some(mime_type),
                    description: None,
                };

                Ok(ToolResult::success(serde_json::to_string(&output).unwrap_or_else(|_| {
                    format!("Image encoded: {} (base64 length: {})", params.file_path, base64_str.len())
                })))
            }

            ImageAction::Analyze => {
                // Note: Actual analysis would require calling the provider
                // This returns the encoded data ready for provider call
                let data = match fs::read(path) {
                    Ok(d) => d,
                    Err(e) => return Ok(ToolResult::error(format!("Failed to read image: {}", e))),
                };

                let base64_str = STANDARD.encode(&data);

                // Get dimensions
                let dimensions = match get_image_dimensions(path) {
                    Ok(d) => Some(d),
                    Err(_) => None,
                };

                let output = ImageOutput {
                    file_path: params.file_path.clone(),
                    dimensions,
                    format: Some(format_name.clone()),
                    size_bytes,
                    base64: Some(base64_str.clone()),
                    mime_type: Some(mime_type),
                    description: Some("Image ready for analysis. Pass base64 data to vision-capable provider (GPT-4 Vision, Claude 3, Gemini).".to_string()),
                };

                Ok(ToolResult::success(serde_json::to_string(&output).unwrap_or_else(|_| {
                    format!("Image ready for analysis: {}", params.file_path)
                })))
            }
        }
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(get_mime_type(ImageFormat::Png), "image/png");
        assert_eq!(get_mime_type(ImageFormat::Jpeg), "image/jpeg");
        assert_eq!(get_mime_type(ImageFormat::Gif), "image/gif");
        assert_eq!(get_mime_type(ImageFormat::Bmp), "image/bmp");
        assert_eq!(get_mime_type(ImageFormat::WebP), "image/webp");
        assert_eq!(get_mime_type(ImageFormat::Tiff), "image/tiff");
        assert_eq!(get_mime_type(ImageFormat::Pnm), "application/octet-stream");
    }

    #[test]
    fn test_format_name_detection() {
        assert_eq!(format_name_from_image_format(ImageFormat::Png), "PNG");
        assert_eq!(format_name_from_image_format(ImageFormat::Jpeg), "JPEG");
        assert_eq!(format_name_from_image_format(ImageFormat::Gif), "GIF");
        assert_eq!(format_name_from_image_format(ImageFormat::Bmp), "BMP");
        assert_eq!(format_name_from_image_format(ImageFormat::WebP), "WebP");
        assert_eq!(format_name_from_image_format(ImageFormat::Tiff), "TIFF");
    }

    #[test]
    fn test_is_supported_image() {
        assert!(is_supported_image(Path::new("test.png")));
        assert!(is_supported_image(Path::new("test.jpg")));
        assert!(is_supported_image(Path::new("test.jpeg")));
        assert!(is_supported_image(Path::new("test.gif")));
        assert!(is_supported_image(Path::new("test.webp")));
        assert!(!is_supported_image(Path::new("test.txt")));
        assert!(!is_supported_image(Path::new("test.pdf")));
        assert!(!is_supported_image(Path::new("test")));
    }

    #[test]
    fn test_detect_format() {
        assert_eq!(detect_format(Path::new("test.png")), Some(ImageFormat::Png));
        assert_eq!(detect_format(Path::new("test.jpg")), Some(ImageFormat::Jpeg));
        assert_eq!(detect_format(Path::new("test.jpeg")), Some(ImageFormat::Jpeg));
        assert_eq!(detect_format(Path::new("test.gif")), Some(ImageFormat::Gif));
        assert_eq!(detect_format(Path::new("test.webp")), Some(ImageFormat::WebP));
        assert_eq!(detect_format(Path::new("test.txt")), None);
        assert_eq!(detect_format(Path::new("test")), None);
    }

    #[test]
    fn test_image_tool_creation() {
        let tool = ImageTool::new();
        assert_eq!(tool.name(), "Image");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_image_input_schema() {
        let tool = ImageTool::new();
        let schema = tool.input_schema();
        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert!(obj.contains_key("properties"));
    }

    #[test]
    fn test_default_action() {
        assert!(matches!(default_action(), ImageAction::Info));
    }

    #[test]
    fn test_base64_encode_decode() {
        let data = "test image data";
        let encoded = STANDARD.encode(data.as_bytes());
        assert!(!encoded.is_empty());

        // Verify it can be decoded
        let decoded = STANDARD.decode(&encoded).unwrap();
        assert_eq!(decoded, data.as_bytes());
    }

    #[test]
    fn test_image_input_deserialization() {
        let json = r#"{"file_path": "/path/to/image.png"}"#;
        let input: ImageInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.file_path, "/path/to/image.png");
        assert!(matches!(input.action, ImageAction::Info));
    }

    #[test]
    fn test_image_input_with_action() {
        let json = r#"{"file_path": "/path/to/image.jpg", "action": "encode"}"#;
        let input: ImageInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.file_path, "/path/to/image.jpg");
        assert!(matches!(input.action, ImageAction::Encode));
    }

    #[test]
    fn test_image_output_serialization() {
        let output = ImageOutput {
            file_path: "/test/image.png".to_string(),
            dimensions: Some((100, 200)),
            format: Some("png".to_string()),
            size_bytes: 1024,
            base64: Some("base64data".to_string()),
            mime_type: Some("image/png".to_string()),
            description: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("file_path"));
        assert!(json.contains("dimensions"));
        assert!(json.contains("format"));
    }

    #[test]
    fn test_image_tool_is_read_only() {
        let tool = ImageTool::new();
        let input = serde_json::json!({"file_path": "test.png"});
        assert!(tool.is_read_only(&input));
    }
}