use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::content::Annotations;

/// A known resource that the server is capable of reading. This struct provides metadata about
/// resources in list calls. Contents are provided by `ResourceContents`.
///
/// In contrast to `EmbeddedResource`, this struct is provided independently to a client (when they
/// list available resources), whereas EmbeddedResource is embedded into a prompt or tool call result.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// The URI of this resource. Common schemes include `https`, `file`, and `git`.
    pub uri: String,
    /// A human-readable name for this resource. This can be used by clients to populate UI elements.
    pub name: String,
    /// Optional description of what this resource represents.
    /// This can be used by clients to improve the LLM's understanding of available resources. It
    /// can be thought of like a "hint" to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional annotations for the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    /// The size of the raw resource content, in bytes (i.e., before base64 encoding or an
    /// tokenization), if known. This can be used by Hosts to display file sizes and estimate
    /// context window usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

/// The contents of a resource, identified by the `uri` field in `Resource`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ResourceContents {
    /// Text resources contain UTF-8 encoded text data. They're most suitable for things like
    /// source code, config or log files, JSON/XML data or plain text.
    TextResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        text: String,
    },
    /// Binary resources contain raw binary data encoded in base64. They're most suitable for
    /// things like images, audio, video, or other non-text, binary data.
    BlobResourceContents {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        blob: String,
    },
}

// TODO: Consider a ResourceBuilder
impl Resource {
    /// Creates a new Resource from a URI.
    ///
    /// The mime type is optional, and can be provided if known.
    /// The name is optional, and will be extracted from the URI if not provided.
    pub fn new<S: Into<String>>(
        uri: S,
        mime_type: Option<String>,
        name: Option<String>,
        annotations: Option<Annotations>,
    ) -> Result<Self, ResourceError> {
        let uri = uri.into();
        // Constructing the URL validates the URI scheme.
        let url =
            Url::parse(&uri).map_err(|e| ResourceError::InvalidUri(uri.clone(), e.to_string()))?;

        // Extract name from the path component of the URI if not provided.
        let name = match name {
            Some(n) => n,
            None => url
                .path_segments()
                .and_then(|mut segments| segments.next_back())
                .ok_or_else(|| {
                    ResourceError::InvalidUri(
                        uri.clone(),
                        "Could not extract name from URI path".to_string(),
                    )
                })?
                .to_string(),
        };

        Ok(Self {
            uri,
            name,
            description: None,
            mime_type,
            annotations,
            size: None,
        })
    }
}

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Invalid URI: {0}. Error: {1}")]
    InvalidUri(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_new_resource_with_file_uri() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "test content")?;

        let uri = Url::from_file_path(temp_file.path())
            .map_err(|_| anyhow!("Invalid file path"))?
            .to_string();

        let resource = Resource::new(&uri, Some("text/plain".to_string()), None, None)?;
        assert!(resource.uri.starts_with("file:///"));
        assert_eq!(resource.mime_type, Some("text/plain".to_string()));

        Ok(())
    }

    #[test]
    fn test_mime_type_validation() -> Result<()> {
        // Test valid mime types
        let resource = Resource::new(
            "file:///test.txt",
            Some("text/plain".to_string()),
            None,
            None,
        )?;
        assert_eq!(resource.mime_type, Some("text/plain".to_string()));

        let resource = Resource::new(
            "file:///test.png",
            Some("image/png".to_string()),
            None,
            None,
        )?;
        assert_eq!(resource.mime_type, Some("image/png".to_string()));

        // We don't validate the mime type, so it will be "invalid"
        let resource = Resource::new("file:///test.txt", Some("invalid".to_string()), None, None)?;
        assert_eq!(resource.mime_type, Some("invalid".to_string()));

        // mime type is optional, so it will be None
        let resource = Resource::new("file:///test.txt", None, None, None)?;
        assert_eq!(resource.mime_type, None);

        Ok(())
    }

    #[test]
    fn test_invalid_uri() {
        let result = Resource::new(
            "not-a-uri",
            Some("text/plain".to_string()),
            Some("test.txt".to_string()),
            None,
        );
        assert!(result.is_err());
    }
}
