use crate::resource::ResourceContents;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Optional annotations for the client. The client can use annotations to inform how objects are used or displayed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// Describes who the intended customer of this object is. It can include multiple entries to
    /// indicate content useful for multiple audiences (e.g., `["user", "assistant"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,
    /// Describes how important this data is for operating the server.
    /// A value of 1 means "most important," and indicates that the data is effectively required,
    /// while 0 means "least important," and indicates that the data is entirely optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// The text content of the message.
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// The base64 encoded image data.
    pub data: String,
    /// The MIME type of the image.
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// The contents of a resource, embedded into a prompt or tool call result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedResource {
    pub resource: ResourceContents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioContent {
    /// The base64 encoded audio data.
    pub data: String,
    /// The MIME type of the audio.
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
    Resource(EmbeddedResource),
    Audio(AudioContent),
}

impl Content {
    pub fn text<S: Into<String>>(text: S) -> Self {
        Content::Text(TextContent {
            text: text.into(),
            annotations: None,
        })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        Content::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
        })
    }

    pub fn resource(resource: ResourceContents) -> Self {
        Content::Resource(EmbeddedResource {
            resource,
            annotations: None,
        })
    }

    pub fn embedded_text<S: Into<String>, T: Into<String>>(uri: S, content: T) -> Self {
        Content::Resource(EmbeddedResource {
            resource: ResourceContents::TextResourceContents {
                uri: uri.into(),
                mime_type: Some("text/plain".to_string()),
                text: content.into(),
            },
            annotations: None,
        })
    }

    pub fn audio<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        Content::Audio(AudioContent {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
        })
    }

    /// Set the audience for the content
    pub fn with_audience(mut self, audience: Vec<Role>) -> Self {
        let annotations = match &mut self {
            Content::Text(text) => &mut text.annotations,
            Content::Image(image) => &mut image.annotations,
            Content::Resource(resource) => &mut resource.annotations,
            Content::Audio(audio) => &mut audio.annotations,
        };
        *annotations = Some(match annotations.take() {
            Some(mut a) => {
                a.audience = Some(audience);
                a
            }
            None => Annotations {
                audience: Some(audience),
                priority: None,
            },
        });
        self
    }

    /// Get the audience if set
    pub fn audience(&self) -> Option<&Vec<Role>> {
        match self {
            Content::Text(text) => text.annotations.as_ref().and_then(|a| a.audience.as_ref()),
            Content::Image(image) => image.annotations.as_ref().and_then(|a| a.audience.as_ref()),
            Content::Resource(resource) => resource
                .annotations
                .as_ref()
                .and_then(|a| a.audience.as_ref()),
            Content::Audio(audio) => audio.annotations.as_ref().and_then(|a| a.audience.as_ref()),
        }
    }

    /// Get an unannotated copy of the content
    pub fn unannotated(&self) -> Self {
        match self {
            Content::Text(text) => Content::text(text.text.clone()),
            Content::Image(image) => Content::image(image.data.clone(), image.mime_type.clone()),
            Content::Resource(resource) => Content::resource(resource.resource.clone()),
            Content::Audio(audio) => Content::audio(audio.data.clone(), audio.mime_type.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audience_annotation() {
        let content = Content::text("hello");
        assert_eq!(content.audience(), None);

        let content = Content::text("hello").with_audience(vec![Role::User]);
        assert_eq!(content.audience(), Some(&vec![Role::User]));

        let content = Content::image("data", "image/png").with_audience(vec![Role::User]);
        assert_eq!(content.audience(), Some(&vec![Role::User]));
    }

    #[test]
    fn test_audience_annotation_override() {
        let content = Content::text("hello")
            .with_audience(vec![Role::User])
            .with_audience(vec![Role::Assistant]);

        assert_eq!(content.audience(), Some(&vec![Role::Assistant]));
    }

    #[test]
    fn test_audience_preserved_in_annotations() {
        let text_content = Content::text("hello").with_audience(vec![Role::User]);

        match &text_content {
            Content::Text(TextContent { annotations, .. }) => {
                assert!(annotations.is_some());
                let ann = annotations.as_ref().unwrap();
                assert_eq!(ann.audience, Some(vec![Role::User]));
            }
            _ => panic!("unexpected content type"),
        }
    }

    #[test]
    fn test_unannotated() {
        let content = Content::text("hello").with_audience(vec![Role::User]);
        let unannotated = content.unannotated();
        assert_eq!(unannotated.audience(), None);
    }
}
