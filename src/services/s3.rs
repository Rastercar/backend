use crate::config::{app_config, aws_config};
use aws_sdk_s3 as s3;
use axum::body::Bytes;
use http::Response;
use s3::{
    error::SdkError,
    operation::put_object::{PutObjectError, PutObjectOutput},
    primitives::SdkBody,
    Client,
};

pub struct S3Key {
    /// the "folder" a file using this key will be stored into
    ///
    /// in practice this determines the middle of the path
    pub folder: String,

    /// filename with extension, eg: `profile-pic.jpeg`
    pub filename: String,
}

impl From<S3Key> for String {
    fn from(v: S3Key) -> Self {
        format!(
            "{}/{}/{}",
            app_config().tenant_slug.clone(),
            v.folder,
            v.filename
        )
    }
}

#[derive(Clone)]
pub struct S3 {
    client: Client,
    uploads_bucket: String,
}

impl S3 {
    pub async fn new() -> Self {
        Self {
            client: s3::Client::new(aws_config().await),
            uploads_bucket: app_config().aws_uploads_bucket_name.clone(),
        }
    }

    pub async fn upload(
        &self,
        key: S3Key,
        data: Bytes,
    ) -> Result<PutObjectOutput, SdkError<PutObjectError, Response<SdkBody>>> {
        self.client
            .put_object()
            .bucket(&self.uploads_bucket)
            .key(String::from(key))
            .body(data.into())
            .send()
            .await
    }
}
