fn main() {
    let mut port = serialport::new("/dev/ttyS0", 115200)
        .open()
        .expect("Unable to open");
    port.write_all("test".as_bytes()).unwrap();
}
