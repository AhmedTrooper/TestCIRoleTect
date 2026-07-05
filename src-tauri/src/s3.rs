use aws_sdk_s3 as s3;

pub struct S3Config {
    pub endpoint_url: String,
    pub bucket_name: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub force_path_style: bool,
}

pub async fn build_s3_client(config: &S3Config) -> Result<s3::Client, String> {
    let credentials = aws_sdk_s3::config::Credentials::new(
        &config.access_key_id,
        &config.secret_access_key,
        None, // session token
        None, // expiry
        "roletect",
    );

    let s3_config = s3::config::Builder::new()
        .endpoint_url(&config.endpoint_url)
        .region(s3::config::Region::new(config.region.clone()))
        .credentials_provider(credentials)
        .force_path_style(config.force_path_style)
        .behavior_version(s3::config::BehaviorVersion::latest())
        .build();

    Ok(s3::Client::from_conf(s3_config))
}

pub async fn upload_backup(
    client: &s3::Client,
    bucket: &str,
    backup_data: &str, // serialized JSON
) -> Result<String, String> {
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let key = format!("roletect_{}.json", timestamp);

    client.put_object()
        .bucket(bucket)
        .key(&key)
        .body(s3::primitives::ByteStream::from(backup_data.as_bytes().to_vec()))
        .content_type("application/json")
        .send()
        .await
        .map_err(|e| format!("S3 upload error: {}", e))?;

    Ok(key)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BackupEntry {
    pub key: String,
    pub size: i64,
    pub last_modified: String,
}

pub async fn list_backups(
    client: &s3::Client,
    bucket: &str,
    limit: i32,
) -> Result<Vec<BackupEntry>, String> {
    let response = client.list_objects_v2()
        .bucket(bucket)
        .prefix("roletect_")
        .send()
        .await
        .map_err(|e| format!("S3 list error: {}", e))?;

    let mut entries: Vec<BackupEntry> = response.contents()
        .iter()
        .filter_map(|obj| {
            let key = obj.key()?.to_string();
            let size = obj.size().unwrap_or(0);
            let last_modified = obj.last_modified()
                .map(|t| t.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime).unwrap_or_default())
                .unwrap_or_default();
            Some(BackupEntry { key, size, last_modified })
        })
        .collect();

    entries.sort_by(|a, b| b.key.cmp(&a.key));
    entries.truncate(limit as usize);

    Ok(entries)
}

pub async fn download_backup(
    client: &s3::Client,
    bucket: &str,
    key: &str,
) -> Result<String, String> {
    let response = client.get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| format!("S3 download error: {}", e))?;

    let bytes = response.body.collect().await
        .map_err(|e| format!("S3 read error: {}", e))?;

    String::from_utf8(bytes.into_bytes().to_vec())
        .map_err(|e| format!("UTF-8 decode error: {}", e))
}

pub async fn test_connection(
    client: &s3::Client,
    bucket: &str,
) -> Result<(), String> {
    client.head_bucket()
        .bucket(bucket)
        .send()
        .await
        .map_err(|e| format!("S3 connection test failed: {}", e))?;
    Ok(())
}
