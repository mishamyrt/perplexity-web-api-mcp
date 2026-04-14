use crate::config::{
    API_BASE_URL, API_REFERER, API_VERSION, ENDPOINT_ATTACHMENT_PROCESSING,
    ENDPOINT_BATCH_UPLOAD_URL,
};
use crate::error::{Error, Result};
use crate::http::ensure_success_response;
use crate::types::{
    BatchUploadFileInfo, BatchUploadFileMeta, BatchUploadFileResponse, BatchUploadFileResults,
    UploadFile,
};
use rquest::Client as HttpClient;
use rquest::header::{HeaderValue, ORIGIN, REFERER};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

const PERPLEXITY_ORIGIN: HeaderValue = HeaderValue::from_static(API_BASE_URL);
const PERPLEXITY_REFERER: HeaderValue = HeaderValue::from_static(API_REFERER);

#[derive(Serialize)]
struct BatchUploadUrlRequest {
    files: HashMap<String, BatchUploadFileInfo>,
}

#[derive(Serialize)]
struct ProcessingSubscribeRequest {
    file_uuids: Vec<String>,
}

/// Uploads multiple files in one batch using the presigned-POST flow:
/// 1. Obtain presigned S3 form fields for all files in a single request
/// 2. Upload every file to S3 in parallel
/// 3. Wait for server-side attachment processing of all files via SSE
///
/// Returns one `s3_object_url` per file (same order as input).
pub(crate) async fn upload_files(
    http: &HttpClient,
    files: &[&UploadFile],
    timeout: Duration,
) -> Result<Vec<String>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    // Build client-UUID -> file mapping so we can correlate response entries
    // back to the original files while preserving input order.
    let keyed: Vec<(String, &UploadFile)> =
        files.iter().map(|f| (Uuid::new_v4().to_string(), *f)).collect();

    // Step 1: obtain presigned upload fields for all files at once
    let batch_resp = request_upload_urls(http, &keyed, timeout).await?;

    // Collect per-file metadata preserving original order
    let file_metas: Vec<(BatchUploadFileMeta, &BatchUploadFileResults, &UploadFile)> = keyed
        .iter()
        .map(|(client_uuid, file)| {
            let results =
                batch_resp.results.get(client_uuid).ok_or(Error::MissingUploadResponse)?;
            let meta = BatchUploadFileMeta {
                s3_object_url: results.s3_object_url.clone(),
                uuid: results.file_uuid.clone(),
            };
            Ok((meta, results, *file))
        })
        .collect::<Result<Vec<_>>>()?;

    // Step 2: upload every file to S3 in parallel
    let s3_futures: Vec<_> = file_metas
        .iter()
        .map(|(_, results, file)| upload_to_s3(http, results, file, timeout))
        .collect();

    let s3_results = futures_util::future::join_all(s3_futures).await;
    for res in s3_results {
        res?;
    }

    // Step 3: wait for server-side attachment processing
    let file_uuids: Vec<String> = file_metas.iter().map(|(m, _, _)| m.uuid.clone()).collect();
    wait_for_processing(http, &file_uuids, timeout).await?;

    let urls = file_metas.into_iter().map(|(m, _, _)| m.s3_object_url).collect();
    Ok(urls)
}

/// Step 1: single batch request to obtain presigned S3 credentials for all files.
async fn request_upload_urls(
    http: &HttpClient,
    keyed: &[(String, &UploadFile)],
    timeout: Duration,
) -> Result<BatchUploadFileResponse> {
    let mut files = HashMap::with_capacity(keyed.len());
    for (client_uuid, file) in keyed {
        let content_type =
            mime_guess::from_path(file.filename()).first_or_octet_stream().to_string();
        files.insert(
            client_uuid.clone(),
            BatchUploadFileInfo {
                filename: file.filename().to_string(),
                content_type,
                source: "default".to_string(),
                file_size: file.len(),
                force_image: false,
                skip_parsing: false,
                persistent_upload: false,
            },
        );
    }

    let full_url = format!(
        "{API_BASE_URL}{ENDPOINT_BATCH_UPLOAD_URL}?version={API_VERSION}&source=default"
    );

    let fut = http
        .post(&full_url)
        .header(ORIGIN, PERPLEXITY_ORIGIN)
        .header(REFERER, PERPLEXITY_REFERER)
        .header("x-app-apiclient", "default")
        .header("x-app-apiversion", API_VERSION)
        .json(&BatchUploadUrlRequest { files })
        .send();

    let resp = tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| Error::Timeout(timeout))?
        .map_err(Error::UploadRequest)?;
    let resp = ensure_success_response(resp)?;

    resp.json::<BatchUploadFileResponse>().await.map_err(Error::UploadRequest)
}

/// Step 2: upload a single file to S3 using the presigned form fields.
async fn upload_to_s3(
    http: &HttpClient,
    results: &BatchUploadFileResults,
    file: &UploadFile,
    timeout: Duration,
) -> Result<()> {
    let content_type =
        mime_guess::from_path(file.filename()).first_or_octet_stream().to_string();

    let mut form = rquest::multipart::Form::new();
    for (key, value) in &results.fields {
        form = form.text(key.clone(), value.clone());
    }

    let raw: Vec<u8> = match file {
        UploadFile::Binary { data, .. } => data.to_vec(),
        UploadFile::Text { content, .. } => content.as_bytes().to_vec(),
    };
    let file_part = rquest::multipart::Part::bytes(raw)
        .file_name(file.filename().to_string())
        .mime_str(&content_type)
        .map_err(|e| Error::InvalidMimeType(e.to_string()))?;
    form = form.part("file", file_part);

    let fut = http.post(&results.s3_bucket_url).multipart(form).send();

    let response = tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| Error::Timeout(timeout))?
        .map_err(Error::UploadRequest)?;
    response.error_for_status().map_err(Error::S3UploadFailed)?;

    Ok(())
}

/// Step 3: subscribe to the attachment-processing SSE endpoint and wait
/// until the server finishes processing all files.
async fn wait_for_processing(
    http: &HttpClient,
    file_uuids: &[String],
    timeout: Duration,
) -> Result<()> {
    let body = ProcessingSubscribeRequest { file_uuids: file_uuids.to_vec() };

    let sse_fut = http
        .post(format!("{API_BASE_URL}{ENDPOINT_ATTACHMENT_PROCESSING}"))
        .header("Accept", "text/event-stream")
        .header(ORIGIN, PERPLEXITY_ORIGIN)
        .header(REFERER, PERPLEXITY_REFERER)
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header(
            "x-perplexity-request-endpoint",
            format!("{API_BASE_URL}{ENDPOINT_ATTACHMENT_PROCESSING}"),
        )
        .header("x-perplexity-request-reason", "ask-input-inner-home")
        .header("x-perplexity-request-try-number", "1")
        .json(&body)
        .send();

    let resp = tokio::time::timeout(timeout, sse_fut)
        .await
        .map_err(|_| Error::Timeout(timeout))?
        .map_err(Error::UploadRequest)?;
    let resp = ensure_success_response(resp)?;

    let body_fut = resp.bytes();
    let _: bytes::Bytes = tokio::time::timeout(timeout, body_fut)
        .await
        .map_err(|_| Error::Timeout(timeout))?
        .map_err(Error::AttachmentProcessing)?;

    Ok(())
}
