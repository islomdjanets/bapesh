use std::env;

use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client as S3Client, config::Credentials, operation::put_object::PutObjectOutput, primitives::ByteStream};

// use reqwest::Client;

// use crate::{AppState, LOSS_QUALITY};

// const PROVIDER: &str = "tigris";

// pub async fn upload_old(client: &Client, endpoint: &str, image_data: Vec<u8>, filename: &str, bucket: &str) -> Result<String, String> {
//     let format = filename.split('.').last().unwrap();
//     if format != "png" && format != "jpg" && format != "jpeg" && format != "webp" {
//         return Err("Invalid image format, only png and jpg are supported".to_string());
//     }
    
//     // println!("convert to webp, if not already");
//     let webp = crate::webp::convert(&image_data, LOSS_QUALITY);
    
//     let format = "webp";

//     let content_type = match format {
//         "png" => "image/png",
//         "jpg" | "jpeg" => "image/jpeg",
//         "webp" => "image/webp",
//         _ => "application/octet-stream",
//     };

//     let unique_key = format!("{}", uuid::Uuid::new_v4());
//     let key = format!("user_images/{}.{}", unique_key, format); // Unique key for the model

//     client.put_object()
//         .bucket(bucket)
//         .key(key.clone())
//         .body(aws_sdk_s3::primitives::ByteStream::from(webp))
//         .content_type(content_type)
//         // .acl(ObjectCannedAcl::PublicRead)
//         .send()
//         .await
//         .map_err(|e| e.to_string())?;


//     // Ok(format!("https://{}.fly.storage.tigris.dev/{}", bucket, key))
//     let public_url = &endpoint;
//     let url = if public_url.ends_with('/') {
//         format!("{}{}", public_url, key)
//     } else {
//         format!("{}/{}", public_url, key)
//     };
//     // Ok(format!("https://{}.fly.storage.tigris.dev/{}", bucket, key))
//     Ok(url)
// }

// pub async fn delete(endpoint_s3: &str, bucket: &str, client: &Client) -> Result<(), String> {
//     let parts: Vec<&str> = endpoint_s3.split('/').collect();
//     if parts.len() < 4 {
//         let err = format!("Invalid S3 URL: {}", endpoint_s3);
//         println!("{}", err);
//         Err(err)
//     } else {
//         // let bucket = parts[2].to_string();
//         // let bucket = parts[2].to_string();
//         let key = parts[3..].join("/");
    
//         println!("Deleting from S3: bucket={}, key={}", bucket, key);
    
//         let delete_result = client
//             .delete_object()
//             .bucket(bucket)
//             .key(key)
//             .send()
//             .await;
    
//         if delete_result.is_err() {
//             println!("Error deleting from S3: {}", delete_result.err().unwrap());
//             Err("Error deleting from S3".to_string())
//         } else {
//             println!("Deleted from S3: {}", endpoint_s3);
//             Ok(())
//         }
//     }
// }

pub async fn get_client_and_endpoint() -> S3Client {

    // Configure Tigris client
    let access_key = env::var("S3_ACCESS_KEY")
        .expect("Missing S3_ACCESS_KEY_ID");

    let secret_key = env::var("S3_SECRET_KEY")
        .expect("Missing S3_SECRET_KEY");

    let account_id= env::var("S3_ACCOUNT_ID")
        .expect("Missing ACCOUNT_ID");

    // let bucket = env::var("BUCKET_NAME")
    //     .map_err(|_| "Missing BUCKET_NAME".to_string())?;

    let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .credentials_provider(Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "r2",
        ))
        .region(Region::new("auto"))
        // 2. USE THIS: simpler string-based configuration
        .endpoint_url(&endpoint) 
        .build();

    let client = S3Client::from_conf(config);

    // let endpoint = env::var("S3_ENDPOINT")
    //     .expect("Missing S3_ENDPOINT");

    // let credentials = Credentials::new(
    //     access_key,
    //     secret_key,
    //     None,
    //     None,
    //     PROVIDER,
    // );

    // let config = aws_config::from_env()
    //     .credentials_provider(credentials)
    //     .endpoint_url(endpoint.clone())
    //     .region("auto")  // Tigris uses a custom region
    //     .load()
    //     .await;
    // let s3client = S3Client::new(&config);

    client
    // (client, endpoint)
}

