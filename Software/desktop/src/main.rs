use serialport::{SerialPortType, UsbPortInfo, SerialPort};

fn main() {
    let ports = serialport::available_ports().expect("Failed to enumerate serial ports");
    let mut device = None;
    for port in ports {
        let info = if let SerialPortType::UsbPort(info) = &port.port_type {
            if info.vid == 0x16c0 && info.pid == 0x27dd {
                info.clone()
            } else {
                continue;
            }
        } else {
            continue;
        };
        device = Some((port, info));
    }
    let (port, info) = device.expect("Failed to find device. Is it connected?");
    println!("Found device {:?} {:?} {:?}", info.product, info.manufacturer, info.serial_number);
    println!("Opening {}", port.port_name);
    let port = serialport::new(&port.port_name, 115_200).open().expect("Failed to open serial port!");
    match handle_loop(port) {
        Ok(_) => println!("Serial port done"),
        Err(err) => println!("Serial closed with error {:?}", err),
    }
}

fn handle_loop(mut device: Box<dyn SerialPort>) -> Result<(), std::io::Error> {
    let mut data = [0u8; 1024]; 
    loop {
        let msg = "HeLlO";
        device.write_all(msg.as_bytes())?;
        let bytes = device.read(&mut data)?;
        println!("Got bytes {:?}", bytes);
    }
}
