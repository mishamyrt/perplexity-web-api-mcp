use crate::config::{API_BASE_URL, API_VERSION, ENDPOINT_UPLOAD_URL};
use crate::error::{Error, Result};
use crate::types::{S3UploadResponse, UploadFile, UploadUrlRequest, UploadUrlResponse};
use regex::Regex;
use rquest::Client as HttpClient;
use std::sync::LazyLock;
use std::time::Duration;

static S3_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"/private/s--.*?--/v\d+/user_uploads/").expect("Invalid S3 URL regex pattern")
});

pub(crate) async fn upload_file(
    http: &HttpClient,
    file: &UploadFile,
    timeout: Duration,
) -> Result<String> {
    let content_type =
        mime_guess::from_path(file.filename()).first_or_octet_stream().to_string();

    let upload_url_fut = http
        .post(format!("{}{}", API_BASE_URL, ENDPOINT_UPLOAD_URL))
        .query(&[("version", API_VERSION), ("source", "default")])
        .json(&UploadUrlRequest {
            content_type: content_type.clone(),
            file_size: file.len(),
            filename: file.filename().to_string(),
            force_image: false,
            source: "default".to_string(),
        })
        .send();

    let upload_url_resp: UploadUrlResponse = tokio::time::timeout(timeout, upload_url_fut)
        .await
        .map_err(|_| Error::Timeout(timeout))?
        .map_err(Error::Http)?
        .error_for_status()
        .map_err(|e| Error::UploadUrlFailed(e.to_string()))?
        .json()
        .await?;

    let mut form = rquest::multipart::Form::new();
    for (key, value) in &upload_url_resp.fields {
        form = form.text(key.clone(), value.clone());
    }

    let file_part = rquest::multipart::Part::bytes(file.as_bytes().to_vec())
        .file_name(file.filename().to_string())
        .mime_str(&content_type)
        .map_err(|e| Error::InvalidMimeType(e.to_string()))?;
    form = form.part("file", file_part);

    let s3_upload_fut = http.post(&upload_url_resp.s3_bucket_url).multipart(form).send();

    let upload_resp = tokio::time::timeout(timeout, s3_upload_fut)
        .await
        .map_err(|_| Error::Timeout(timeout))?
        .map_err(Error::Http)?
        .error_for_status()
        .map_err(|e| Error::S3UploadFailed(e.to_string()))?;

    let uploaded_url = if upload_url_resp.s3_object_url.contains("image/upload") {
        let s3_resp: S3UploadResponse = upload_resp.json().await?;
        let secure_url = s3_resp.secure_url.ok_or(Error::MissingSecureUrl)?;

        S3_URL_REGEX.replace(&secure_url, "/private/user_uploads/").to_string()
    } else {
        upload_url_resp.s3_object_url
    };

    Ok(uploaded_url)
}
