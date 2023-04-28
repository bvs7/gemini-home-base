
use futures::future::Future;
use paho_mqtt as mqtt;

use mysql;
use mysql::prelude::*;

use std::{
    thread,
    time::Duration
};

const MQTT_BROKER: &str = "mqtt://localhost:1883";
const CLIENT_ID:&str = "base-logger";
const TOPIC : &str = "/homie/#";

const QOS : i32 = 0;

fn get_create_options() -> mqtt::CreateOptions{
    mqtt::CreateOptionsBuilder::new()
        .server_uri(MQTT_BROKER)
        .client_id(CLIENT_ID)
        .finalize()
}

fn on_connect(cli: &mqtt::AsyncClient){
    println!("Connected");
}

fn on_connection_lost(cli: &mqtt::AsyncClient){
    println!("Connection Lost!");
}

fn on_disconnect(cli: &mqtt::AsyncClient, props: mqtt::Properties, reason: mqtt::ReasonCode){
    println!("Disconnected: {:?}, {:?}", props, reason);
}

fn on_message(cli: &mqtt::AsyncClient, msg: Option<mqtt::Message>){
    if let Some(msg) = msg {
        println!("Received Message: {}", msg);

        let topic = msg.topic();
        let loc_name = topic.split_once("/").unwrap().1.split_once("/").unwrap().0;

        let loc_id = find_name(loc_name);

        let temp = msg.payload_str().parse::<f32>().expect("Payload not float!");

        submit_temperature(loc_id, temp, None);
        println!("Temp submitted");
    }else{
        println!("Received None Message");
    }
}

fn open_connection(cli: &mqtt::AsyncClient) -> Result<(), mqtt::errors::Error>{
    let tok = cli.connect(None);
    tok.wait()?;

    Ok(())
}

fn close_connection(cli: &mqtt::AsyncClient) -> Result<(), mqtt::errors::Error>{
    let tok = cli.disconnect(None);
    tok.wait()?;

    Ok(())
}

fn set_callbacks(cli: &mqtt::AsyncClient) -> Result<(), mqtt::errors::Error>{
    cli.set_connected_callback(on_connect);
    cli.set_connection_lost_callback(on_connection_lost);
    cli.set_disconnected_callback(on_disconnect);

    cli.set_message_callback(on_message);

    Ok(())
}

fn find_name(loc_name: &str) -> i32 {
    let url = "mysql://gemini@localhost/climate_control";

    let mut conn = mysql::Conn::new(url).expect("Conn failed");

    if let Some(loc_id) = conn.query(format!("SELECT locID FROM location_names WHERE name = '{}'", loc_name.to_string())).expect("Select failed").pop(){
        return loc_id;
    }else{
        conn.exec_drop("INSERT INTO location_names (name) VALUES (?)",(loc_name,)).expect("Failed to insert name");
    }
    return conn.query(format!("SELECT locID FROM location_names WHERE name = '{}'", loc_name.to_string())).expect("Select failed").pop().expect("Name failed to add!");
}

// Connect to mysql server and push data
fn submit_temperature(loc_id: i32, temp: f32, hum: Option<f32>) -> Result<(), mysql::Error>{
    let url = "mysql://gemini@localhost/climate_control";

    let mut conn = mysql::Conn::new(url)?;

    println!("Submitting temperature data: {} {} {:?}", loc_id, temp, hum);

    conn.exec_drop("INSERT INTO temperature_data VALUES (?, NOW(), ?, ?)", (loc_id, temp, hum))?;
    Ok(())
}

fn main() -> Result<(),mqtt::errors::Error> {
    let cli = mqtt::AsyncClient::new(get_create_options())?;

    set_callbacks(&cli)?;

    open_connection(&cli)?;

    cli.subscribe("homie/+/temp_sensor/temperature", QOS);

    thread::sleep(Duration::from_secs(5));

    close_connection(&cli)?;

    Ok(())

}