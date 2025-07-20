use crate::{
    api::{responses::HealthResponse, Error},
    bird::Birdc,
    config,
};

pub async fn retrieve() -> Result<HealthResponse, Error> {
    let birdc = Birdc::default();
    let version = crate::version();
    let bird_socket = config::get_birdc_socket();

    match birdc.show_status().await {
        Ok(bird_status) => Ok(HealthResponse {
            status: "ok".to_string(),
            version,
            bird_socket,
            bird_status: Some(bird_status),
            error: None,
            bird_error: None,
        }),
        Err(e) => Ok(HealthResponse {
            status: "error".to_string(),
            version,
            bird_socket,
            bird_status: None,
            error: Some("Could not connect to bird daemon".to_string()),
            bird_error: Some(e.to_string()),
        }),
    }
}