// pub async fn download_and_upload(client: &Client, endpoint: &str, model_url: &str, task_id: &str, bucket: &str, folder: &str) -> Result<String, String> {
//     // Download the GLB file from Meshy.ai
//     let response = client
//         .get(model_url)
//         .send()
//         .await
//         .map_err(|e| e.to_string())?;

//     let (base_url, query) = if let Some(pos) = model_url.find('?') {
//         (&model_url[..pos], &model_url[pos..])
//     } else {
//         (model_url, "")
//     };

//     let mut format = base_url.split('.').last().unwrap_or("glb");
//     // println!("format: {}, url: {}", format, model_url);

//     let mut content_type = match format {
//         "glb" => "model/gltf-binary",
//         "gltf" => "model/gltf+json",
//         "png" => "image/png",
//         "jpg" | "jpeg" => "image/jpeg",
//         "mp3" => "audio/mpeg",
//         "wav" => "audio/wav",
//         "webp" => "image/webp",
//         _ => "application/octet-stream",
//     };
//     println!("content_type: {}", content_type);

//     if !response.status().is_success() {
//         println!("Download failed: {}", response.status());
//         return Err(format!("Download failed: {}", response.status()));
//     }

//     let mut body = response.bytes().await.map_err(|e| e.to_string())?.to_vec();

//     if format == "glb" {
//         println!("uncompressed size of glb: {}", &body.len());
//         body = crate::glb::compress(&body, LOSS_QUALITY);
//         println!("compressed size of glb: {}", &body.len());
//     }
//     else if format == "png" {
//         // convert to webp
//         format = "webp";
//         content_type = "image/webp";
//         body = crate::webp::convert(&body, LOSS_QUALITY);
//     }

//     // Upload to Tigris
//     let unique_key = task_id;
//     let key = format!("{}/{}.{}", folder, unique_key, format); // Unique key for the model
//     client.put_object()
//         .bucket(bucket)
//         .key(&key)
//         .body(aws_sdk_s3::primitives::ByteStream::from(body))
//         .content_type(content_type)
//         // .acl(ObjectCannedAcl::PublicRead) // Make the object publicly readable
//         .send()
//         .await
//         .map_err(|e| e.to_string())?;

//     // Return the public URL (Tigris provides direct URLs)
//     // Ok(format!("{}/{}/{}", state.s3endpoint, bucket, key))

//     // Ensure no double slashes if public_url ends with /
//     let public_url = &endpoint;
//     let url = if public_url.ends_with('/') {
//         format!("{}{}", public_url, key)
//     } else {
//         format!("{}/{}", public_url, key)
//     };
//     // Ok(format!("https://{}.fly.storage.tigris.dev/{}", bucket, key))
//     Ok(url)
// }

pub async fn upload(
    client: &S3Client,
    buffer: Vec<u8>,
    bucket_name: &str,
    key: &str,
    content_type: &str,
) -> Option<PutObjectOutput> {

    // 4. Send to Cloudflare R2
    let result = client
        .put_object()
        .bucket(bucket_name)
        .key(key)
        .body(ByteStream::from(buffer))
        .content_type(content_type)
        .send()
        .await;
        // .map_err(|e| {
        //     eprintln!("R2 Upload Error: {:?}", e);
        //     // AppError::Internal("Cloud storage failure".into())
        //     e
        // });

    if result.is_err() {
        let e = result.err().unwrap();
        eprintln!("R2 Upload Error: {:?}", e);
        return None;
    }

    Some(result.unwrap())
}