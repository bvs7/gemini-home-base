use std::fs;

use mqtt::AsyncClient;
use paho_mqtt as mqtt;
use reqwest::{Client, Error};
use serde::Deserialize;

const REFRESH_HOST: &str = "https://oauth2.googleapis.com/token";
const REFRESH_TOKEN: &str = "1//06wRa6yfCWxlVCgYIARAAGAYSNwF-L9IrqJoWV9OyEiPQ04oB-Npd4XiBFbQb2d4S4orFLTHXuP-bzoPtu9wNF1EC92goGvtt8W8";
const REFRESH_GRANT_TYPE: &str = "refresh_token";

const PROJECT_ID: &str = "a1cd7915-4b7a-4b7c-99f7-7b5162af3934";
const DEVICE_ID: &str =
    "AVPHwEv41qUeoJJ_JXB1UNgpamQreNbEtiQAFHzJKhhZPvxJTofYF3XLMVG8_kmhYbBrkqeFrT0yJYyTsnNzct0-7t8_";
const DEVICE_HOST: &str = "https://smartdevicemanagement.googleapis.com/v1";

const CLIENT_ID: &str = "";
const CLIENT_SECRET: &str = "";

const MQTT_HOST: &str = "localhost";
const MQTT_PORT: u16 = 1883;
const MQTT_USERNAME: &str = "nest-accessor";

// get secrets from outside git projects

#[derive(Deserialize, Debug)]
struct refresh_response_data {
    access_token: String,
    expires_in: u32,
    scope: String,
    token_type: String,
}

async fn refresh_token(client_id: &str, client_data: &str) -> Result<String, Error> {
    let client = Client::new();

    // Create client POST request and add data
    let response = client
        .post(REFRESH_HOST)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", REFRESH_TOKEN),
            ("grant_type", REFRESH_GRANT_TYPE),
        ])
        .send()
        .await?;

    // Get access token from response
    let response_json: refresh_response_data = response.json().await?;
    let access_token = response_json.access_token;
    Ok(access_token)
}

async fn get_thermostat_data(
    client_id: &str,
    client_data: &str,
) -> Result<(f64, f64, String), Error> {
    let client = Client::new();

    // Get access token
    let access_token = refresh_token(client_id, client_data).await?;

    // Create client GET request and add data
    let response = client
        .get(format![
            "{device_host}/enterprises/{projectID}/devices/{deviceID}",
            device_host = DEVICE_HOST,
            projectID = PROJECT_ID,
            deviceID = DEVICE_ID
        ])
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    // Get thermostat data from response
    let response_json: serde_json::Value = response.json().await?;
    let traits = response_json["traits"].clone();
    let temperature_C = match &traits["sdm.devices.traits.Temperature"]["ambientTemperatureCelsius"]
    {
        serde_json::Value::Number(n) => n.as_f64().unwrap(),
        _ => panic!("Unexpected temperature value"),
    };

    let humidity = match &traits["sdm.devices.traits.Humidity"]["ambientHumidityPercent"] {
        serde_json::Value::Number(n) => n.as_f64().unwrap(),
        _ => panic!("Unexpected humidity value"),
    };

    let hvac_status = match &traits["sdm.devices.traits.ThermostatHvac"]["status"] {
        serde_json::Value::String(s) => s.clone(),
        _ => panic!("Unexpected hvac status value"),
    };

    Ok((temperature_C, humidity, hvac_status))
}

async fn mqtt_client() -> Result<AsyncClient, Error> {
    // Create mqtt client
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri("tcp://localhost:1883")
        .client_id("nest-accessor")
        .finalize();

    let cli = mqtt::Client::new(create_opts)?;
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(std::time::Duration::from_secs(20))
        .clean_session(true)
        .finalize();

    cli.connect(conn_opts).await?;
    Ok(cli)
}

async fn push_to_mqtt(
    cli: AsyncClient,
    temperature_C: f64,
    humidity: f64,
    hvac_status: String,
) -> Result<(), Error> {
    // Publish data to mqtt broker
    let msg = mqtt::Message::new(
        "homie/nest/thermostat/temperature",
        temperature_C.to_string(),
        0,
    );
    cli.publish(msg).await?;

    let msg = mqtt::Message::new("homie/nest/thermostat/humidity", humidity.to_string(), 0);
    cli.publish(msg).await?;

    let msg = mqtt::Message::new("homie/nest/thermostat/hvac_status", hvac_status, 0);
    cli.publish(msg).await?;

    Ok(())
}

async fn get_secrets(fname: &str) -> Result<(String, String), Error> {
    let contents = fs::read_to_string(fname)?;
    let mut lines = contents.lines();
    let client_id = lines.next().unwrap().to_string();
    let client_secret = lines.next().unwrap().to_string();
    Ok((client_id, client_secret))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get secrets from file
    let (client_id, client_secret) = get_secrets("secrets.txt").await?;
    // Create mqtt client
    let cli = mqtt_client().await?;
    // Every 5 seconds, collect thermostat data, then push that data to the homie mqtt broker
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    loop {
        interval.tick().await;
        let (temperature_C, humidity, hvac_status) =
            get_thermostat_data(&client_id, &client_secret).await?;
        println!(
            "Temperature: {} C, Humidity: {}%, HVAC Status: {}",
            temperature_C, humidity, hvac_status
        );
        push_to_mqtt(cli, temperature_C, humidity, hvac_status).await?;
    }

    Ok(())
}
