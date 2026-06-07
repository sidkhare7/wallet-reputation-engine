use actix_web::{HttpResponse, ResponseError};
use thiserror::Error;
use actix_multipart::MultipartError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Multipart error: {0}")]
    MultipartError(#[from] MultipartError),
    #[error("Custom error: {0}")]
    CustomError(String),
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::JsonError(err) => {
                HttpResponse::BadRequest().json(format!("Invalid JSON: {}", err))
            }
            AppError::Utf8Error(err) => {
                HttpResponse::BadRequest().json(format!("Invalid UTF-8: {}", err))
            }
            AppError::Io(err) => {
                HttpResponse::InternalServerError().json(format!("IO error: {}", err))
            }
            AppError::Image(err) => {
                HttpResponse::InternalServerError().json(format!("Image processing error: {}", err))
            }
            AppError::MultipartError(err) => {
                HttpResponse::InternalServerError().json(format!("Multipart  error: {}", err))
            }
            AppError::CustomError(msg) => {
                HttpResponse::InternalServerError().json(format!("Error: {}", msg))
            }
        }
    }
}
