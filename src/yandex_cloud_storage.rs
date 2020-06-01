use std::error::Error;
use std::fs::File;
use std::io;
use std::path::Path;

use rusoto_core::RusotoError;
use rusoto_s3::{DeleteObjectRequest, GetObjectRequest, ListObjectsV2Request, PutObjectRequest, S3, S3Client, StreamingBody};
use tokio::runtime::Runtime;

use lazy_static::lazy_static;

use crate::util;
use crate::util::{new_buf_reader, new_buf_writer};

#[cfg(test)]
mod tests;

const BUCKET: &str = "factorio-servers-statistics";

struct YandexCloud {
    s3_client: S3Client,
}

impl YandexCloud {
    pub fn new() -> Self {
        let credentials_provider = rusoto_credential::EnvironmentProvider::default();
        let region = rusoto_core::Region::Custom {
            name: String::from("us-east-1"),
            endpoint: String::from("storage.yandexcloud.net"),
        };
        let s3_client = S3Client::new_with(rusoto_core::HttpClient::new().unwrap(), credentials_provider, region);

        YandexCloud { s3_client }
    }
}

lazy_static! {
    static ref YANDEX_CLOUD: YandexCloud = YandexCloud::new();
}

pub fn list_bucket(path: &str) -> Vec<String> {
    let mut path = path.to_owned();
    if !path.ends_with('/') {
        path.push('/');
    }

    let list_request = ListObjectsV2Request {
        bucket: "factorio-servers-statistics".to_owned(),
        prefix: Some(path.to_owned()),
        ..Default::default()
    };
    let mut runtime = Runtime::new().unwrap();
    let result = runtime.block_on(YANDEX_CLOUD.s3_client.list_objects_v2(list_request))
        .unwrap_or_else(|err| panic!(format!(
            "[error] [yandex_cloud] Can't list bucket `{}`: {}", path, err)));

    result.contents.unwrap_or_default().into_iter()
        .filter_map(|object| object.key)
        .filter(|key| key != &path)
        .collect()
}

async fn get_rusoto_streaming_body(filename: &Path) -> (StreamingBody, u64) {
    // https://users.rust-lang.org/t/turning-a-file-into-futures-stream/33480/6
    // (old) https://github.com/rusoto/rusoto/issues/1509
    // (old) https://stackoverflow.com/a/57812269/5812238

    use bytes::BytesMut;
    use futures::TryStreamExt;
    use tokio_util::codec::{BytesCodec, FramedRead};

    let file = tokio::fs::File::open(filename).await.unwrap();
    let file_length = file.metadata().await.unwrap().len();

    let stream = FramedRead::new(file, BytesCodec::new()).map_ok(BytesMut::freeze);
    let streaming_body = StreamingBody::new(stream);
    (streaming_body, file_length)
}

async fn upload_async(path: &str, filename: &Path, content_type: &str) -> Result<(), Box<dyn Error>> {
    let (streaming_body, file_length) = get_rusoto_streaming_body(filename).await;
    let put_request = PutObjectRequest {
        bucket: BUCKET.to_owned(),
        key: path.to_string(),
        body: Some(streaming_body),
        // Yandex.Cloud стал требовать заголовок Content-Length 07.10.2019 ~16:00 UTC
        content_length: Some(file_length as i64),
        content_type: Some(content_type.to_owned()),
        ..Default::default()
    };

    let result = YANDEX_CLOUD.s3_client.put_object(put_request).await;
    if let Err(err) = &result {
        if let RusotoError::Unknown(err) = err {
            eprintln!("[error] [yandex_cloud] Can't upload: {:?}", err.body);
        } else {
            eprintln!("[error] [yandex_cloud] Can't upload: {}", err);
        }
    }
    result?;
    Ok(())
}

fn upload(path: &str, filename: &Path, content_type: &str) -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new().unwrap();
    runtime.block_on(upload_async(path, filename, content_type))
}

pub fn upload_with_retries(path: &str, filename: &Path, content_type: &str, number_retries: usize) {
    util::run_with_retries(
        number_retries,
        || upload(path, filename, content_type),
        |retry_index, response| {
            eprintln!("[warn]  [yandex_cloud] upload failed (retry_index = {}):\n\tpath: {}\n\terror message: {}",
                      retry_index, path, response);
        },
    ).unwrap();
}

/// если создавать runtime внутри функции download,
/// то он будет уничтожен (drop) после выхода из функции
/// и поэтому почему-то файл будет обрезан до первых ~4-8КБ
pub fn download(runtime: &mut Runtime, path: &str) -> Result<impl io::Read, Box<dyn Error>> {
    let get_request = GetObjectRequest {
        bucket: BUCKET.to_owned(),
        key: path.to_owned(),
        ..Default::default()
    };

    let result = runtime.block_on(YANDEX_CLOUD.s3_client.get_object(get_request))?;
    let reader = result.body.ok_or("no body")?.into_blocking_read();
    let reader = new_buf_reader(reader);  // todo is it necessary?
    Ok(reader)
}

pub fn delete(path: &str) -> Result<(), Box<dyn Error>> {
    let delete_request = DeleteObjectRequest {
        bucket: BUCKET.to_owned(),
        key: path.to_owned(),
        ..Default::default()
    };

    let mut runtime = Runtime::new().unwrap();
    runtime.block_on(YANDEX_CLOUD.s3_client.delete_object(delete_request))?;
    Ok(())
}

pub fn download_to_file(path: &str, filename: &Path) -> Result<(), Box<dyn Error>> {
    let mut runtime = Runtime::new().unwrap();
    let mut reader = download(&mut runtime, path)?;
    let writer = File::create(filename)?;
    let mut writer = new_buf_writer(writer);

    std::io::copy(&mut reader, &mut writer)?;
    Ok(())
}
