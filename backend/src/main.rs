mod sensors;

fn main() {
    let devices = sensors::sensors::Devices::new();

    for device in devices.devices {
        match device.get_data(&devices.api_key) {
            // Print the data if it is available
            Some(data) => {
                println!("{}", data.device_name);
                println!("Time:        {}", data.timestamp.to_rfc2822());
                println!("Online:      {}", data.online);
                println!("Temperature: {:.1}°C", data.temperature);
                println!("Humidity:    {}%", data.humidity);
            }
            None => println!("Could not get data"),
        }

        println!();
    }
}
