use clap::{App, Arg, SubCommand};
use crate::ipn_metadata::{IpTxtRecord, IpTxtType};
use crate::ipn_dht::{store_txt_record, retrieve_txt_records};
use crate::node_config::NodeConfig;

fn main() {
    let matches = App::new("IPPAN CLI")
        .version("1.0")
        .author("IPPAN Devs")
        .about("Manage IPPAN TXT Records")
        .subcommand(SubCommand::with_name("txt")
            .about("Manage TXT records")
            .subcommand(SubCommand::with_name("publish")
                .about("Publish a TXT record")
                .arg(Arg::with_name("handle")
                    .help("The handle to publish the record for")
                    .required(true))
                .arg(Arg::with_name("record_type")
                    .help("The type of TXT record")
                    .required(true))
                .arg(Arg::with_name("content")
                    .help("The content of the TXT record")
                    .required(true)))
            .subcommand(SubCommand::with_name("list")
                .about("List TXT records for a handle")
                .arg(Arg::with_name("handle")
                    .help("The handle to list records for")
                    .required(true))))
        .subcommand(SubCommand::with_name("archive")
            .about("Manage archive mode")
            .subcommand(SubCommand::with_name("status")
                .about("Check archive mode status"))
            .subcommand(SubCommand::with_name("push-now")
                .about("Push transactions now")))
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("txt") {
        if let Some(matches) = matches.subcommand_matches("publish") {
            let handle = matches.value_of("handle").unwrap();
            let record_type_str = matches.value_of("record_type").unwrap();
            let content = matches.value_of("content").unwrap();
            let record_type = match record_type_str {
                "FileDescription" => IpTxtType::FileDescription,
                "ServerInfo" => IpTxtType::ServerInfo,
                "DNSLikeRecord" => IpTxtType::DNSLikeRecord,
                "ProofBinding" => IpTxtType::ProofBinding,
                _ => {
                    eprintln!("Invalid record type");
                    return;
                }
            };
            let record = IpTxtRecord::new(record_type, handle.to_string(), content.to_string(), HashTimer::now(), "signature_placeholder".to_string());
            if let Err(_) = store_txt_record(&record) {
                eprintln!("Failed to store TXT record");
            }
        } else if let Some(matches) = matches.subcommand_matches("list") {
            let handle = matches.value_of("handle").unwrap();
            if let Ok(records) = retrieve_txt_records(handle, None) {
                for record in records {
                    println!("TXT Record: {}", record.content);
                }
            } else {
                eprintln!("Failed to retrieve TXT records");
            }
        }
    }

    if let Some(matches) = matches.subcommand_matches("archive") {
        if matches.subcommand_matches("status").is_some() {
            // Implement logic to check archive mode status
            println!("Archive mode is active");
        } else if matches.subcommand_matches("push-now").is_some() {
            // Implement logic to push transactions immediately
            println!("Pushing transactions now...");
        }
    }
} 