use crate::config::{API_BASE_URL, API_VERSION, ENDPOINT_UPLOAD_URL};
use crate::error::{Error, Result};
use crate::types::{S3UploadResponse, UploadFile, UploadUrlRequest, UploadUrlResponse};
use regex::Regex;
use rquest::Client as HttpClient;

pub(crate) async fn upload_file(http: &HttpClient, file: &UploadFile) -> Result<String> {
    let content_type = mime_guess::from_path(file.filename())
        .first_or_octet_stream()
        .to_string();

    let upload_url_resp: UploadUrlResponse = http
        .post(format!("{}{}", API_BASE_URL, ENDPOINT_UPLOAD_URL))
        .query(&[("version", API_VERSION), ("source", "default")])
        .json(&UploadUrlRequest {
            content_type: content_type.clone(),
            file_size: file.len(),
            filename: file.filename().to_string(),
            force_image: false,
            source: "default".to_string(),
        })
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Upload(format!("Failed to get upload URL: {}", e)))?
        .json()
        .await?;

    let mut form = rquest::multipart::Form::new();
    for (key, value) in &upload_url_resp.fields {
        form = form.text(key.clone(), value.clone());
    }

    let file_part = rquest::multipart::Part::bytes(file.as_bytes().to_vec())
        .file_name(file.filename().to_string())
        .mime_str(&content_type)
        .map_err(|e| Error::Upload(format!("Invalid MIME type: {}", e)))?;
    form = form.part("file", file_part);

    let upload_resp = http
        .post(&upload_url_resp.s3_bucket_url)
        .multipart(form)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::Upload(format!("S3 upload failed: {}", e)))?;

    let uploaded_url = if upload_url_resp.s3_object_url.contains("image/upload") {
        let s3_resp: S3UploadResponse = upload_resp.json().await?;
        let secure_url = s3_resp
            .secure_url
            .ok_or_else(|| Error::Upload("Missing secure_url in S3 response".to_string()))?;

        let re = Regex::new(r"/private/s--.*?--/v\d+/user_uploads/")
            .map_err(|e| Error::Upload(format!("Regex error: {}", e)))?;
        re.replace(&secure_url, "/private/user_uploads/").to_string()
    } else {
        upload_url_resp.s3_object_url
    };

    Ok(uploaded_url)
}
