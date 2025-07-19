pub fn display_file_description(handle: &str) {
    // Fetch and display file descriptions for a handle
    if let Ok(records) = retrieve_txt_records(handle, Some(IpTxtType::FileDescription)) {
        for record in records {
            println!("File Description: {}", record.content);
        }
    }
}

pub fn display_server_info(handle: &str) {
    // Fetch and display server info for a handle
    if let Ok(records) = retrieve_txt_records(handle, Some(IpTxtType::ServerInfo)) {
        for record in records {
            println!("Server Info: {}", record.content);
        }
    }
} 