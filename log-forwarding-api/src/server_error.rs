use actix_web::{
    HttpResponse, error,
    http::{StatusCode, header::ContentType},
};
use derive_more::derive::{Display, Error};

#[derive(Debug, Display, Error)]
#[display("Error: **{code}** \n {message} \n\n Details: {additional_information}")]
pub struct ServerError {
    pub code: StatusCode,
    pub message: String,
    pub additional_information: String,
}

impl error::ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(format!("Message: {}", self.message))
    }

    fn status_code(&self) -> StatusCode {
        self.code
    }
}
