use actix_web::{get, post, web, HttpResponse, Responder};
use crate::ipn_metadata::{IpTxtRecord, IpTxtType};
use crate::ipn_dht::{store_txt_record, retrieve_txt_records};

#[get("/txt/{handle}")]
async fn list_txt_records(handle: web::Path<String>) -> impl Responder {
    // Retrieve all TXT records for a handle
    match retrieve_txt_records(&handle, None) {
        Ok(records) => HttpResponse::Ok().json(records),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/txt/{handle}/{record_type}")]
async fn filter_txt_records(info: web::Path<(String, String)>) -> impl Responder {
    // Retrieve TXT records filtered by record type
    let (handle, record_type_str) = info.into_inner();
    let record_type = match record_type_str.as_str() {
        "FileDescription" => IpTxtType::FileDescription,
        "ServerInfo" => IpTxtType::ServerInfo,
        "DNSLikeRecord" => IpTxtType::DNSLikeRecord,
        "ProofBinding" => IpTxtType::ProofBinding,
        _ => return HttpResponse::BadRequest().finish(),
    };
    match retrieve_txt_records(&handle, Some(record_type)) {
        Ok(records) => HttpResponse::Ok().json(records),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/txt/announce")]
async fn announce_txt_record(record: web::Json<IpTxtRecord>) -> impl Responder {
    // Store a new TXT record
    match store_txt_record(&record) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
} 